use std::{
    alloc::GlobalAlloc,
    sync::atomic::{AtomicUsize, Ordering},
};

/// A global allocator which tracks the amount of allocated memory.
pub struct TrackAllocator<A> {
    wrapped: A,
    allocated: AtomicUsize,
}

impl<A> TrackAllocator<A> {
    pub const fn new(wrapped: A) -> Self {
        Self {
            wrapped,
            allocated: AtomicUsize::new(0),
        }
    }

    /// Returns the number of allocated bytes.
    pub fn allocated(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }
}

unsafe impl<A> GlobalAlloc for TrackAllocator<A>
where
    A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        self.allocated.fetch_add(layout.size(), Ordering::Relaxed);
        self.wrapped.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        self.allocated.fetch_sub(layout.size(), Ordering::Relaxed);
        self.wrapped.dealloc(ptr, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        self.allocated.fetch_add(layout.size(), Ordering::Relaxed);
        self.wrapped.alloc_zeroed(layout)
    }

    // TODO: figure out how we can track realloc?
}
