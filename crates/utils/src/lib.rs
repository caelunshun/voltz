//! Assorted utility data structures. Not necessarily specific to Voltz.

#![feature(allocator_api, slice_fill)]

mod bitset;
mod packed_array;

use std::cell::RefCell;

pub use bitset::BitSet;
use bumpalo::Bump;
pub use packed_array::PackedArray;

thread_local! {
    /// A thread-local bump allocator.
    pub static THREAD_BUMP: RefCell<Bump> = RefCell::new(Bump::new());
}
