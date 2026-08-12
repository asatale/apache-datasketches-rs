//! Throughput harness for `ArrayOfDoublesSketch::update_*`.
//!
//! This exists because the ArrayOfDoubles update path crosses the FFI
//! boundary once per item and is the only hot loop in the crate where
//! per-call overhead in the shim is measurable against upstream C++. Run it
//! before and after any change to that path.
//!
//! Run with (release matters — a debug build measures nothing useful):
//!   cargo run --release --example bench_tuple_update --features tuple
//!   cargo run --release --example bench_tuple_update --features tuple -- 100000000
//!
//! Fixed parameters, so numbers are comparable across runs: `lg_k = 12`,
//! `num_values = 3`, `resize_factor` and `p` at their defaults. The item
//! count defaults to 10M and can be overridden as the first argument.
//!
//! This measures absolute throughput. To get the number that actually matters
//! — how much of the cost is *this binding* rather than the algorithm — run
//! the native C++ counterpart on the same item count and divide:
//!
//!   ./benches/cpp_reference/run.sh 10000000
//!
//! That program mirrors this one's parameters and scenarios exactly. Keep the
//! two in sync: if you change `LG_K`, `NUM_VALUES` or `HOT_KEY_SPACE` here,
//! change them there too. Both print the sketch estimate, and the estimates
//! must match — that is the cheap check that they are doing the same work.
//!
//! Three scenarios, because they exercise different halves of upstream's
//! `update_tuple_sketch::update`:
//!
//! - `distinct` — every key is new. Once theta drops below 1.0 most keys are
//!   rejected by `hash_and_screen`, which returns *before* upstream ever
//!   reads the values. Per-call work in the shim that happens ahead of that
//!   screen is pure waste here.
//! - `hot` — keys drawn from a space small enough to stay fully retained, so
//!   every call reaches the summary-combine path.
//! - `str` — the string key path, which crosses the boundary as a borrowed
//!   `(pointer, length)` pair rather than an integer. The C++ counterpart
//!   calls the same `(data, length)` overload the shim does, so the difference
//!   between them is binding overhead and not a choice of overload.

use apache_datasketches::tuple::ArrayOfDoublesSketchBuilder;
use std::time::{Duration, Instant};

const LG_K: u8 = 12;
const NUM_VALUES: u8 = 3;
const HOT_KEY_SPACE: u64 = 1 << 10;

/// Values passed on every update. Length must equal `NUM_VALUES`.
const VALUES: [f64; 3] = [1.0, 2.0, 3.0];

/// Size of the pre-built string-key pool. See `string_keys`.
const STR_KEY_SPACE: u64 = 1 << 16;

/// Built once, outside every timed region: formatting a key costs more than
/// the update does, and it costs a different amount in each language, so
/// including it would swamp the per-call delta this harness exists to show.
/// Keep the format identical to the C++ counterpart or the estimates diverge.
fn string_keys() -> Vec<String> {
    (0..STR_KEY_SPACE).map(|i| format!("key_{i:010}")).collect()
}

fn report(label: &str, items: u64, elapsed: Duration, estimate: f64) {
    let nanos = elapsed.as_secs_f64() * 1e9;
    let per_op = nanos / items as f64;
    let rate = items as f64 / elapsed.as_secs_f64() / 1e6;
    println!(
        "{label:9}  {items:>12} items  {:>8.3} s  {per_op:>7.2} ns/op  {rate:>8.1} M/s  \
         (estimate {estimate:.0})",
        elapsed.as_secs_f64()
    );
}

fn bench_distinct(items: u64) {
    let mut sketch = ArrayOfDoublesSketchBuilder::new()
        .lg_k(LG_K)
        .num_values(NUM_VALUES)
        .build()
        .expect("builder rejected fixed valid parameters");

    let start = Instant::now();
    for key in 0..items {
        sketch
            .update_u64(key, &VALUES)
            .expect("update rejected a correctly-sized value slice");
    }
    let elapsed = start.elapsed();

    // Reading the estimate keeps the loop above from being optimised out.
    report("distinct", items, elapsed, sketch.get_estimate());
}

fn bench_hot(items: u64) {
    let mut sketch = ArrayOfDoublesSketchBuilder::new()
        .lg_k(LG_K)
        .num_values(NUM_VALUES)
        .build()
        .expect("builder rejected fixed valid parameters");

    let start = Instant::now();
    for i in 0..items {
        sketch
            .update_u64(i % HOT_KEY_SPACE, &VALUES)
            .expect("update rejected a correctly-sized value slice");
    }
    let elapsed = start.elapsed();

    report("hot", items, elapsed, sketch.get_estimate());
}

fn bench_str(items: u64) {
    let mut sketch = ArrayOfDoublesSketchBuilder::new()
        .lg_k(LG_K)
        .num_values(NUM_VALUES)
        .build()
        .expect("builder rejected fixed valid parameters");

    let keys = string_keys();
    let start = Instant::now();
    for i in 0..items {
        sketch
            .update_str(&keys[(i % STR_KEY_SPACE) as usize], &VALUES)
            .expect("update rejected a correctly-sized value slice");
    }
    let elapsed = start.elapsed();

    report("str", items, elapsed, sketch.get_estimate());
}

fn main() {
    let items: u64 = std::env::args()
        .nth(1)
        .map(|arg| arg.parse().expect("item count must be a positive integer"))
        .unwrap_or(10_000_000);

    println!("lg_k={LG_K} num_values={NUM_VALUES} items={items}");
    bench_distinct(items);
    bench_hot(items);
    bench_str(items);
}
