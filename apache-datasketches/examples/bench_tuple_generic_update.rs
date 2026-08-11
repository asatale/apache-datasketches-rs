//! Throughput harness for `TupleSketch::update_*` — the generic (type-erased)
//! Tuple path, where C++ calls back into Rust to clone and combine summaries.
//!
//! Separate from `bench_tuple_update.rs` because the two have genuinely
//! different cost structures. ArrayOfDoubles binds a concrete C++
//! instantiation and its summary is a plain `f64` array; the generic path
//! wraps every summary in a `rust::Box<RustSummary>` and reaches Rust through
//! a trampoline for each clone and combine. There is no native C++ reference
//! for this one — the callback design has no C++ equivalent to compare
//! against, so the number to watch is this file against itself across changes.
//!
//! Run with (release matters — a debug build measures nothing useful):
//!   cargo run --release --example bench_tuple_generic_update --features tuple
//!   cargo run --release --example bench_tuple_generic_update --features tuple -- 100000000
//!
//! Fixed parameters so runs are comparable: `lg_k = 12`, defaults elsewhere,
//! item count defaults to 10M and can be overridden as the first argument.
//!
//! Same two scenarios as the ArrayOfDoubles harness, for the same reason —
//! they exercise different halves of upstream's `update_tuple_sketch::update`:
//!
//! - `distinct` — every key is new, so once theta drops most keys are rejected
//!   by `hash_and_screen`, which returns *before* the update value is read.
//!   Per-call work performed ahead of that screen is pure waste here.
//! - `hot` — keys drawn from a space small enough to stay fully retained, so
//!   every call reaches `union_combine`.
//!
//! The summary below is deliberately the cheapest possible: a single `u64`,
//! with `create`/`union_combine` doing one add. That is the point — it makes
//! the harness measure binding overhead rather than the user's own work.

use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};
use std::time::{Duration, Instant};

const LG_K: u8 = 12;
const HOT_KEY_SPACE: u64 = 1 << 10;

#[derive(Clone)]
struct Count(u64);

impl TupleSummary for Count {
    type Update = ();
    fn create(_: &()) -> Self {
        Count(1)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
}

fn report(label: &str, items: u64, elapsed: Duration, estimate: f64) {
    let per_op = elapsed.as_secs_f64() * 1e9 / items as f64;
    let rate = items as f64 / elapsed.as_secs_f64() / 1e6;
    println!(
        "{label:9}  {items:>12} items  {:>8.3} s  {per_op:>7.2} ns/op  {rate:>8.1} M/s  \
         (estimate {estimate:.0})",
        elapsed.as_secs_f64()
    );
}

fn build() -> TupleSketch<Count> {
    TupleSketchBuilder::new()
        .lg_k(LG_K)
        .build()
        .expect("builder rejected fixed valid parameters")
}

fn bench_distinct(items: u64) {
    let mut sketch = build();
    let start = Instant::now();
    for key in 0..items {
        sketch.update_u64(key, &());
    }
    let elapsed = start.elapsed();
    // Reading the estimate keeps the loop above from being optimised out.
    report("distinct", items, elapsed, sketch.get_estimate());
}

fn bench_hot(items: u64) {
    let mut sketch = build();
    let start = Instant::now();
    for i in 0..items {
        sketch.update_u64(i % HOT_KEY_SPACE, &());
    }
    let elapsed = start.elapsed();
    report("hot", items, elapsed, sketch.get_estimate());
}

fn main() {
    let items: u64 = std::env::args()
        .nth(1)
        .map(|arg| arg.parse().expect("item count must be a positive integer"))
        .unwrap_or(10_000_000);

    println!("lg_k={LG_K} items={items}");
    bench_distinct(items);
    bench_hot(items);
}
