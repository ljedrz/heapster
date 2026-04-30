#[cfg(feature = "fmt")]
mod fmt;
mod stats;
pub use stats::Stats;

use std::{
    alloc::{GlobalAlloc, Layout},
    cmp,
    sync::atomic::{AtomicUsize, Ordering},
};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_SUM: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BUCKETS: [AtomicUsize; 64] = [const { AtomicUsize::new(0) }; 64];

static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_SUM: AtomicUsize = AtomicUsize::new(0);

static REALLOC_GROWTH_COUNT: AtomicUsize = AtomicUsize::new(0);
static REALLOC_GROWTH_SUM: AtomicUsize = AtomicUsize::new(0);
static REALLOC_GROWTH_BUCKETS: [AtomicUsize; 64] = [const { AtomicUsize::new(0) }; 64];

static REALLOC_SHRINK_COUNT: AtomicUsize = AtomicUsize::new(0);
static REALLOC_SHRINK_SUM: AtomicUsize = AtomicUsize::new(0);
static REALLOC_SHRINK_BUCKETS: [AtomicUsize; 64] = [const { AtomicUsize::new(0) }; 64];

static REALLOC_MOVE_COUNT: AtomicUsize = AtomicUsize::new(0);
static REALLOC_MOVE_SUM: AtomicUsize = AtomicUsize::new(0);

static USE_CURR: AtomicUsize = AtomicUsize::new(0);
static USE_MAX: AtomicUsize = AtomicUsize::new(0);

/// A global allocator enhanced with stats.
#[derive(Debug, Default, Clone, Copy)]
pub struct Heapster<A: GlobalAlloc>(A);

fn bucket_snapshot(buckets: &[AtomicUsize; 64]) -> [usize; 64] {
    let mut out = [0usize; 64];
    for (i, b) in buckets.iter().enumerate() {
        out[i] = b.load(Ordering::Relaxed);
    }
    out
}

impl<A: GlobalAlloc> Heapster<A> {
    /// Wraps an allocator, facilitating useful stats.
    pub const fn new(alloc: A) -> Self {
        Self(alloc)
    }

    /// Returns the total number of allocations.
    pub fn alloc_count(&self) -> usize {
        ALLOC_COUNT.load(Ordering::Relaxed)
    }

    /// Returns the sum of all allocations.
    pub fn alloc_sum(&self) -> usize {
        ALLOC_SUM.load(Ordering::Relaxed)
    }

    /// Returns buckets containing the numbers of allocations of
    /// different sizes, starting with 2^0 and ending with 2^63.
    pub fn alloc_buckets(&self) -> [usize; 64] {
        bucket_snapshot(&ALLOC_BUCKETS)
    }

    /// Returns the total number of deallocations.
    pub fn dealloc_count(&self) -> usize {
        DEALLOC_COUNT.load(Ordering::Relaxed)
    }

    /// Returns the sum of all deallocations.
    pub fn dealloc_sum(&self) -> usize {
        DEALLOC_SUM.load(Ordering::Relaxed)
    }

    /// Returns the total number of reallocations caused by object growth.
    pub fn realloc_growth_count(&self) -> usize {
        REALLOC_GROWTH_COUNT.load(Ordering::Relaxed)
    }

    /// Returns the sum of all reallocations caused by object growth.
    pub fn realloc_growth_sum(&self) -> usize {
        REALLOC_GROWTH_SUM.load(Ordering::Relaxed)
    }

    /// Returns buckets containing the numbers of growth reallocations
    /// of different sizes, starting with 2^0 and ending with 2^63.
    pub fn realloc_growth_buckets(&self) -> [usize; 64] {
        bucket_snapshot(&REALLOC_GROWTH_BUCKETS)
    }

    /// Returns the total number of reallocations caused by object shrinkage.
    pub fn realloc_shrink_count(&self) -> usize {
        REALLOC_SHRINK_COUNT.load(Ordering::Relaxed)
    }

    /// Returns the sum of all reallocations caused by object shrinkage.
    pub fn realloc_shrink_sum(&self) -> usize {
        REALLOC_SHRINK_SUM.load(Ordering::Relaxed)
    }

    /// Returns buckets containing the numbers of shrink reallocations
    /// of different sizes, starting with 2^0 and ending with 2^63.
    pub fn realloc_shrink_buckets(&self) -> [usize; 64] {
        bucket_snapshot(&REALLOC_SHRINK_BUCKETS)
    }

    /// Returns the total number of full reallocations.
    pub fn realloc_move_count(&self) -> usize {
        REALLOC_MOVE_COUNT.load(Ordering::Relaxed)
    }

    /// Returns the sum of all full reallocations.
    pub fn realloc_move_sum(&self) -> usize {
        REALLOC_MOVE_SUM.load(Ordering::Relaxed)
    }

    /// Returns the average size of allocations.
    pub fn alloc_avg(&self) -> Option<usize> {
        let sum = ALLOC_SUM.load(Ordering::Relaxed);
        let count = ALLOC_COUNT.load(Ordering::Relaxed);
        sum.checked_div(count)
    }

