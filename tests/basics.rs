use std::alloc::System;

use heapster::{Heapster, Histogram};

#[global_allocator]
static GLOBAL: Heapster<System> = Heapster::new(System);

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
    let h = Histogram::from_buckets(buckets);
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
    let h = Histogram::from_buckets(buckets);

    let json = serde_json::to_string(&h).unwrap();
    let h2: Histogram = serde_json::from_str(&json).unwrap();
    assert_eq!(h.buckets(), h2.buckets());
}
