use std::alloc::System;

use heapster::{Heapster, Histogram};
use rand::{RngExt, SeedableRng, rng, rngs::SmallRng};

#[global_allocator]
static GLOBAL: Heapster<System> = Heapster::new(System);

// TODO: fix the occasional flakiness
#[test]
fn integration() {
    // A bit of test config.
    let min_alloc_size = 2;
    let max_alloc_size = 100;

    // Prepare counters holding expected values.
    let mut num_allocs = 0;
    let mut sum_allocs = 0;
    let mut num_deallocs = 0;
    let mut sum_deallocs = 0;
    let mut num_realloc_growth = 0;
    let mut sum_realloc_growth = 0;
    let mut num_realloc_shrink = 0;
    let mut sum_realloc_shrink = 0;
    let mut curr_use;
    let mut max_alloc = 0;

    // Prepare a source of randomness.
    let mut rng = SmallRng::from_rng(&mut rng());

    // Start with a blank slate.
    GLOBAL.reset();

    // Initialize current and max mem use.
    curr_use = GLOBAL.use_curr();
    let max_init_use = GLOBAL.use_max();

    for _ in 0..100 {
        // Create a small allocation of a random size.
        let alloc_size: usize = rng.random_range(min_alloc_size..=max_alloc_size);
        let mut alloc: Vec<u8> = Vec::with_capacity(alloc_size);

        // Possibly update the max alloc size.
        if alloc_size > max_alloc {
            max_alloc = alloc_size;
        }

        // Update the manual counters.
        num_allocs += 1;
        sum_allocs += alloc_size;
        curr_use += alloc_size;

        // Check the allocation stats.
        assert_eq!(GLOBAL.alloc_count(), num_allocs);
        assert_eq!(GLOBAL.alloc_sum(), sum_allocs);
        assert_eq!(GLOBAL.alloc_avg(), sum_allocs.checked_div(num_allocs));

        // Check the allocation stats.
        assert_eq!(GLOBAL.dealloc_count(), num_deallocs);
        assert_eq!(GLOBAL.dealloc_sum(), sum_deallocs);
        assert_eq!(GLOBAL.dealloc_avg(), sum_deallocs.checked_div(num_deallocs));

        // Check the growth reallocation stats.
        assert_eq!(GLOBAL.realloc_growth_count(), num_realloc_growth);
        assert_eq!(GLOBAL.realloc_growth_sum(), sum_realloc_growth);
        assert_eq!(
            GLOBAL.realloc_growth_avg(),
            sum_realloc_growth.checked_div(num_realloc_growth)
        );

        // Check the shrink reallocation stats.
        assert_eq!(GLOBAL.realloc_shrink_count(), num_realloc_shrink);
        assert_eq!(GLOBAL.realloc_shrink_sum(), sum_realloc_shrink);
        assert_eq!(
            GLOBAL.realloc_shrink_avg(),
            sum_realloc_shrink.checked_div(num_realloc_shrink)
        );

        // The move count isn't deterministic.
        assert!(
            GLOBAL.realloc_move_count()
                <= GLOBAL.realloc_growth_count() + GLOBAL.realloc_shrink_count()
        );

        // Check current and max heap use stats.
        assert_eq!(GLOBAL.use_curr(), curr_use);
        assert_eq!(GLOBAL.use_max(), max_init_use + max_alloc);

        // Potentially carry out an additional action.
        let bonus_action: u8 = rng.random_range(0..4);
        match bonus_action {
            0 => {
                // Don't do anything.
            }
            1 => {
                // Grow by the existing allocation.
                let realloc_size = rng.random_range(min_alloc_size..max_alloc_size);
                alloc.reserve_exact(alloc_size + realloc_size);

                // Possibly update the max alloc size.
                if alloc_size + realloc_size > max_alloc {
                    max_alloc = alloc_size + realloc_size;
                }

                // Update the counters after the reallocation.
                num_realloc_growth += 1;
                sum_realloc_growth += realloc_size;

                curr_use += realloc_size;
                assert_eq!(GLOBAL.use_curr(), curr_use);

                // Update the counters related to the end of the scope.
                // note: these values are offset by those at the end of
                // the iteration.
                sum_deallocs += realloc_size;
                curr_use -= realloc_size;
            }
            2 => {
                // Shrink the existing allocation.
                let realloc_size = rng.random_range(1..min_alloc_size);
                alloc.shrink_to(alloc_size - realloc_size);

                // Update the counters after the reallocation.
                num_realloc_shrink += 1;
                sum_realloc_shrink += realloc_size;

                curr_use -= realloc_size;
                assert_eq!(GLOBAL.use_curr(), curr_use);

                // Update the counters related to the end of the scope.
                // note: these values are offset by those at the end of
                // the iteration.
                sum_deallocs -= realloc_size;
                curr_use += realloc_size;
            }
            3 => {
                // Add an extra allocation.
                let alloc2_size: usize = rng.random_range(min_alloc_size..=max_alloc_size);
                let _alloc2: Vec<u8> = Vec::with_capacity(alloc2_size);

                // Possibly update the max alloc size.
                if alloc_size + alloc2_size > max_alloc {
                    max_alloc = alloc_size + alloc2_size;
                }

                // Update the relevant manual counters.
                num_allocs += 1;
                sum_allocs += alloc2_size;

                curr_use += alloc2_size;
                assert_eq!(GLOBAL.use_curr(), curr_use);

                // Update the counters related to the end of the scope.
                num_deallocs += 1;
                sum_deallocs += alloc2_size;
                curr_use -= alloc2_size;
            }
            _ => unreachable!(),
        }

        // The original `alloc` gets dropped here.
        num_deallocs += 1;
        sum_deallocs += alloc_size;
        curr_use -= alloc_size;
    }

    // Reset the stats.
    GLOBAL.reset();

    // Check that all the relevant counters are now 0.
    assert_eq!(GLOBAL.alloc_count(), 0);
    assert_eq!(GLOBAL.alloc_sum(), 0);
    assert_eq!(GLOBAL.alloc_avg(), None);
    assert_eq!(GLOBAL.dealloc_count(), 0);
    assert_eq!(GLOBAL.dealloc_sum(), 0);
    assert_eq!(GLOBAL.dealloc_avg(), None);
    assert_eq!(GLOBAL.realloc_growth_count(), 0);
    assert_eq!(GLOBAL.realloc_growth_sum(), 0);
    assert_eq!(GLOBAL.realloc_growth_avg(), None);
    assert_eq!(GLOBAL.realloc_shrink_count(), 0);
    assert_eq!(GLOBAL.realloc_shrink_sum(), 0);
    assert_eq!(GLOBAL.realloc_shrink_avg(), None);
    assert_eq!(GLOBAL.realloc_move_count(), 0);
    assert_eq!(GLOBAL.realloc_move_sum(), 0);
    assert_eq!(GLOBAL.realloc_move_avg(), None);
}

