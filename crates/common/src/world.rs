//! Data structure for accessing blocks in the world.

use crate::{chunk::CHUNK_DIM, BlockId, Chunk, ChunkPos};
use ahash::AHashMap;
use uuid::Uuid;

/// Position of a block within a zone. Measured in blocks.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    /// Determines the chunk containing this block.
    pub fn chunk(self) -> ChunkPos {
        let x = self.x.div_euclid(CHUNK_DIM as i32);
        let y = self.y.div_euclid(CHUNK_DIM as i32);
        let z = self.z.div_euclid(CHUNK_DIM as i32);
        ChunkPos { x, y, z }
    }

    /// Determines the position of this BlockPos relative to its chunk.
    pub fn chunk_local(self) -> (usize, usize, usize) {
        (
            self.x.rem_euclid(CHUNK_DIM as i32) as usize,
            self.y.rem_euclid(CHUNK_DIM as i32) as usize,
            self.z.rem_euclid(CHUNK_DIM as i32) as usize,
        )
    }
}

/// A zone in the world.
///
/// Each zone is a set of chunks (16x16x16 blocks) that
/// has its own transform. Using multiple zones, we can
/// create voxel worlds with blocks that are not axis-aligned,
/// or with blocks that move throughout the world efficiently
/// (e.g. a ship).
///
/// A zone has a fixed, rectangular (box) size.
pub struct Zone {
    chunks: Vec<Chunk>,
    min: ChunkPos,
    max: ChunkPos,
}

#[derive(Debug, thiserror::Error)]
#[error("block {0:?} is outside of zone boundaries")]
pub struct BlockOutOfBounds(BlockPos);

impl Zone {
    /// Creates a new `ZoneBuilder`.
    pub fn builder(min: ChunkPos, max: ChunkPos) -> ZoneBuilder {
        ZoneBuilder::new(min, max)
    }

