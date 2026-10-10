//! A counting allocator for the crate's unit tests: the system
//! allocator, plus a per-thread count of allocations, so a test can
//! assert that a hot path allocates nothing (`allocations_in`). Tests
//! run on their own threads, so one test's count is not another's.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    /// The largest single request (bytes) since `largest_allocation_in`
    /// reset it.
    static LARGEST: Cell<usize> = const { Cell::new(0) };
}

fn note(size: usize) {
    // `try_with`: no count while the thread's locals are torn down.
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
    let _ = LARGEST.try_with(|l| l.set(l.get().max(size)));
}

// SAFETY: every call forwards to `System` unchanged; counting touches
// only a const-initialized thread local, which never allocates.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: as `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note(new_size);
        // SAFETY: as `alloc`; `ptr` came from this allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator with `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// How many allocations `f` made on this thread.
pub(crate) fn allocations_in(f: impl FnOnce()) -> u64 {
    let before = ALLOCATIONS.with(Cell::get);
    f();
    ALLOCATIONS.with(Cell::get) - before
}

/// The largest single allocation (bytes) `f` asked for on this thread: a
/// bound on what one hostile value can make a path allocate.
pub(crate) fn largest_allocation_in(f: impl FnOnce()) -> usize {
    let outer = LARGEST.with(|l| l.replace(0));
    f();
    let largest = LARGEST.with(Cell::get);
    LARGEST.with(|l| l.set(outer.max(largest)));
    largest
}
