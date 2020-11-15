//! Data structure for compactly storing blocks in the world.

use crate::{blocks, BlockId};

/// The dimensions of a chunk (cube).
pub const CHUNK_DIM: usize = 16;
/// The volume of a chunk in blocks.
pub const CHUNK_VOLUME: usize = CHUNK_DIM * CHUNK_DIM * CHUNK_DIM;

/// Position of a chunk relative to the zone origin.
/// Measured in units of CHUNK_DIM = 16 blocks.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// The starting number of bits per block to use in a chunk.
const INITIAL_BITS_PER_BLOCK: usize = 3;

/// Efficiently and compactly stores a 16x16x16 chunk of blocks.
///
/// Internally, a chunk contains a packed array of bits and a palette.
/// Each entry in the packed array is an index into the palette, which
/// is a `Vec<BlockId>`. For chunks with small numbers of blocks, we can
/// use as few as 3-4 bits per block.
pub struct Chunk {
    /// Stores indexes into `palette` of blocks for each position.
    indexes: PackedArray,
    /// The set of distinct block states in this chunk.
    ///
    /// This palette must remain stable unless `indexes` is updated
    /// in accordance.
    palette: Vec<BlockId>,
}

impl Chunk {
    /// Creates a new chunk initialized with air.
    pub fn new() -> Self {
        Self {
            indexes: PackedArray::new(CHUNK_VOLUME, INITIAL_BITS_PER_BLOCK),
            palette: vec![BlockId::new(blocks::Air)],
        }
    }

    /// Gets the block at the given position within this chunk.
    ///
    /// # Panics
    /// Panics if `x, y, or z >= CHUNK_DIM`.
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        Self::check_bounds(x, y, z);
        let index = self
            .indexes
            .get(Self::ordinal(x, y, z))
            .expect("bounds checked") as usize;

        self.palette[index]
    }

    /// Sets the block at the given position within this chunk.
    ///
    /// # Panics
    /// Panics if `x, y, or z >= CHUNK_DIM`.
    pub fn set(&mut self, x: usize, y: usize, z: usize, block: BlockId) {
        Self::check_bounds(x, y, z);
        let index = self.find_in_palette(block);
        self.indexes.set(Self::ordinal(x, y, z), index as u64);
    }

    fn find_in_palette(&mut self, block: BlockId) -> usize {
        match self.palette.iter().position(|b| *b == block) {
            Some(pos) => pos,
            None => {
                let pos = self.palette.len();
                self.grow_palette(block);
                pos
            }
        }
    }

    fn grow_palette(&mut self, block: BlockId) {
        self.palette.push(block);

        // If the new length of the palette exceeds the
        // max value in the `indexes` packed array, we need
        // to resize the indexes.
        if self.palette.len() - 1 > self.indexes.max_value() as usize {
            self.indexes = self.indexes.resized(self.indexes.bits_per_value() + 1);
        }
    }

    fn check_bounds(x: usize, y: usize, z: usize) {
        assert!(x < CHUNK_DIM, "x coordinate {} out of bounds", x);
        assert!(y < CHUNK_DIM, "y coordinate {} out of bounds", y);
        assert!(z < CHUNK_DIM, "z coordinate {} out of bounds", z);
    }

    fn ordinal(x: usize, y: usize, z: usize) -> usize {
        (y * CHUNK_DIM * CHUNK_DIM) + (z * CHUNK_DIM) + x
    }
}

/// A packed array of integers, where each integer consumes
/// `n` bits (where `n` is determined at runtime and not necessarily
/// a power of 2).
pub struct PackedArray {
    length: usize,
    bits_per_value: usize,
    bits: Vec<u64>,
}

impl PackedArray {
    /// Creates a new `PackedArray` with the given length
    /// and number of bits per value. Values are initialized
    /// to zero.
    ///
    /// # Panics
    /// Panics if `bits_per_value > 64`.
    pub fn new(length: usize, bits_per_value: usize) -> Self {
        let mut this = Self {
            length,
            bits_per_value,
            bits: Vec::new(),
        };
        let needed_u64s = this.needed_u64s();
        this.bits = vec![0u64; needed_u64s];

        this
    }

    /// Gets the value at the given index.
    pub fn get(&self, index: usize) -> Option<u64> {
        if index >= self.len() {
            return None;
        }

        let (u64_index, bit_index) = self.indexes(index);

        let u64 = self.bits[u64_index];
        Some((u64 >> bit_index) & self.mask())
    }

    /// Sets the value at the given index.
    ///
    /// # Panics
    /// Panics if `index >= self.length()` or `value > self.max_value()`.
    pub fn set(&mut self, index: usize, value: u64) {
        assert!(
            index < self.len(),
            "index out of bounds: index is {}; length is {}",
            index,
            self.len()
        );

        let mask = self.mask();
        assert!(value <= mask);

        let (u64_index, bit_index) = self.indexes(index);

        let u64 = &mut self.bits[u64_index];
        *u64 &= !(mask << bit_index);
        *u64 |= value << bit_index;
    }

    /// Resizes this packed array to a new bits per value.
    pub fn resized(&mut self, new_bits_per_value: usize) -> PackedArray {
        // Currently a naive algorithm - could be optimized if necessary.
        let mut array = PackedArray::new(self.len(), new_bits_per_value);

        for i in 0..self.len() {
            array.set(i, self.get(i).unwrap());
        }

        array
    }

    /// Returns the maximum value of an integer in this packed array.
    pub fn max_value(&self) -> u64 {
        self.mask()
    }

    /// Returns the length of this packed array.
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of bits used to represent each value.
    pub fn bits_per_value(&self) -> usize {
        self.bits_per_value
    }

    fn mask(&self) -> u64 {
        (1 << self.bits_per_value) - 1
    }

    fn needed_u64s(&self) -> usize {
        (self.length + self.values_per_u64() - 1) / self.values_per_u64()
    }

    fn values_per_u64(&self) -> usize {
        64 / self.bits_per_value
    }

    fn indexes(&self, index: usize) -> (usize, usize) {
        let u64_index = index / self.values_per_u64();
        let bit_index = (index % self.values_per_u64()) * self.bits_per_value;

        (u64_index, bit_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_array_smoke() {
        let length = 100;
        let mut array = PackedArray::new(length, 10);
        assert_eq!(array.len(), length);
        assert_eq!(array.bits_per_value(), 10);
        assert_eq!(array.bits.len(), 17);

        for i in 0..length {
            assert_eq!(array.get(i), Some(0));
            array.set(i, (i * 10) as u64);
            assert_eq!(array.get(i), Some((i * 10) as u64));
        }
    }

    #[test]
    fn packed_array_out_of_bounds() {
        let array = PackedArray::new(97, 10);
        assert_eq!(array.bits.len(), 17);
        assert_eq!(array.get(96), Some(0));
        assert_eq!(array.get(97), None);
    }

    #[test]
    fn chunk_smoke() {
        let mut chunk = Chunk::new();

        for x in 0..CHUNK_DIM {
            for y in 0..CHUNK_DIM {
                for z in 0..CHUNK_DIM {
                    assert!(chunk.get(x, y, z).is::<blocks::Air>());
                    chunk.set(x, y, z, BlockId::new(blocks::Dirt));
                    assert!(chunk.get(x, y, z).is::<blocks::Dirt>());
                }
            }
        }
    }
}
