use arrayvec::ArrayVec;
use bitflags::bitflags;
use bumpalo::Bump;
use common::{
    blocks,
    chunk::{CHUNK_DIM, CHUNK_VOLUME},
    BlockId, Chunk,
};
use utils::BitSet;

/// Algorithm to skip rendering chunks which are occluded
/// by other chunks.
///
/// This works by determining, for each chunk, which
/// adjacent chunks can be seen through that chunk. Using
/// a breadth-first search, we then determine the set
/// of chunks visible from the player's chunk.
///
/// This struct contains the necessary state to offload
/// the culling computation to another thread.
pub struct Culler {}

bitflags! {
    /// A set of faces.
    #[derive(Default)]
    struct FaceBit: u8 {
        const BOTTOM = 0x01;
        const TOP = 0x02;
        const NEGX = 0x04;
        const POSX = 0x08;
        const NEGZ = 0x10;
        const POSZ = 0x20;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
enum Face {
    Bottom,
    Top,
    NegX,
    PosX,
    NegZ,
    PosZ,
}

impl Face {
    pub fn to_bit(self) -> FaceBit {
        match self {
            Face::Bottom => FaceBit::BOTTOM,
            Face::Top => FaceBit::TOP,
            Face::NegX => FaceBit::NEGX,
            Face::PosX => FaceBit::POSX,
            Face::NegZ => FaceBit::NEGZ,
            Face::PosZ => FaceBit::POSZ,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        static ITEMS: [Face; 6] = [
            Face::Bottom,
            Face::Top,
            Face::NegX,
            Face::PosX,
            Face::NegZ,
            Face::PosZ,
        ];
        ITEMS.iter().copied()
    }

    pub fn pos_index(self, pos: [usize; 3]) -> Option<usize> {
        match self {
            Face::Bottom => {
                if pos[1] == 0 {
                    Some(pos[0] * CHUNK_DIM + pos[2])
                } else {
                    None
                }
            }
            Face::Top => {
                if pos[1] == CHUNK_DIM - 1 {
                    Some(pos[0] * CHUNK_DIM + pos[2])
                } else {
                    None
                }
            }
            Face::NegX => {
                if pos[0] == 0 {
                    Some(pos[1] * CHUNK_DIM + pos[2])
                } else {
                    None
                }
            }
            Face::PosX => {
                if pos[0] == CHUNK_DIM - 1 {
                    Some(pos[1] * CHUNK_DIM + pos[2])
                } else {
                    None
                }
            }
            Face::NegZ => {
                if pos[2] == 0 {
                    Some(pos[0] * CHUNK_DIM + pos[1])
                } else {
                    None
                }
            }
            Face::PosZ => {
                if pos[2] == CHUNK_DIM - 1 {
                    Some(pos[0] * CHUNK_DIM + pos[1])
                } else {
                    None
                }
            }
        }
    }

    pub fn pos_from_index(self, index: usize) -> [usize; 3] {
        let a = index / CHUNK_DIM;
        let b = index % CHUNK_DIM;
        let end = CHUNK_DIM - 1;
        match self {
            Face::Bottom => [a, 0, b],
            Face::Top => [a, end, b],
            Face::NegX => [0, a, b],
            Face::PosX => [end, a, b],
            Face::NegZ => [a, b, 0],
            Face::PosZ => [a, b, end],
        }
    }

    pub fn start_pos(self) -> [usize; 3] {
        let end = CHUNK_DIM - 1;
        match self {
            Face::Bottom => [0, 0, 0],
            Face::Top => [0, end, 0],
            Face::NegX => [0, 0, 0],
            Face::PosX => [end, 0, 0],
            Face::NegZ => [0, 0, 0],
            Face::PosZ => [0, 0, end],
        }
    }

    /// Determines the set of up to three faces containing
    /// the given block.
    pub fn containing(pos: [usize; 3]) -> ArrayVec<[Face; 3]> {
        let mut result = ArrayVec::new();
        let end = CHUNK_DIM - 1;

        if pos[1] == 0 {
            result.push(Face::Bottom);
        } else if pos[1] == end {
            result.push(Face::Top);
        }

        if pos[0] == 0 {
            result.push(Face::NegX);
        } else if pos[0] == end {
            result.push(Face::PosX);
        }

        if pos[2] == 0 {
            result.push(Face::NegZ);
        } else if pos[2] == end {
            result.push(Face::PosZ);
        }

        result
    }
}

/// Stores which faces are visible from each face in a chunk.{}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
struct ChunkVisibility {
    faces: [FaceBit; 6],
}

impl ChunkVisibility {
    /// Gets faces visible from the given face.
    pub fn visible_faces(self, face: Face) -> FaceBit {
        self.faces[face as u8 as usize]
    }

    pub fn set_visible(&mut self, from: Face, to: Face) {
        self.faces[from as u8 as usize] |= to.to_bit();
        self.faces[to as u8 as usize] |= from.to_bit();
    }
}

/// Stores a set of remaining blocks to visit for a single
/// face of a chunk (so only 16x16=256 blocks).
struct RemainingSet<'bump> {
    remaining: BitSet<&'bump Bump>,
    face: Face,
}

impl<'bump> RemainingSet<'bump> {
    pub fn new(face: Face, bump: &'bump Bump) -> Self {
        let mut remaining = BitSet::new_in(CHUNK_DIM * CHUNK_DIM, bump);
        remaining.fill();
        Self { remaining, face }
    }

    /// Marks a block as visited if it lies on this face.
    pub fn mark_visited(&mut self, pos: [usize; 3]) {
        if let Some(index) = self.face.pos_index(pos) {
            self.remaining.remove(index);
        }
    }

    /// Gets the next unvisited block, or `None` if all
    /// blocks on this face are marked as visited.
    /// `start` must lie on this face.
    pub fn next_remaining(&self, start: [usize; 3]) -> Option<[usize; 3]> {
        let start = self.face.pos_index(start)?;
        let next = self.remaining.next(start)?;
        Some(self.face.pos_from_index(next))
    }
}

/// Computes a `ChunkVisibility` for the given chunk.
fn compute_visibility(chunk: &Chunk, bump: &Bump) -> ChunkVisibility {
    if chunk.is_empty() {
        // Fast path: all faces are visible from all other faces.
        return full_visibility();
    }

    let air_index = chunk
        .palette()
        .iter()
        .position(|&block| block == BlockId::new(blocks::Air));
    let air_index = match air_index {
        Some(a) => a,
        None => return ChunkVisibility::default(), // solid chunk
    };

    let mut result = ChunkVisibility::default();
    let mut remaining: ArrayVec<[RemainingSet; 6]> = Face::iter()
        .map(|face| RemainingSet::new(face, bump))
        .collect();

    let mut stack = Vec::new_in(bump);

    let mut all_visited = BitSet::new_in(CHUNK_VOLUME, bump);

    for face in Face::iter() {
        let mut pos = face.start_pos();
        while let Some(next_pos) = remaining[face as usize].next_remaining(pos) {
            remaining[face as usize].mark_visited(pos);
            pos = next_pos;

            // Perform a depth-first search beginning at this
            // block to detect connected faces.
            stack.clear();
            stack.push(pos);
            while let Some(dfs_pos) = stack.pop() {
                if chunk
                    .indexes()
                    .get(Chunk::ordinal(dfs_pos[0], dfs_pos[1], dfs_pos[2]))
                    != Some(air_index as u64)
                {
                    continue;
                }

                for connected_face in Face::containing(dfs_pos) {
                    result.set_visible(face, connected_face);
                    remaining[connected_face as usize].mark_visited(dfs_pos);
                }

                for adjacent in adjacent_positions(dfs_pos) {
                    if !all_visited.insert(Chunk::ordinal(adjacent[0], adjacent[1], adjacent[2])) {
                        stack.push(adjacent);
                    }
                }
            }
        }
    }

    result
}

fn full_visibility() -> ChunkVisibility {
    ChunkVisibility {
        faces: [FaceBit::all(); 6],
    }
}

fn adjacent_positions(pos: [usize; 3]) -> impl Iterator<Item = [usize; 3]> {
    let adjacent = [
        [pos[0], pos[1].wrapping_sub(1), pos[2]],
        [pos[0], pos[1] + 1, pos[2]],
        [pos[0].wrapping_sub(1), pos[1], pos[2]],
        [pos[0] + 1, pos[1], pos[2]],
        [pos[0], pos[1], pos[2].wrapping_sub(1)],
        [pos[0], pos[1], pos[2] + 1],
    ];
    ArrayVec::<[[usize; 3]; 6]>::from(adjacent)
        .into_iter()
        .filter(|pos| pos[0] < CHUNK_DIM && pos[1] < CHUNK_DIM && pos[2] < CHUNK_DIM)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn face_pos_index_roundtrip() {
        for face in Face::iter() {
            for index in 0..CHUNK_DIM * CHUNK_DIM {
                let pos = face.pos_from_index(index);
                assert_eq!(face.pos_index(pos), Some(index));
            }
        }
    }

    #[test]
    fn face_pos_index_outside_face() {
        let face = Face::Bottom;
        assert_eq!(face.pos_index([0, 1, 0]), None);
        assert_eq!(face.pos_index([15, 15, 15]), None);
    }

    #[test]
    fn visibility_empty_chunk() {
        let chunk = Chunk::new();
        let bump = Bump::new();
        let vis = compute_visibility(&chunk, &bump);
        assert_eq!(vis, full_visibility());
    }

    #[test]
    fn visibility_full_chunk() {
        let mut chunk = Chunk::new();
        chunk.fill(BlockId::new(blocks::Stone));

        let vis = compute_visibility(&chunk, &Bump::new());

        assert_eq!(vis, ChunkVisibility::default());
    }

    #[test]
    fn visibility_two_faces() {
        let mut chunk = Chunk::new();
        chunk.fill(BlockId::new(blocks::Stone));

        for x in 0..CHUNK_DIM {
            chunk.set(x, 8, 8, BlockId::new(blocks::Air));
        }

        for _ in 0..100 {
            let start = Instant::now();
            let vis = compute_visibility(&chunk, &Bump::new());
            println!("{:?}", start.elapsed());

            assert_eq!(vis.visible_faces(Face::NegX), FaceBit::POSX | FaceBit::NEGX);
            assert_eq!(vis.visible_faces(Face::PosX), FaceBit::NEGX);
            assert_eq!(vis.visible_faces(Face::Bottom), FaceBit::empty());
            assert_eq!(vis.visible_faces(Face::Top), FaceBit::empty());
            assert_eq!(vis.visible_faces(Face::NegZ), FaceBit::empty());
            assert_eq!(vis.visible_faces(Face::PosZ), FaceBit::empty());
        }
    }
}
