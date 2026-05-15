//! **Lightweight heap telemetry for Rust, built on relaxed atomics.**
//!
//! `heapster` is a lightweight, generic wrapper over any `GlobalAlloc`
//! that tracks allocations, deallocations, and reallocations using pure relaxed atomics.
//! it is designed to be always-on, allowing you to identify allocation patterns,
//! diff heap usage between code paths, and export raw allocator metrics to your telemetry
//! dashboards with minimal overhead.
//!
//! ## why heapster?
//!
//! Heap profilers like dhat or heaptrack capture rich per-allocation data but add significant
//! overhead and require dedicated viewers. Heapster occupies a lighter tier: aggregate
//! counters and histograms only, with overhead low enough to leave on in production.
//!
//! - **atomics-only**: no mutexes, no thread-locals, no external viewer files. just relaxed atomic counters.
//! - **`no_std` by default**: uses only `core` and `alloc` in the default build, with no third-party dependencies. The `fmt` and `serde` features add `std` requirements.
//! - **generic over any allocator**: wraps `System`, jemalloc, mimalloc, or any custom `GlobalAlloc`.
//! - **size histograms**: power-of-two buckets for allocations and reallocations make the size distribution visible at a glance.
//! - **realloc classification**: distinguishes between reallocations that grew in-place, shrank in-place, or forced a full memory copy.
//! - **snapshot diffing**: `measure()` returns a `Stats` delta for a closure, suitable for assertion-style tests and benchmark comparisons.
//!
//! ## quickstart
//!
//! wrap your global allocator of choice (e.g., `System`) in your `main.rs` or `lib.rs`:
//!
//! ```rust
//! use heapster::Heapster;
//! use std::alloc::System;
//!
//! #[global_allocator]
//! static GLOBAL: Heapster<System> = Heapster::new(System);
//!
//! fn main() {
//!     // ... do some heavy work ...
//!
//!     // see what has transpired in the heap
//!     let stats = GLOBAL.stats();
//!     println!("allocated: {} bytes", stats.alloc_sum);
//! }
//! ```
//!
//! ## measuring specific operations
//!
//! `heapster` lets you diff the heap stats of critical sections of code using snapshot math:
//!
//! ```rust
//! # use heapster::Heapster;
//! # use std::alloc::System;
//! # #[global_allocator]
//! # static GLOBAL: Heapster<System> = Heapster::new(System);
//! let (result, heap_diff) = GLOBAL.measure(|| {
//!     // ... operation to measure ...
//!     42
//! });
//!
//! assert!(heap_diff.alloc_count < 10, "regression: the operation allocated too many times!");
//! ```

#![no_std]
#![deny(missing_docs)]
#![deny(clippy::all)]

#[cfg(feature = "fmt")]
mod fmt;
mod histogram;
mod stats;

pub use histogram::Histogram;
pub use stats::Stats;

#[cfg(feature = "fmt")]
extern crate alloc;

