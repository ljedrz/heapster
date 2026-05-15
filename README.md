# 📊 heapster

[![crates.io](https://img.shields.io/crates/v/heapster)](https://crates.io/crates/heapster)
[![docs.rs](https://docs.rs/heapster/badge.svg)](https://docs.rs/heapster)

**Lightweight heap telemetry for Rust, built on relaxed atomics.**

`heapster` is a lightweight, generic wrapper over any `GlobalAlloc` that tracks allocations, deallocations, and reallocations using pure relaxed atomics.

It is designed to be always-on, allowing you to identify allocation patterns, diff heap usage between code paths, and export raw allocator metrics to your telemetry dashboards with minimal overhead.

### Why Heapster?

Heap profilers like dhat or heaptrack capture rich per-allocation data but add significant overhead and require dedicated viewers. Heapster occupies a lighter tier: aggregate counters and histograms only, with overhead low enough to leave on in production.

- **Atomics-only**: No mutexes, no thread-locals, no external viewer files. Just relaxed atomic counters.
- **`no_std` by default**: Uses only `core` and `alloc` in the default build, with no third-party dependencies. The `fmt` and `serde` features add `std` requirements.
- **Generic over any allocator**: Wraps `System`, jemalloc, mimalloc, or any custom `GlobalAlloc`.
- **Size histograms**: Power-of-two buckets for allocations and reallocations make the size distribution visible at a glance.
- **Realloc classification**: Distinguishes between reallocations that grew in-place, shrank in-place, or forced a full memory move (copying).
- **Snapshot diffing**: `measure()` returns a `Stats` delta for a closure, suitable for assertion-style tests and benchmark comparisons.

### Quickstart

Add `heapster` to your `Cargo.toml`. Enable the `fmt` feature if you want pretty, human-readable terminal histograms:

```toml
[dependencies]
heapster = { version = "0.5", features = ["fmt"] }
```

Wrap your global allocator of choice (e.g., `System`) in your `main.rs` or `lib.rs`:

```rust
use heapster::Heapster;
use std::alloc::System;

#[global_allocator]
static GLOBAL: Heapster<System> = Heapster::new(System);

fn main() {
    // ... do some heavy work ...

    // See what has transpired in the heap
    println!("{}", GLOBAL.stats());
}
```

### Features

Cargo features are opt-in; the default build has no non-`std` dependencies.

- `fmt` — `Display` impls for `Stats` and `Histogram`, including ASCII-rendered histograms. Pulls in `humansize` and `num-format`.
- `serde` — `Serialize`/`Deserialize` impls for `Stats` and `Histogram`, for exporting snapshots to JSON, MessagePack, or other formats consumed by metrics pipelines.

```toml
[dependencies]
heapster = { version = "0.5", features = ["fmt", "serde"] }
```

### Use Cases
**1. Benchmarking and regression tests**

Stop guessing if a PR increased allocations. `heapster` lets you measure the heap stats of critical sections of code.

```rust
let (result, heap_diff) = GLOBAL.measure(|| operation_to_measure());
assert!(heap_diff.alloc_count < 10, "Regression: The operation allocated too many times!");
```

**2. Catching reallocation thrashing**

When a `Vec` or `String` grows beyond its capacity, the underlying buffer may be moved to a new location, copying the contents. Heapster's `realloc_move_count` makes these moves visible so you can pre-size collections that thrash.

**3. Always-On production metrics**

Overhead is a small constant per allocation (typically tens of nanoseconds for the atomic operations), so Heapster can be left on in production. `stats()` exposes a `Stats` struct that's straightforward to wire into a Prometheus or other metrics endpoint, especially with the `serde` feature enabled.

### Simple Histogram Output

The `fmt` feature provides `Display` impls that render stats and ASCII histograms.

```plaintext
alloc_count: 10,949,628
alloc_avg: 2.45 KiB

dealloc_count: 10,949,372
dealloc_avg: 4.09 KiB

realloc_growth_count: 365,968
realloc_growth_avg: 49.12 KiB

realloc_move_count: 351,933
realloc_move_avg: 7.21 KiB

use_curr: 260.39 KiB
use_max: 25.01 MiB

alloc_histogram:
[    4 B ..     8 B):         2  █
[    8 B ..    16 B):   642,064  ███████████
[   16 B ..    32 B):       155  █
[   32 B ..    64 B): 1,639,279  █████████████████████████████
[   64 B ..   128 B): 1,926,643  ██████████████████████████████████
[  128 B ..   256 B): 1,123,746  ████████████████████
[  256 B ..   512 B): 1,284,154  ██████████████████████
[  512 B ..   1 KiB): 2,246,658  ████████████████████████████████████████
[  1 KiB ..   2 KiB): 1,283,935  ██████████████████████
[  2 KiB ..   4 KiB):   160,612  ██
[  4 KiB ..   8 KiB):       411  █
[  8 KiB ..  16 KiB):   320,985  █████
[ 16 KiB ..  32 KiB):         1  █
[ 32 KiB ..  64 KiB):         1  █
[ 64 KiB .. 128 KiB):   320,982  █████

realloc_growth_histogram:
[    1 B ..     2 B):        16  █
[    2 B ..     4 B):         0  
[    4 B ..     8 B):         0  
[    8 B ..    16 B):         0  
[   16 B ..    32 B):         0  
[   32 B ..    64 B):    25,477  ███
[   64 B ..   128 B):    14,976  █
[  128 B ..   256 B):     4,411  █
[  256 B ..   512 B):       106  █
[  512 B ..   1 KiB):         0  
[  1 KiB ..   2 KiB):         0  
[  2 KiB ..   4 KiB):         0  
[  4 KiB ..   8 KiB):         0  
[  8 KiB ..  16 KiB):         0  
[ 16 KiB ..  32 KiB):         0  
[ 32 KiB ..  64 KiB):   320,982  ████████████████████████████████████████
```

### License

Dual-licensed under either of:

- Creative Commons Zero v1.0 Universal ([LICENSE-CC0](LICENSE-CC0))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
