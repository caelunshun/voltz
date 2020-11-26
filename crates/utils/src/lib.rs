//! Assorted utility data structures. Not necessarily specific to Voltz.

#![feature(allocator_api, slice_fill)]

mod bitset;
mod packed_array;

pub use bitset::BitSet;
pub use packed_array::PackedArray;