use core::{
    alloc::{GlobalAlloc, Layout},
    cmp,
    ops::Deref,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

#[repr(align(64))]
struct CacheAligned<T>(T);

impl<T> Deref for CacheAligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct AllocCluster {
    count: AtomicUsize,
    sum: AtomicU64,
}

struct UseCluster {
    curr: AtomicUsize,
    max: AtomicUsize,
}

static ALLOC: CacheAligned<AllocCluster> = CacheAligned(AllocCluster {
    count: AtomicUsize::new(0),
    sum: AtomicU64::new(0),
});

static DEALLOC: CacheAligned<AllocCluster> = CacheAligned(AllocCluster {
    count: AtomicUsize::new(0),
    sum: AtomicU64::new(0),
});

static REALLOC_GROWTH: CacheAligned<AllocCluster> = CacheAligned(AllocCluster {
    count: AtomicUsize::new(0),
    sum: AtomicU64::new(0),
});

static REALLOC_SHRINK: CacheAligned<AllocCluster> = CacheAligned(AllocCluster {
    count: AtomicUsize::new(0),
    sum: AtomicU64::new(0),
});

static REALLOC_MOVE: CacheAligned<AllocCluster> = CacheAligned(AllocCluster {
    count: AtomicUsize::new(0),
    sum: AtomicU64::new(0),
});

static USE: CacheAligned<UseCluster> = CacheAligned(UseCluster {
    curr: AtomicUsize::new(0),
    max: AtomicUsize::new(0),
});

static ALLOC_BUCKETS: CacheAligned<[AtomicUsize; 64]> =
    CacheAligned([const { AtomicUsize::new(0) }; 64]);
static REALLOC_GROWTH_BUCKETS: CacheAligned<[AtomicUsize; 64]> =
    CacheAligned([const { AtomicUsize::new(0) }; 64]);
static REALLOC_SHRINK_BUCKETS: CacheAligned<[AtomicUsize; 64]> =
    CacheAligned([const { AtomicUsize::new(0) }; 64]);

static ALLOC_FAIL_COUNT: AtomicUsize = AtomicUsize::new(0);
static REALLOC_FAIL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// A global allocator enhanced with stats.
#[derive(Debug, Default, Clone, Copy)]
pub struct Heapster<A: GlobalAlloc>(A);

fn bucket_snapshot(buckets: &[AtomicUsize; 64]) -> Histogram {
    let mut out = [0usize; 64];
    for (i, b) in buckets.iter().enumerate() {
        out[i] = b.load(Ordering::Relaxed);
    }
    Histogram { buckets: out }
}

impl<A: GlobalAlloc> Heapster<A> {
    /// Wraps an allocator, facilitating useful stats.
    pub const fn new(alloc: A) -> Self {
        Self(alloc)
    }

    /// Returns a reference to the underlying allocator.
    pub const fn inner(&self) -> &A {
        &self.0
    }

    /// Returns the total number of allocations.
    #[inline]
    pub fn alloc_count(&self) -> usize {
        ALLOC.count.load(Ordering::Relaxed)
    }

    /// Returns the sum of all allocations.
    #[inline]
    pub fn alloc_sum(&self) -> u64 {
        ALLOC.sum.load(Ordering::Relaxed)
    }

    /// Returns a histogram representing the number
    /// of allocations of different sizes.
    #[inline]
    pub fn alloc_histogram(&self) -> Histogram {
        bucket_snapshot(&ALLOC_BUCKETS)
    }

    /// Returns the total number of failed allocations.
    #[inline]
    pub fn alloc_fail_count(&self) -> usize {
        ALLOC_FAIL_COUNT.load(Ordering::Relaxed)
    }

    /// Returns the total number of deallocations.
    #[inline]
    pub fn dealloc_count(&self) -> usize {
        DEALLOC.count.load(Ordering::Relaxed)
    }

    /// Returns the sum of all deallocations.
    #[inline]
    pub fn dealloc_sum(&self) -> u64 {
        DEALLOC.sum.load(Ordering::Relaxed)
    }

    /// Returns the total number of reallocations caused by object growth.
    #[inline]
    pub fn realloc_growth_count(&self) -> usize {
        REALLOC_GROWTH.count.load(Ordering::Relaxed)
    }

    /// Returns the sum of all reallocations caused by object growth.
    #[inline]
    pub fn realloc_growth_sum(&self) -> u64 {
        REALLOC_GROWTH.sum.load(Ordering::Relaxed)
    }

    /// Returns a histogram representing the number
    /// of growth reallocations of different sizes.
    pub fn realloc_growth_histogram(&self) -> Histogram {
        bucket_snapshot(&REALLOC_GROWTH_BUCKETS)
    }

    /// Returns the total number of reallocations caused by object shrinkage.
    #[inline]
    pub fn realloc_shrink_count(&self) -> usize {
        REALLOC_SHRINK.count.load(Ordering::Relaxed)
    }

    /// Returns the sum of all reallocations caused by object shrinkage.
    #[inline]
    pub fn realloc_shrink_sum(&self) -> u64 {
        REALLOC_SHRINK.sum.load(Ordering::Relaxed)
    }

    /// Returns a histogram representing the number
    /// of shrink reallocations of different sizes.
    #[inline]
    pub fn realloc_shrink_histogram(&self) -> Histogram {
        bucket_snapshot(&REALLOC_SHRINK_BUCKETS)
    }

    /// Returns the total number of full reallocations.
    #[inline]
    pub fn realloc_move_count(&self) -> usize {
        REALLOC_MOVE.count.load(Ordering::Relaxed)
    }

    /// Returns the sum of all full reallocations.
    #[inline]
    pub fn realloc_move_sum(&self) -> u64 {
        REALLOC_MOVE.sum.load(Ordering::Relaxed)
    }

    /// Returns the total number of failed reallocations.
    #[inline]
    pub fn realloc_fail_count(&self) -> usize {
        REALLOC_FAIL_COUNT.load(Ordering::Relaxed)
    }

    /// Returns the average size of allocations.
    pub fn alloc_avg(&self) -> Option<usize> {
        let sum = self.alloc_sum();
        let count = self.alloc_count();
        sum.checked_div(count as u64).map(|avg| avg as usize)
    }

    /// Returns the average size of deallocations.
    pub fn dealloc_avg(&self) -> Option<usize> {
        let sum = self.dealloc_sum();
        let count = self.dealloc_count();
        sum.checked_div(count as u64).map(|avg| avg as usize)
    }

    /// Returns the average size of reallocations caused by object growth.
    pub fn realloc_growth_avg(&self) -> Option<usize> {
        let sum = self.realloc_growth_sum();
        let count = self.realloc_growth_count();
        sum.checked_div(count as u64).map(|avg| avg as usize)
    }

    /// Returns the average size of reallocations caused by object shrinkage.
    pub fn realloc_shrink_avg(&self) -> Option<usize> {
        let sum = self.realloc_shrink_sum();
        let count = self.realloc_shrink_count();
        sum.checked_div(count as u64).map(|avg| avg as usize)
    }

    /// Returns the average size of full reallocations.
    pub fn realloc_move_avg(&self) -> Option<usize> {
        let sum = self.realloc_move_sum();
        let count = self.realloc_move_count();
        sum.checked_div(count as u64).map(|avg| avg as usize)
    }

    /// Returns current heap use.
    #[inline]
    pub fn use_curr(&self) -> usize {
        USE.curr.load(Ordering::Relaxed)
    }

    /// Returns maximum recorded heap use.
    #[inline]
    pub fn use_max(&self) -> usize {
        USE.max.load(Ordering::Relaxed)
    }

    /// Measures the heap stats for the given operation, returning its
    /// result alongside the [`Stats`] object.
    pub fn measure<R>(&self, f: impl FnOnce() -> R) -> (R, Stats) {
        let before = self.stats();
        let r = f();
        let after = self.stats();
        (r, &after - &before)
    }

    /// Sets the stats to 0, except for current heap use (which is unaffected)
    /// and maximum heap use, which is reset to the value of current heap use.
    ///
    /// **Concurrency note:** `reset` is not synchronized with allocator activity.
    /// If another thread is mid-allocation when `reset` runs, its increment may
    /// land on a freshly-zeroed counter, briefly producing skewed values (or, in
    /// rare cases, a transient apparent decrease in `use_curr`). For measuring a
    /// specific operation in a multi-threaded program, prefer [`Heapster::measure`],
    /// which uses snapshot diffing and avoids touching shared state.
    pub fn reset(&self) {
        ALLOC.sum.store(0, Ordering::Relaxed);
        ALLOC.count.store(0, Ordering::Relaxed);
        for b in &*ALLOC_BUCKETS {
            b.store(0, Ordering::Relaxed);
        }
        ALLOC_FAIL_COUNT.store(0, Ordering::Relaxed);

        DEALLOC.sum.store(0, Ordering::Relaxed);
        DEALLOC.count.store(0, Ordering::Relaxed);

        REALLOC_GROWTH.count.store(0, Ordering::Relaxed);
        REALLOC_GROWTH.sum.store(0, Ordering::Relaxed);
        for b in &*REALLOC_GROWTH_BUCKETS {
            b.store(0, Ordering::Relaxed);
        }

        REALLOC_SHRINK.count.store(0, Ordering::Relaxed);
        REALLOC_SHRINK.sum.store(0, Ordering::Relaxed);
        for b in &*REALLOC_SHRINK_BUCKETS {
            b.store(0, Ordering::Relaxed);
        }

        REALLOC_MOVE.count.store(0, Ordering::Relaxed);
        REALLOC_MOVE.sum.store(0, Ordering::Relaxed);
        REALLOC_FAIL_COUNT.store(0, Ordering::Relaxed);

        USE.max.store(self.use_curr(), Ordering::Relaxed);
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
            ALLOC.sum.fetch_add(size as u64, Ordering::Relaxed);
            ALLOC.count.fetch_add(1, Ordering::Relaxed);
            let curr = USE.curr.fetch_add(size, Ordering::Relaxed) + size;
            USE.max.fetch_max(curr, Ordering::Relaxed);
            ALLOC_BUCKETS[bucket_of(size)].fetch_add(1, Ordering::Relaxed);
        } else {
            ALLOC_FAIL_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(ptr, layout) };
        let size = layout.size();
        USE.curr.fetch_sub(size, Ordering::Relaxed);
        DEALLOC.sum.fetch_add(size as u64, Ordering::Relaxed);
        DEALLOC.count.fetch_add(1, Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { self.0.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            if new_size >= layout.size() {
                let diff = new_size - layout.size();
                REALLOC_GROWTH.count.fetch_add(1, Ordering::Relaxed);
                REALLOC_GROWTH.sum.fetch_add(diff as u64, Ordering::Relaxed);
                let curr = USE.curr.fetch_add(diff, Ordering::Relaxed) + diff;
                USE.max.fetch_max(curr, Ordering::Relaxed);
                REALLOC_GROWTH_BUCKETS[bucket_of(diff)].fetch_add(1, Ordering::Relaxed);
            } else {
                let diff = layout.size() - new_size;
                REALLOC_SHRINK.count.fetch_add(1, Ordering::Relaxed);
                REALLOC_SHRINK.sum.fetch_add(diff as u64, Ordering::Relaxed);
                USE.curr.fetch_sub(diff, Ordering::Relaxed);
                REALLOC_SHRINK_BUCKETS[bucket_of(diff)].fetch_add(1, Ordering::Relaxed);
            }
            if new_ptr != ptr {
                REALLOC_MOVE.count.fetch_add(1, Ordering::Relaxed);
                REALLOC_MOVE
                    .sum
                    .fetch_add(cmp::min(layout.size(), new_size) as u64, Ordering::Relaxed);
            }
        } else {
            REALLOC_FAIL_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        new_ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ret = unsafe { self.0.alloc_zeroed(layout) };
        if !ret.is_null() {
            let size = layout.size();
            ALLOC.sum.fetch_add(size as u64, Ordering::Relaxed);
            ALLOC.count.fetch_add(1, Ordering::Relaxed);
            let curr = USE.curr.fetch_add(size, Ordering::Relaxed) + size;
            USE.max.fetch_max(curr, Ordering::Relaxed);
            ALLOC_BUCKETS[bucket_of(size)].fetch_add(1, Ordering::Relaxed);
        } else {
            ALLOC_FAIL_COUNT.fetch_add(1, Ordering::Relaxed);
        }

        ret
    }
}
