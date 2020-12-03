//! Assorted utility data structures. Not necessarily specific to Voltz.

#![feature(allocator_api, slice_fill)]

mod bitset;
mod geom;
mod packed_array;

use bumpalo::Bump;
use std::cell::RefCell;

pub use bitset::BitSet;
pub use geom::{Color, Rect};
pub use packed_array::PackedArray;

thread_local! {
    /// A thread-local bump allocator.
    pub static THREAD_BUMP: RefCell<Bump> = RefCell::new(Bump::new());
}
