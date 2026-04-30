use std::{alloc::GlobalAlloc, ops::Sub};

use crate::{Heapster, Histogram};

/// A summary of an allocator's stats.
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Stats {
    /// The total number of allocations.
    pub alloc_count: usize,
    /// The sum of all allocations.
    pub alloc_sum: usize,
    /// The average size of allocations.
    pub alloc_avg: Option<usize>,
    /// The allocation size buckets.
    pub alloc_histogram: Histogram,
    /// The total number of failed allocations.
    pub alloc_fail_count: usize,
    /// The total number of deallocations.
    pub dealloc_count: usize,
    /// The sum of all deallocations.
    pub dealloc_sum: usize,
    /// The average size of deallocations.
    pub dealloc_avg: Option<usize>,
    /// The total number of reallocations caused by object growth.
    pub realloc_growth_count: usize,
    /// The sum of all growth reallocations.
    pub realloc_growth_sum: usize,
    /// The average size of reallocations caused by object growth.
    pub realloc_growth_avg: Option<usize>,
    /// The growth reallocation size buckets.
    pub realloc_growth_histogram: Histogram,
    /// The total number of reallocations caused by object shrinkage.
    pub realloc_shrink_count: usize,
    /// The sum of all shrink reallocations.
    pub realloc_shrink_sum: usize,
    /// The average size of reallocations caused by object shrinkage.
    pub realloc_shrink_avg: Option<usize>,
    /// The shrink reallocation size buckets.
    pub realloc_shrink_histogram: Histogram,
    /// The total number of full reallocations.
    pub realloc_move_count: usize,
    /// The sum of all full reallocations.
    pub realloc_move_sum: usize,
    /// The average size of full reallocations.
    pub realloc_move_avg: Option<usize>,
    /// The total number of failed reallocations.
    pub realloc_fail_count: usize,
    /// Current heap use.
    pub use_curr: usize,
    /// Maximum recorded heap use.
    pub use_max: usize,
}

impl<A: GlobalAlloc> Heapster<A> {
    /// Returns a summary of the allocator's stats.
    pub fn stats(&self) -> Stats {
        let alloc_count = self.alloc_count();
        let alloc_sum = self.alloc_sum();
        let alloc_avg = alloc_sum.checked_div(alloc_count);
        let alloc_histogram = self.alloc_histogram();
        let alloc_fail_count = self.alloc_fail_count();

        let dealloc_count = self.dealloc_count();
        let dealloc_sum = self.dealloc_sum();
        let dealloc_avg = dealloc_sum.checked_div(dealloc_count);

        let realloc_growth_count = self.realloc_growth_count();
        let realloc_growth_sum = self.realloc_growth_sum();
        let realloc_growth_avg = realloc_growth_sum.checked_div(realloc_growth_count);
        let realloc_growth_histogram = self.realloc_growth_histogram();

        let realloc_shrink_count = self.realloc_shrink_count();
        let realloc_shrink_sum = self.realloc_shrink_sum();
        let realloc_shrink_avg = realloc_shrink_sum.checked_div(realloc_shrink_count);
        let realloc_shrink_histogram = self.realloc_shrink_histogram();

        let realloc_move_count = self.realloc_move_count();
        let realloc_move_sum = self.realloc_move_sum();
        let realloc_move_avg = realloc_move_sum.checked_div(realloc_move_count);
        let realloc_fail_count = self.realloc_fail_count();

        let use_curr = self.use_curr();
        let use_max = self.use_max();

        Stats {
            alloc_count,
            alloc_sum,
            alloc_avg,
            alloc_histogram,
            alloc_fail_count,
            dealloc_count,
            dealloc_sum,
            dealloc_avg,
            realloc_growth_count,
            realloc_growth_sum,
            realloc_growth_avg,
            realloc_growth_histogram,
            realloc_shrink_count,
            realloc_shrink_sum,
            realloc_shrink_avg,
            realloc_shrink_histogram,
            realloc_move_count,
            realloc_move_sum,
            realloc_move_avg,
            realloc_fail_count,
            use_curr,
            use_max,
        }
    }
}

impl Sub<&Stats> for &Stats {
    type Output = Stats;

    fn sub(self, old: &Stats) -> Stats {
        let alloc_count = self.alloc_count.saturating_sub(old.alloc_count);
        let alloc_sum = self.alloc_sum.saturating_sub(old.alloc_sum);
        let alloc_avg = alloc_sum.checked_div(alloc_count);
        let alloc_histogram = self.alloc_histogram - old.alloc_histogram;
        let alloc_fail_count = self.alloc_fail_count.saturating_sub(old.alloc_fail_count);

        let dealloc_count = self.dealloc_count.saturating_sub(old.dealloc_count);
        let dealloc_sum = self.dealloc_sum.saturating_sub(old.dealloc_sum);
        let dealloc_avg = dealloc_sum.checked_div(dealloc_count);

        let realloc_growth_count = self
            .realloc_growth_count
            .saturating_sub(old.realloc_growth_count);
        let realloc_growth_sum = self
            .realloc_growth_sum
            .saturating_sub(old.realloc_growth_sum);
        let realloc_growth_avg = realloc_growth_sum.checked_div(realloc_growth_count);
        let realloc_growth_histogram = self.realloc_growth_histogram - old.realloc_growth_histogram;

        let realloc_shrink_count = self
            .realloc_shrink_count
            .saturating_sub(old.realloc_shrink_count);
        let realloc_shrink_sum = self
            .realloc_shrink_sum
            .saturating_sub(old.realloc_shrink_sum);
        let realloc_shrink_avg = realloc_shrink_sum.checked_div(realloc_shrink_count);
        let realloc_shrink_histogram = self.realloc_shrink_histogram - old.realloc_shrink_histogram;

        let realloc_move_count = self
            .realloc_move_count
            .saturating_sub(old.realloc_move_count);
        let realloc_move_sum = self.realloc_move_sum.saturating_sub(old.realloc_move_sum);
        let realloc_move_avg = realloc_move_sum.checked_div(realloc_move_count);
        let realloc_fail_count = self
            .realloc_fail_count
            .saturating_sub(old.realloc_fail_count);

        Stats {
            alloc_count,
            alloc_sum,
            alloc_avg,
            alloc_histogram,
            alloc_fail_count,
            dealloc_count,
            dealloc_sum,
            dealloc_avg,
            realloc_growth_count,
            realloc_growth_sum,
            realloc_growth_avg,
            realloc_growth_histogram,
            realloc_shrink_count,
            realloc_shrink_sum,
            realloc_shrink_avg,
            realloc_shrink_histogram,
            realloc_move_count,
            realloc_move_sum,
            realloc_move_avg,
            realloc_fail_count,
            use_curr: self.use_curr,
            use_max: self.use_max,
        }
    }
}

impl Sub<Stats> for Stats {
    type Output = Stats;

    fn sub(self, old: Stats) -> Stats {
        &self - &old
    }
}

impl Sub<&Stats> for Stats {
    type Output = Stats;

    fn sub(self, old: &Stats) -> Stats {
        &self - old
    }
}

impl Sub<Stats> for &Stats {
    type Output = Stats;

    fn sub(self, old: Stats) -> Stats {
        self - &old
    }
}
