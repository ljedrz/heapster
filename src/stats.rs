use std::{alloc::GlobalAlloc, ops::Sub};

use crate::Heapster;

/// A summary of an allocator's stats.
#[derive(Debug, Clone)]
pub struct Stats {
    /// The total number of allocations.
    pub alloc_count: usize,
    /// The sum of all allocations.
    pub alloc_sum: usize,
    /// The average size of allocations.
    pub alloc_avg: Option<usize>,
    /// The allocation size buckets.
    pub alloc_buckets: [usize; 64],
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
    pub realloc_growth_buckets: [usize; 64],
    /// The total number of reallocations caused by object shrinkage.
    pub realloc_shrink_count: usize,
    /// The sum of all shrink reallocations.
    pub realloc_shrink_sum: usize,
    /// The average size of reallocations caused by object shrinkage.
    pub realloc_shrink_avg: Option<usize>,
    /// The shrink reallocation size buckets.
    pub realloc_shrink_buckets: [usize; 64],
    /// The total number of full reallocations.
    pub realloc_move_count: usize,
    /// The sum of all full allocations.
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
        let alloc_buckets = self.alloc_buckets();
        let alloc_fail_count = self.alloc_fail_count();

        let dealloc_count = self.dealloc_count();
        let dealloc_sum = self.dealloc_sum();
        let dealloc_avg = dealloc_sum.checked_div(dealloc_count);

        let realloc_growth_count = self.realloc_growth_count();
        let realloc_growth_sum = self.realloc_growth_sum();
        let realloc_growth_avg = realloc_growth_sum.checked_div(realloc_growth_count);
        let realloc_growth_buckets = self.realloc_growth_buckets();

        let realloc_shrink_count = self.realloc_shrink_count();
        let realloc_shrink_sum = self.realloc_shrink_sum();
        let realloc_shrink_avg = realloc_shrink_sum.checked_div(realloc_shrink_count);
        let realloc_shrink_buckets = self.realloc_shrink_buckets();

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
            alloc_buckets,
            alloc_fail_count,
            dealloc_count,
            dealloc_sum,
            dealloc_avg,
            realloc_growth_count,
            realloc_growth_sum,
            realloc_growth_avg,
            realloc_growth_buckets,
            realloc_shrink_count,
            realloc_shrink_sum,
            realloc_shrink_avg,
            realloc_shrink_buckets,
            realloc_move_count,
            realloc_move_sum,
            realloc_move_avg,
            realloc_fail_count,
            use_curr,
            use_max,
        }
    }
}

fn diff_buckets(base: [usize; 64], old: [usize; 64]) -> [usize; 64] {
    let mut out = [0usize; 64];
    for (i, (b_base, b_old)) in base.iter().zip(&old).enumerate() {
        out[i] = b_base - b_old;
    }
    out
}

impl Sub<&Stats> for &Stats {
    type Output = Stats;

    fn sub(self, old: &Stats) -> Stats {
        let alloc_count = self.alloc_count - old.alloc_count;
        let alloc_sum = self.alloc_sum - old.alloc_sum;
        let alloc_avg = alloc_sum.checked_div(alloc_count);
        let alloc_buckets = diff_buckets(self.alloc_buckets, old.alloc_buckets);
        let alloc_fail_count = self.alloc_fail_count - old.alloc_fail_count;

        let dealloc_count = self.dealloc_count - old.dealloc_count;
        let dealloc_sum = self.dealloc_sum - old.dealloc_sum;
        let dealloc_avg = dealloc_sum.checked_div(dealloc_count);

        let realloc_growth_count = self.realloc_growth_count - old.realloc_growth_count;
        let realloc_growth_sum = self.realloc_growth_sum - old.realloc_growth_sum;
        let realloc_growth_avg = realloc_growth_sum.checked_div(realloc_growth_count);
        let realloc_growth_buckets =
            diff_buckets(self.realloc_growth_buckets, old.realloc_growth_buckets);

        let realloc_shrink_count = self.realloc_shrink_count - old.realloc_shrink_count;
        let realloc_shrink_sum = self.realloc_shrink_sum - old.realloc_shrink_sum;
        let realloc_shrink_avg = realloc_shrink_sum.checked_div(realloc_shrink_count);
        let realloc_shrink_buckets =
            diff_buckets(self.realloc_shrink_buckets, old.realloc_shrink_buckets);

        let realloc_move_count = self.realloc_move_count - old.realloc_move_count;
        let realloc_move_sum = self.realloc_move_sum - old.realloc_move_sum;
        let realloc_move_avg = realloc_move_sum.checked_div(realloc_move_count);
        let realloc_fail_count = self.realloc_fail_count - old.realloc_fail_count;

        Stats {
            alloc_count,
            alloc_sum,
            alloc_avg,
            alloc_buckets,
            alloc_fail_count,
            dealloc_count,
            dealloc_sum,
            dealloc_avg,
            realloc_growth_count,
            realloc_growth_sum,
            realloc_growth_avg,
            realloc_growth_buckets,
            realloc_shrink_count,
            realloc_shrink_sum,
            realloc_shrink_avg,
            realloc_shrink_buckets,
            realloc_move_count,
            realloc_move_sum,
            realloc_move_avg,
            realloc_fail_count,
            use_curr: self.use_curr,
            use_max: self.use_max,
        }
    }
}
