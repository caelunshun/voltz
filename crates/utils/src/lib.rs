//! Assorted utility data structures. Not necessarily specific to Voltz.

#![feature(allocator_api, slice_fill)]

mod bitset;
pub mod bytecount;
mod geom;
mod packed_array;
mod track_alloc;

use bumpalo::Bump;
use std::cell::RefCell;

pub use bitset::BitSet;
pub use bytecount::format_bytes;
pub use geom::{Color, Rect};
pub use packed_array::PackedArray;
pub use track_alloc::TrackAllocator;

thread_local! {
    /// A thread-local bump allocator.
    pub static THREAD_BUMP: RefCell<Bump> = RefCell::new(Bump::new());
}
