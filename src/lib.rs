#[cfg(feature = "fmt")]
use std::fmt;
use std::{
    alloc::{GlobalAlloc, Layout},
    cmp,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(feature = "fmt")]
use humansize::{BINARY, format_size};
#[cfg(feature = "fmt")]
use num_format::{Locale, ToFormattedString};

/// A global allocator enhanced with stats.
#[derive(Debug, Default, Clone, Copy)]
pub struct Heapster<A: GlobalAlloc>(A);

/// A summary of an allocator's stats.
#[derive(Debug, Clone)]
pub struct Stats {
    /// The total number of allocations.
    pub alloc_count: usize,
    /// The average size of allocations.
    pub alloc_avg: Option<usize>,
    /// The allocation size buckets.
    pub alloc_buckets: [usize; 64],
    /// The total number of deallocations.
    pub dealloc_count: usize,
    /// The average size of deallocations.
    pub dealloc_avg: Option<usize>,
    /// The total number of reallocations caused by object growth.
    pub realloc_growth_count: usize,
    /// The average size of reallocations caused by object growth.
    pub realloc_growth_avg: Option<usize>,
    /// The growth reallocation size buckets.
    pub realloc_growth_buckets: [usize; 64],
    /// The total number of reallocations caused by object shrinkage.
    pub realloc_shrink_count: usize,
    /// The average size of reallocations caused by object shrinkage.
    pub realloc_shrink_avg: Option<usize>,
    /// The shrink reallocation size buckets.
    pub realloc_shrink_buckets: [usize; 64],
    /// The total number of full reallocations.
    pub realloc_move_count: usize,
    /// The average size of full reallocations.
    pub realloc_move_avg: Option<usize>,
    /// Current heap use.
    pub use_curr: usize,
    /// Maximum recorded heap use.
    pub use_max: usize,
}

#[cfg(feature = "fmt")]
impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "alloc_count: {}",
            self.alloc_count.to_formatted_string(&Locale::en)
        )?;
        if let Some(alloc_avg) = self.alloc_avg {
            writeln!(f, "alloc_avg: {}", format_size(alloc_avg, BINARY))?;
        }
        writeln!(
            f,
            "\ndealloc_count: {}",
            self.dealloc_count.to_formatted_string(&Locale::en)
        )?;
        if let Some(dealloc_avg) = self.dealloc_avg {
            writeln!(f, "dealloc_avg: {}", format_size(dealloc_avg, BINARY))?;
        }
        writeln!(
            f,
            "\nrealloc_growth_count: {}",
            self.realloc_growth_count.to_formatted_string(&Locale::en)
        )?;
        if let Some(realloc_growth_avg) = self.realloc_growth_avg {
            writeln!(
                f,
                "realloc_growth_avg: {}",
                format_size(realloc_growth_avg, BINARY)
            )?;
        }
        writeln!(
            f,
            "\nrealloc_shrink_count: {}",
            self.realloc_shrink_count.to_formatted_string(&Locale::en)
        )?;
        if let Some(realloc_shrink_avg) = self.realloc_shrink_avg {
            writeln!(
                f,
                "realloc_shrink_avg: {}",
                format_size(realloc_shrink_avg, BINARY)
            )?;
        }
        writeln!(
            f,
            "\nrealloc_move_count: {}",
            self.realloc_move_count.to_formatted_string(&Locale::en)
        )?;
        if let Some(realloc_move_avg) = self.realloc_move_avg {
            writeln!(
                f,
                "realloc_move_avg: {}",
                format_size(realloc_move_avg, BINARY)
            )?;
        }
        writeln!(f, "\nuse_curr: {}", format_size(self.use_curr, BINARY))?;
        writeln!(f, "use_max: {}", format_size(self.use_max, BINARY))?;

        fmt_histogram(f, "\nalloc_buckets", &self.alloc_buckets)?;
        fmt_histogram(f, "\nrealloc_growth_buckets", &self.realloc_growth_buckets)?;
        fmt_histogram(f, "\nrealloc_shrink_buckets", &self.realloc_shrink_buckets)?;

        Ok(())
    }
}