    /// Gets the chunk at `pos`.
    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        let index = self.chunk_index(pos)?;
        Some(&self.chunks[index])
    }

    /// Mutably gets the chunk at `pos`.
    pub fn chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        let index = self.chunk_index(pos)?;
        Some(&mut self.chunks[index])
    }

    /// Gets the block at `pos`, or `None` if `pos` is outside
    /// the bounds of this zone.
    pub fn block(&self, pos: BlockPos) -> Option<BlockId> {
        let chunk = self.chunk(pos.chunk())?;
        let (x, y, z) = pos.chunk_local();
        Some(chunk.get(x, y, z))
    }

    /// Sets the block at `pos`. Returns an error if `pos`
    /// is outside this zone.
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) -> Result<(), BlockOutOfBounds> {
        let chunk = self
            .chunk_mut(pos.chunk())
            .ok_or_else(|| BlockOutOfBounds(pos))?;
        let (x, y, z) = pos.chunk_local();
        chunk.set(x, y, z, block);
        Ok(())
    }

    /// Returns the number of chunks in the X direction.
    pub fn x_dim(&self) -> usize {
        (self.max.x - self.min.x + 1) as usize
    }

    /// Returns the number of chunks in the Y direction.
    pub fn y_dim(&self) -> usize {
        (self.max.y - self.min.y + 1) as usize
    }

    /// Returns the number of chunks in the Z direction.
    pub fn z_dim(&self) -> usize {
        (self.max.z - self.min.z + 1) as usize
    }

    pub fn min(&self) -> ChunkPos {
        self.min
    }

    pub fn max(&self) -> ChunkPos {
        self.max
    }

    fn chunk_index(&self, pos: ChunkPos) -> Option<usize> {
        if pos.x < self.min.x
            || pos.x > self.max.x
            || pos.y < self.min.y
            || pos.y > self.max.y
            || pos.z < self.min.z
            || pos.z > self.max.z
        {
            None
        } else {
            let xdiff = (pos.x - self.min.x) as usize;
            let ydiff = (pos.y - self.min.y) as usize;
            let zdiff = (pos.z - self.min.z) as usize;

            Some((xdiff * self.y_dim() * self.z_dim()) + (ydiff * self.z_dim()) + zdiff)
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("chunk {0:?} outside of zone bounds (min: {1:?}; max: {2:?})")]
pub struct ChunkOutOfBounds(ChunkPos, ChunkPos, ChunkPos);

/// Builder for a `Zone`.
pub struct ZoneBuilder {
    min: ChunkPos,
    max: ChunkPos,
    chunks: AHashMap<ChunkPos, Chunk>,
}

impl ZoneBuilder {
    pub fn new(min: ChunkPos, max: ChunkPos) -> Self {
        let mut min = min;
        let mut max = max;
        Self::make_min_max(&mut min, &mut max);

        Self {
            min,
            max,
            chunks: AHashMap::new(),
        }
    }

    fn make_min_max(min: &mut ChunkPos, max: &mut ChunkPos) {
        let minx = min.x.min(max.x);
        let miny = min.y.min(max.y);
        let minz = min.z.min(max.z);

        let maxx = min.x.max(max.x);
        let maxy = min.y.max(max.y);
        let maxz = min.z.max(max.z);

        *min = ChunkPos {
            x: minx,
            y: miny,
            z: minz,
        };
        *max = ChunkPos {
            x: maxx,
            y: maxy,
            z: maxz,
        };
    }

    /// Adds a chunk. Returns an error if the
    /// given chunk is not within the bounds of the zone
    /// being built.
    pub fn add_chunk(&mut self, pos: ChunkPos, chunk: Chunk) -> Result<(), ChunkOutOfBounds> {
        if pos.x < self.min.x
            || pos.x > self.max.x
            || pos.y < self.min.y
            || pos.y > self.max.y
            || pos.z < self.min.z
            || pos.z > self.max.z
        {
            return Err(ChunkOutOfBounds(pos, self.min, self.max));
        }

        self.chunks.insert(pos, chunk);
        debug_assert!(self.num_chunks() <= self.needed_chunks());
        Ok(())
    }

    /// Determines whether the zone is complete, i.e. whether
    /// all chunks within the bounds have been added via calls
    /// to `add_chunk()`. If this returns `true`, then calling
    /// `buil()` will return `Ok`.
    pub fn is_complete(&self) -> bool {
        self.num_chunks() == self.needed_chunks()
    }

    /// Returns the number of chunks needed for
    /// this zone to be complete.
    pub fn needed_chunks(&self) -> usize {
        let dimx = (self.max.x - self.min.x + 1) as usize;
        let dimy = (self.max.y - self.min.y + 1) as usize;
        let dimz = (self.max.z - self.min.z + 1) as usize;

        dimx * dimy * dimz
    }

    /// Returns the number of chunks currently added to this zone.
    pub fn num_chunks(&self) -> usize {
        self.chunks.len()
    }

    pub fn min(&self) -> ChunkPos {
        self.min
    }

    pub fn max(&self) -> ChunkPos {
        self.max
    }

    /// Builds this `ZoneBuilder` into a `Zone`.
    ///
    /// Returns an error if `!self.is_complete()`.
    pub fn build(mut self) -> Result<Zone, Self> {
        if !self.is_complete() {
            return Err(self);
        }

        let mut chunks = Vec::with_capacity(self.num_chunks());
        for x in self.min.x..=self.max.x {
            for y in self.min.y..=self.max.y {
                for z in self.min.z..=self.max.z {
                    chunks.push(
                        self.chunks
                            .remove(&ChunkPos { x, y, z })
                            .expect("missing chunk"),
                    );
                }
            }
        }

        Ok(Zone {
            min: self.min,
            max: self.max,
            chunks,
        })
    }
}

/// A "sparse" zone: a zone which contains a dynamic, potentially non-contiguous
/// set of chunks. Unlike `Zone`, the size of this zone is not fixed.
///
/// This structure is used by the client to represent the world as it knows
/// about a subset of the world. On the server side, we use [`Zone`] as
/// we know the whole world and it is slightly more efficient.
#[derive(Default)]
pub struct SparseZone {
    chunks: AHashMap<ChunkPos, Chunk>,
}

impl SparseZone {
    /// Creates a new zone containing no chunks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the chunk at `pos`.
    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    /// Mutably gets the chunk at `pos`.
    pub fn chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        self.chunks.get_mut(&pos)
    }

    /// Inserts a new chunk. If a chunk at `pos` already exists,
    /// it is replaced.
    pub fn insert(&mut self, pos: ChunkPos, chunk: Chunk) {
        self.chunks.insert(pos, chunk);
    }

    /// Removes the chunk at `pos`, returning it.
    pub fn remove(&mut self, pos: ChunkPos) -> Option<Chunk> {
        self.chunks.remove(&pos)
    }

    /// Gets the block at `pos`, or `None` if the block's
    /// chunk is not known.
    pub fn block(&self, pos: BlockPos) -> Option<BlockId> {
        let chunk = self.chunk(pos.chunk())?;
        let (x, y, z) = pos.chunk_local();
        Some(chunk.get(x, y, z))
    }

    /// Sets the block at `pos`. Returns an error if the
    /// block's chunk is not loaded.
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) -> Result<(), BlockOutOfBounds> {
        let chunk = self
            .chunk_mut(pos.chunk())
            .ok_or_else(|| BlockOutOfBounds(pos))?;
        let (x, y, z) = pos.chunk_local();
        chunk.set(x, y, z, block);
        Ok(())
    }
}

