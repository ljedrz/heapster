//! Benchmarks aimed at exercising contention on Heapster's atomic counters
//! under concurrent allocator workloads.

use std::{alloc::System, hint::black_box};

use heapster::Heapster;

#[global_allocator]
static GLOBAL: Heapster<System> = Heapster::new(System);

fn main() {
    divan::main();
}

const THREADS: &[usize] = &[1, 2, 4, 8, 16];
const SAMPLE_COUNT: u32 = 200;

/// Pure alloc/dealloc churn. Stresses ALLOC_COUNT, ALLOC_SUM, DEALLOC_COUNT,
/// DEALLOC_SUM, USE_CURR, USE_MAX, and one bucket per allocation. This is
/// the workload where USE_CURR/USE_MAX false sharing is most observable —
/// every alloc and every dealloc touches both of them.
#[divan::bench(threads = THREADS, sample_count = SAMPLE_COUNT)]
fn alloc_churn() {
    // Many short-lived allocations of varying sizes — varying the size
    // spreads writes across bucket atomics, which surfaces any false
    // sharing within the bucket array.
    for i in 0..8192u64 {
        let size = 16 << (i & 0b111); // sizes 16, 32, 64, ..., 2048, then repeats
        let v: Vec<u8> = vec![0; size];
        black_box(v);
    }
}

/// Mix of alloc, dealloc, and realloc (both growth and shrink). Stresses
/// the realloc counters in addition to everything above, including
/// REALLOC_GROWTH_COUNT/SUM/buckets and shrink-side equivalents.
#[divan::bench(threads = THREADS, sample_count = SAMPLE_COUNT)]
fn vec_growth() {
    // Vec::push starts at capacity 0 and reallocs geometrically: alloc(4),
    // realloc(4 -> 8), realloc(8 -> 16), ... — every iteration touches the
    // realloc-growth atomics, which would otherwise be quiet.
    let mut v: Vec<u64> = Vec::new();
    for i in 0..2048u64 {
        v.push(i);
    }
    v.shrink_to_fit(); // exercise the shrink path too
    black_box(v);
}

/// Long-lived state with intermixed allocs/frees. Closer to a real workload
/// than the two above, where memory stays live and use_curr fluctuates.
#[divan::bench(threads = THREADS, sample_count = SAMPLE_COUNT)]
fn live_set() {
    let mut live: Vec<Vec<u8>> = Vec::with_capacity(64);
    for i in 0..1024u64 {
        // Allocate
        let size = 64 + ((i * 17) & 0x3FF) as usize; // varying sizes 64..1088
        live.push(vec![0u8; size]);

        // Drop something every 4 iters once we have a backlog
        if live.len() > 16 && i % 4 == 0 {
            let idx = (i as usize) % live.len();
            let _dropped = live.swap_remove(idx);
        }
    }
    black_box(live);
}