#[cfg(feature = "fmt")]
fn fmt_histogram(f: &mut fmt::Formatter<'_>, name: &str, buckets: &[usize; 64]) -> fmt::Result {
    const BAR_WIDTH: usize = 40;

    let Some(first) = buckets.iter().position(|&c| c > 0) else {
        writeln!(f, "{name}: (empty)")?;
        return Ok(());
    };
    let last = buckets.iter().rposition(|&c| c > 0).unwrap();
    let max = *buckets[first..=last].iter().max().unwrap();

    writeln!(f, "{name}:")?;
    for (k, &count) in (first..=last).zip(buckets[first..=last].iter()) {
        let lo = 1usize << k;
        // bucket k covers [2^k, 2^(k+1) - 1]; the top bucket has no finite upper bound
        let lo_str = format_size(lo, BINARY);
        let range_str = match lo.checked_shl(1) {
            Some(hi) => format!("[{:>9} .. {:>9})", lo_str, format_size(hi, BINARY)),
            None => format!("[{:>9} ..       inf)", lo_str),
        };
        write!(
            f,
            "{}: {:>12}  ",
            range_str,
            count.to_formatted_string(&Locale::en),
        )?;
        // scale bar to max in the trimmed range; show a thin bar for any non-zero
        // count so tiny buckets don't disappear entirely
        let bar_len = if count == 0 {
            0
        } else {
            cmp::max(1, count.saturating_mul(BAR_WIDTH) / max)
        };
        for _ in 0..bar_len {
            f.write_str("█")?;
        }
        writeln!(f)?;
    }
    Ok(())
}

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

#[inline]
fn bucket_of(size: usize) -> usize {
    debug_assert!(size > 0);
    (usize::BITS - 1 - size.leading_zeros()) as usize
}

impl<A: GlobalAlloc> Heapster<A> {
    /// Wraps an allocator, facilitating useful stats..
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

    /// Returns the total number of reallocations caused by object shrinkage.
    pub fn realloc_shrink_count(&self) -> usize {
        REALLOC_SHRINK_COUNT.load(Ordering::Relaxed)
    }

    /// Returns the sum of all reallocations caused by object shrinkage.
    pub fn realloc_shrink_sum(&self) -> usize {
        REALLOC_SHRINK_SUM.load(Ordering::Relaxed)
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

    /// Returns a summary of the allocator's stats.
    pub fn stats(&self) -> Stats {
        let bucket_snapshot = |buckets: &[AtomicUsize; 64]| -> [usize; 64] {
            let mut out = [0usize; 64];
            for (i, b) in buckets.iter().enumerate() {
                out[i] = b.load(Ordering::Relaxed);
            }
            out
        };

        let alloc_count = self.alloc_count();
        let alloc_sum = self.alloc_sum();
        let alloc_avg = alloc_sum.checked_div(alloc_count);
        let alloc_buckets = bucket_snapshot(&ALLOC_BUCKETS);

        let dealloc_count = self.dealloc_count();
        let dealloc_sum = self.dealloc_sum();
        let dealloc_avg = dealloc_sum.checked_div(dealloc_count);

        let realloc_growth_count = self.realloc_growth_count();
        let realloc_growth_sum = self.realloc_growth_sum();
        let realloc_growth_avg = realloc_growth_sum.checked_div(realloc_growth_count);
        let realloc_growth_buckets = bucket_snapshot(&REALLOC_GROWTH_BUCKETS);

        let realloc_shrink_count = self.realloc_shrink_count();
        let realloc_shrink_sum = self.realloc_shrink_sum();
        let realloc_shrink_avg = realloc_shrink_sum.checked_div(realloc_shrink_count);
        let realloc_shrink_buckets = bucket_snapshot(&REALLOC_SHRINK_BUCKETS);

        let realloc_move_count = self.realloc_move_count();
        let realloc_move_sum = self.realloc_move_sum();
        let realloc_move_avg = realloc_move_sum.checked_div(realloc_move_count);

        let use_curr = self.use_curr();
        let use_max = self.use_max();

        Stats {
            alloc_count,
            alloc_avg,
            alloc_buckets,
            dealloc_count,
            dealloc_avg,
            realloc_growth_count,
            realloc_growth_avg,
            realloc_growth_buckets,
            realloc_shrink_count,
            realloc_shrink_avg,
            realloc_shrink_buckets,
            realloc_move_count,
            realloc_move_avg,
            use_curr,
            use_max,
        }
    }
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