#[test]
fn measure_counts() {
    // Start with a blank slate.
    GLOBAL.reset();

    let (_, stats) = GLOBAL.measure(|| {});
    assert_eq!(stats.alloc_count, 0);
    assert_eq!(stats.alloc_sum, 0);

    let _ = Vec::<u8>::with_capacity(2);

    let stats = GLOBAL.stats();
    assert_eq!(stats.alloc_count, 1);
    assert_eq!(stats.alloc_sum, 2);

    let (_, stats) = GLOBAL.measure(|| Vec::<u8>::with_capacity(8));
    assert_eq!(stats.alloc_count, 1);
    assert_eq!(stats.alloc_sum, 8);

    let stats = GLOBAL.stats();
    assert_eq!(stats.alloc_count, 2);
    assert_eq!(stats.alloc_sum, 10);
}

#[test]
fn measure_returns_value() {
    GLOBAL.reset();
    let (result, _stats) = GLOBAL.measure(|| 42);
    assert_eq!(result, 42);
}

#[test]
fn failures() {
    use std::alloc::{GlobalAlloc, Layout};

    GLOBAL.reset();
    let before = GLOBAL.alloc_fail_count();

    // a huge layout; the system allocator will return null
    let layout = Layout::from_size_align(usize::MAX / 2, 1).unwrap();
    let ptr = unsafe { GLOBAL.alloc(layout) };
    assert!(ptr.is_null());

    assert_eq!(GLOBAL.alloc_fail_count(), before + 1);
    assert_eq!(GLOBAL.alloc_count(), 0); // failure didn't bump success counter
}

#[test]
fn histogram_quantile() {
    // empty histogram returns None
    let empty = Histogram::default();
    assert_eq!(empty.quantile(0.5), None);

    // out-of-range q returns None
    let mut buckets = [0usize; 64];
    buckets[5] = 10; // bucket 5 = [32, 64)
    let h = Histogram { buckets };
    assert_eq!(h.quantile(-0.1), None);
    assert_eq!(h.quantile(1.1), None);

    // all values in one bucket: quantile is somewhere in [32, 64)
    let q50 = h.quantile(0.5).unwrap();
    assert!((32..64).contains(&q50));

    // monotonicity
    let q10 = h.quantile(0.1).unwrap();
    let q90 = h.quantile(0.9).unwrap();
    assert!(q10 <= q50 && q50 <= q90);
}

#[test]
fn stats_subtraction() {
    GLOBAL.reset();

    let before = GLOBAL.stats();
    let _v = Vec::<u8>::with_capacity(64);
    let after = GLOBAL.stats();

    let diff = &after - &before;
    assert_eq!(diff.alloc_count, 1);
    assert_eq!(diff.alloc_sum, 64);
    assert_eq!(diff.dealloc_count, 0); // _v still alive

    // saturating sub: subtracting a "future" snapshot from an old one
    // doesn't panic
    let weird = &before - &after;
    assert_eq!(weird.alloc_count, 0); // saturated, not wrapped
}

#[test]
#[cfg(feature = "serde")]
fn serde_roundtrip() {
    let mut buckets = [0usize; 64];
    buckets[3] = 7;
    buckets[63] = 1; // exercise the top bucket
    let h = Histogram { buckets };

    let json = serde_json::to_string(&h).unwrap();
    let h2: Histogram = serde_json::from_str(&json).unwrap();
    assert_eq!(h.buckets(), h2.buckets());
}