/// Unique, persistent ID of a `Zone`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ZoneId(Uuid);

/// A world, containing one or more `Zone`s` of the given
/// type. `Z` should be either [`Zone`] or [`SparseZone`].
///
/// # Main zone
/// The main zone is the zone containing "the ground,"
/// or most of the world. Other zones correspond
/// to e.g. ships.
pub struct World<Z> {
    zones: AHashMap<ZoneId, Z>,
    main_zone: ZoneId,
}

impl<Z> World<Z> {
    /// Creates a `World` with the given main zone.
    pub fn new(main_zone: Z) -> Self {
        let mut zones = AHashMap::new();
        let main_zone_id = Self::create_zone_id();
        zones.insert(main_zone_id, main_zone);

        Self {
            zones,
            main_zone: main_zone_id,
        }
    }

    /// Gets the `Zone` with the given ID.
    pub fn zone(&self, id: ZoneId) -> Option<&Z> {
        self.zones.get(&id)
    }

    /// Mutably gets the `Zone` with the given ID.
    pub fn zone_mut(&mut self, id: ZoneId) -> Option<&mut Z> {
        self.zones.get_mut(&id)
    }

    /// Gets the main zone.
    pub fn main_zone(&self) -> &Z {
        self.zone(self.main_zone).expect("missing main zone")
    }

    /// Gets the main zone.
    pub fn main_zone_mut(&mut self) -> &mut Z {
        self.zone_mut(self.main_zone).expect("missing main zone")
    }

    /// Inserts a new zone into this world.
    /// Returns the ID of this zone.
    pub fn add_zone(&mut self, zone: Z) -> ZoneId {
        let id = Self::create_zone_id();
        self.zones.insert(id, zone);
        id
    }

    /// Removes a zone from this world.
    /// Returns the removed zone.
    pub fn remove_zone(&mut self, id: ZoneId) -> Option<Z> {
        self.zones.remove(&id)
    }

    fn create_zone_id() -> ZoneId {
        ZoneId(Uuid::new_v4())
    }
}

#[cfg(test)]
mod tests {
    use crate::blocks;

    use super::*;

    #[test]
    fn block_to_chunk() {
        assert_eq!(
            BlockPos { x: 0, y: 0, z: 0 }.chunk(),
            ChunkPos { x: 0, y: 0, z: 0 },
        );
        assert_eq!(
            BlockPos {
                x: 15,
                y: 15,
                z: 15
            }
            .chunk(),
            ChunkPos { x: 0, y: 0, z: 0 },
        );
        assert_eq!(
            BlockPos { x: -1, y: 0, z: 0 }.chunk(),
            ChunkPos { x: -1, y: 0, z: 0 },
        );
        assert_eq!(
            BlockPos { x: 0, y: -1, z: 0 }.chunk(),
            ChunkPos { x: 0, y: -1, z: 0 },
        );
        assert_eq!(
            BlockPos { x: 0, y: 0, z: -17 }.chunk(),
            ChunkPos { x: 0, y: 0, z: -2 },
        );
    }

    #[test]
    fn block_chunk_local_pos() {
        assert_eq!(BlockPos { x: 0, y: 0, z: 0 }.chunk_local(), (0, 0, 0));
        assert_eq!(
            BlockPos {
                x: 15,
                y: 14,
                z: 13
            }
            .chunk_local(),
            (15, 14, 13)
        );
        assert_eq!(
            BlockPos {
                x: -1,
                y: -1,
                z: -1
            }
            .chunk_local(),
            (15, 15, 15)
        );
    }

    #[test]
    fn simple_zone() {
        let mut builder =
            Zone::builder(ChunkPos { x: 0, y: 0, z: 0 }, ChunkPos { x: 1, y: 0, z: 0 });
        builder
            .add_chunk(ChunkPos { x: 0, y: 0, z: 0 }, Chunk::new())
            .unwrap();
        builder
            .add_chunk(ChunkPos { x: 1, y: 0, z: 0 }, Chunk::new())
            .unwrap();
        assert!(builder
            .add_chunk(ChunkPos { x: 0, y: 1, z: 0 }, Chunk::new())
            .is_err());

        let mut zone = builder.build().ok().unwrap();

        for x in 0..32 {
            for y in 0..16 {
                for z in 0..16 {
                    let pos = BlockPos { x, y, z };
                    assert_eq!(zone.block(pos), Some(BlockId::new(blocks::Air)));
                    zone.set_block(pos, BlockId::new(blocks::Dirt)).unwrap();
                    assert_eq!(zone.block(pos), Some(BlockId::new(blocks::Dirt)));
                }
            }
        }
    }
}