    /// Returns the average size of deallocations.
    pub fn dealloc_avg(&self) -> Option<usize> {
        let sum = DEALLOC_SUM.load(Ordering::Relaxed);
        let count = DEALLOC_COUNT.load(Ordering::Relaxed);
        sum.checked_div(count)
    }

    /// Returns the average size of reallocations caused by object growth.
    pub fn realloc_growth_avg(&self) -> Option<usize> {
        let sum = REALLOC_GROWTH_SUM.load(Ordering::Relaxed);
        let count = REALLOC_GROWTH_COUNT.load(Ordering::Relaxed);
        sum.checked_div(count)
    }

    /// Returns the average size of reallocations caused by object shrinkage.
    pub fn realloc_shrink_avg(&self) -> Option<usize> {
        let sum = REALLOC_SHRINK_SUM.load(Ordering::Relaxed);
        let count = REALLOC_SHRINK_COUNT.load(Ordering::Relaxed);
        sum.checked_div(count)
    }

    /// Returns the average size of full reallocations.
    pub fn realloc_move_avg(&self) -> Option<usize> {
        let sum = REALLOC_MOVE_SUM.load(Ordering::Relaxed);
        let count = REALLOC_MOVE_COUNT.load(Ordering::Relaxed);
        sum.checked_div(count)
    }

    /// Returns current heap use.
    pub fn use_curr(&self) -> usize {
        USE_CURR.load(Ordering::Relaxed)
    }

    /// Returns maximum recorded heap use.
    pub fn use_max(&self) -> usize {
        USE_MAX.load(Ordering::Relaxed)
    }

    /// Sets the stats to 0, except for current heap use (which is unaffected)
    /// and maximum heap use, which is reset to the value of current heap use.
    pub fn reset(&self) {
        ALLOC_SUM.store(0, Ordering::Relaxed);
        ALLOC_COUNT.store(0, Ordering::Relaxed);
        for b in &ALLOC_BUCKETS {
            b.store(0, Ordering::Relaxed);
        }

        DEALLOC_SUM.store(0, Ordering::Relaxed);
        DEALLOC_COUNT.store(0, Ordering::Relaxed);

        REALLOC_GROWTH_COUNT.store(0, Ordering::Relaxed);
        REALLOC_GROWTH_SUM.store(0, Ordering::Relaxed);
        for b in &REALLOC_GROWTH_BUCKETS {
            b.store(0, Ordering::Relaxed);
        }

        REALLOC_SHRINK_COUNT.store(0, Ordering::Relaxed);
        REALLOC_SHRINK_SUM.store(0, Ordering::Relaxed);
        for b in &REALLOC_SHRINK_BUCKETS {
            b.store(0, Ordering::Relaxed);
        }

        REALLOC_MOVE_COUNT.store(0, Ordering::Relaxed);
        REALLOC_MOVE_SUM.store(0, Ordering::Relaxed);

        USE_MAX.store(self.use_curr(), Ordering::Relaxed);
    }
}

#[inline]
fn bucket_of(size: usize) -> usize {
    debug_assert!(size > 0);
    (usize::BITS - 1 - size.leading_zeros()) as usize
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for Heapster<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = unsafe { self.0.alloc(layout) };
        if !ret.is_null() {
            let size = layout.size();
            ALLOC_SUM.fetch_add(size, Ordering::Relaxed);
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
            let curr = USE_CURR.fetch_add(size, Ordering::Relaxed) + size;
            USE_MAX.fetch_max(curr, Ordering::Relaxed);
            ALLOC_BUCKETS[bucket_of(size)].fetch_add(1, Ordering::Relaxed);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(ptr, layout) };
        let size = layout.size();
        USE_CURR.fetch_sub(size, Ordering::Relaxed);
        DEALLOC_SUM.fetch_add(size, Ordering::Relaxed);
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { self.0.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            if new_size >= layout.size() {
                let diff = new_size - layout.size();
                REALLOC_GROWTH_COUNT.fetch_add(1, Ordering::Relaxed);
                REALLOC_GROWTH_SUM.fetch_add(diff, Ordering::Relaxed);
                let curr = USE_CURR.fetch_add(diff, Ordering::Relaxed) + diff;
                USE_MAX.fetch_max(curr, Ordering::Relaxed);
                REALLOC_GROWTH_BUCKETS[bucket_of(diff)].fetch_add(1, Ordering::Relaxed);
            } else {
                let diff = layout.size() - new_size;
                REALLOC_SHRINK_COUNT.fetch_add(1, Ordering::Relaxed);
                REALLOC_SHRINK_SUM.fetch_add(diff, Ordering::Relaxed);
                USE_CURR.fetch_sub(diff, Ordering::Relaxed);
                REALLOC_SHRINK_BUCKETS[bucket_of(diff)].fetch_add(1, Ordering::Relaxed);
            }
            if new_ptr != ptr {
                REALLOC_MOVE_COUNT.fetch_add(1, Ordering::Relaxed);
                REALLOC_MOVE_SUM.fetch_add(cmp::min(layout.size(), new_size), Ordering::Relaxed);
            }
        }
        new_ptr
    }
}
