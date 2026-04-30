# 🦀 heapster

[![crates.io](https://img.shields.io/crates/v/heapster)](https://crates.io/crates/heapster)
[![docs.rs](https://docs.rs/heapster/badge.svg)](https://docs.rs/heapster)
[![actively maintained](https://img.shields.io/badge/Maintenance%20Level-Actively%20Maintained-green.svg)](https://gist.github.com/cheerfulstoic/d107229326a01ff0f333a1d3476e068d)

**Artisanal, pure-atomic heap telemetry for Rust.**

`heapster` is a stupid-lightweight, generic wrapper over any `GlobalAlloc` that tracks allocations, deallocations, and reallocations using pure relaxed atomics.

It is designed to be always-on, allowing you to hunt down pathological memory patterns, diff heap usage between code paths, and export raw allocator metrics to your telemetry dashboards with minimal overhead.

### Why Heapster?

Heavyweight heap profilers are great for deep-dives, but they are often too slow for production and too complex for simple CI assertions. `heapster` fills the gap:

- **Pure Atomics**: No mutexes, no thread-locals, no external viewer files. Just `AtomicUsize` with `Ordering::Relaxed`.
- **Plug-and-Play Generic**: Wrap `System`, `jemalloc`, `mimalloc`, or any custom allocator.
- **Pathology Hunting**: Logarithmic size bucketing (histograms) tells you exactly what sizes are dominating your heap.
- **Deep Realloc Tracking**: Distinguishes between reallocations that grew in-place, shrank in-place, or forced a full memory move (copying).
- **Benchmarking & Diffing**: Exposes a `measure` method so you can take clean snapshots of memory behavior around hot loops.

### Quickstart

Add `heapster` to your `Cargo.toml`. Enable the `fmt` feature if you want pretty, human-readable terminal histograms:

```TOML
[dependencies]
heapster = { version = "0.1", features = ["fmt"] }
```

Wrap your global allocator of choice (e.g., `System`) in your `main.rs` or `lib.rs`:

```Rust
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

### Use Cases
**1. Zero-Friction Benchmarking & Diffing**

Stop guessing if a PR increased allocations. `heapster` lets you measure the heap stats of critical sections of code.

```Rust
let (result, heap_diff) = GLOBAL.measure(|| operation_to_measure());
assert!(heap_diff.alloc_count < 10, "Regression: The operation allocated too many times!");
```

**2. Catching Reallocation Thrashing**

Appending to a `Vec` or `String` without pre-allocating capacity causes the allocator to move memory around. `heapster` specifically tracks `realloc_move_count` so you can find and eliminate performance-killing memory copies.

**3. Always-On Production Metrics**

Because `heapster` is purely atomic, it is perfectly safe to leave on in production. You can easily wire `GLOBAL.stats()` into your Prometheus or Grafana `/metrics` endpoint to monitor `use_curr`, `use_max`, and allocation counts in real-time.

### Simple Histogram Output

When you enable the `fmt` feature, using `.stats()` generates a clear, human-readable overview of your program's memory behavior, including logarithmic ASCII histograms so you can visualize your allocation sizes:

```Plaintext
alloc_count: 10,949,628
alloc_avg: 2.45 KiB

dealloc_count: 10,949,372
dealloc_avg: 4.09 KiB

realloc_growth_count: 365,968
realloc_growth_avg: 49.12 KiB

realloc_shrink_count: 0

realloc_move_count: 351,933
realloc_move_avg: 7.21 KiB

use_curr: 260.39 KiB
use_max: 25.01 MiB

alloc_buckets:
[      4 B ..       8 B):            2  █
[      8 B ..      16 B):      642,064  ███████████
[     16 B ..      32 B):          155  █
[     32 B ..      64 B):    1,639,279  █████████████████████████████
[     64 B ..     128 B):    1,926,643  ██████████████████████████████████
[    128 B ..     256 B):    1,123,746  ████████████████████
[    256 B ..     512 B):    1,284,154  ██████████████████████
[    512 B ..     1 KiB):    2,246,658  ████████████████████████████████████████
[    1 KiB ..     2 KiB):    1,283,935  ██████████████████████
[    2 KiB ..     4 KiB):      160,612  ██
[    4 KiB ..     8 KiB):          411  █
[    8 KiB ..    16 KiB):      320,985  █████
[   16 KiB ..    32 KiB):            1  █
[   32 KiB ..    64 KiB):            1  █
[   64 KiB ..   128 KiB):      320,982  █████

realloc_growth_buckets:
[      1 B ..       2 B):           16  █
[      2 B ..       4 B):            0  
[      4 B ..       8 B):            0  
[      8 B ..      16 B):            0  
[     16 B ..      32 B):            0  
[     32 B ..      64 B):       25,477  ███
[     64 B ..     128 B):       14,976  █
[    128 B ..     256 B):        4,411  █
[    256 B ..     512 B):          106  █
[    512 B ..     1 KiB):            0  
[    1 KiB ..     2 KiB):            0  
[    2 KiB ..     4 KiB):            0  
[    4 KiB ..     8 KiB):            0  
[    8 KiB ..    16 KiB):            0  
[   16 KiB ..    32 KiB):            0  
[   32 KiB ..    64 KiB):      320,982  ████████████████████████████████████████

realloc_shrink_buckets: (empty)
```
