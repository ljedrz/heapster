use std::alloc::System;

use heapster::Heapster;
use rand::{RngExt, SeedableRng, rng, rngs::SmallRng};

#[global_allocator]
static GLOBAL: Heapster<System> = Heapster::new(System);

#[test]
fn random_allocs() {
    // A bit of test config.
    let min_alloc_size = 2;
    let max_alloc_size = 100;

    // Prepare counters holding expected values.
    let mut num_allocs = 0;
    let mut sum_allocs = 0u64;
    let mut num_deallocs = 0;
    let mut sum_deallocs = 0u64;
    let mut num_realloc_growth = 0;
    let mut sum_realloc_growth = 0u64;
    let mut num_realloc_shrink = 0;
    let mut sum_realloc_shrink = 0u64;
    let mut curr_use;
    let mut max_alloc = 0;

    // Prepare a source of randomness.
    let mut rng = SmallRng::from_rng(&mut rng());

    // Start with a blank slate.
    GLOBAL.reset();

    // Initialize current and max mem use.
    curr_use = GLOBAL.use_curr();
    let max_init_use = GLOBAL.use_max();

    for _ in 0..1000 {
        // Create a small allocation of a random size.
        let alloc_size: usize = rng.random_range(min_alloc_size..=max_alloc_size);
        let mut alloc: Vec<u8> = Vec::with_capacity(alloc_size);

        // Possibly update the max alloc size.
        if alloc_size > max_alloc {
            max_alloc = alloc_size;
        }

        // Update the manual counters.
        num_allocs += 1;
        sum_allocs += alloc_size as u64;
        curr_use += alloc_size;

        // Check the allocation stats.
        assert_eq!(GLOBAL.alloc_count(), num_allocs);
        assert_eq!(GLOBAL.alloc_sum(), sum_allocs);
        assert_eq!(
            GLOBAL.alloc_avg(),
            sum_allocs
                .checked_div(num_allocs as u64)
                .map(|avg| avg as usize)
        );

        // Check the allocation stats.
        assert_eq!(GLOBAL.dealloc_count(), num_deallocs);
        assert_eq!(GLOBAL.dealloc_sum(), sum_deallocs);
        assert_eq!(
            GLOBAL.dealloc_avg(),
            sum_deallocs
                .checked_div(num_deallocs as u64)
                .map(|avg| avg as usize)
        );

        // Check the growth reallocation stats.
        assert_eq!(GLOBAL.realloc_growth_count(), num_realloc_growth);
        assert_eq!(GLOBAL.realloc_growth_sum(), sum_realloc_growth);
        assert_eq!(
            GLOBAL.realloc_growth_avg(),
            sum_realloc_growth
                .checked_div(num_realloc_growth as u64)
                .map(|avg| avg as usize)
        );

        // Check the shrink reallocation stats.
        assert_eq!(GLOBAL.realloc_shrink_count(), num_realloc_shrink);
        assert_eq!(GLOBAL.realloc_shrink_sum(), sum_realloc_shrink);
        assert_eq!(
            GLOBAL.realloc_shrink_avg(),
            sum_realloc_shrink
                .checked_div(num_realloc_shrink as u64)
                .map(|avg| avg as usize)
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
                sum_realloc_growth += realloc_size as u64;

                curr_use += realloc_size;
                assert_eq!(GLOBAL.use_curr(), curr_use);

                // Update the counters related to the end of the scope.
                // note: these values are offset by those at the end of
                // the iteration.
                sum_deallocs += realloc_size as u64;
                curr_use -= realloc_size;
            }
            2 if sum_deallocs >= min_alloc_size as u64 => {
                // Shrink the existing allocation.
                let realloc_size = rng.random_range(1..min_alloc_size);
                alloc.shrink_to(alloc_size - realloc_size);

                // Update the counters after the reallocation.
                num_realloc_shrink += 1;
                sum_realloc_shrink += realloc_size as u64;

                curr_use -= realloc_size;
                assert_eq!(GLOBAL.use_curr(), curr_use);

                // Update the counters related to the end of the scope.
                // note: these values are offset by those at the end of
                // the iteration.
                sum_deallocs -= realloc_size as u64;
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
                sum_allocs += alloc2_size as u64;

                curr_use += alloc2_size;
                assert_eq!(GLOBAL.use_curr(), curr_use);

                // Update the counters related to the end of the scope.
                num_deallocs += 1;
                sum_deallocs += alloc2_size as u64;
                curr_use -= alloc2_size;
            }
            _ => {
                // skip in case of wrong operation order
            }
        }

        // The original `alloc` gets dropped here.
        num_deallocs += 1;
        sum_deallocs += alloc_size as u64;
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
