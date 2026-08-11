//! Throughput harness for `CpcSketch::update_u64`.
//!
//! Run with (release matters -- a debug build measures nothing useful):
//!   cargo run --release --example bench_cpc_update --features cpc
//!   cargo run --release --example bench_cpc_update --features cpc -- 100000000
//!
//! Pair with the native C++ counterpart on the same item count to get the
//! binding's overhead, which is the number worth tracking:
//!
//!   ./benches/cpp_reference/run.sh 10000000
//!
//! Fixed parameters so runs are comparable: `lg_k = 12`, defaults elsewhere.
//! Keep them in sync with `benches/cpp_reference/cpc_update.cc`; both print the
//! estimate, and the estimates must match.
//!
//! `cpc::init()` runs first, and the C++ counterpart does the equivalent. CPC
//! builds global decompression tables lazily on first use, and letting that
//! land inside a timed loop would charge one-time setup to whichever scenario
//! ran first.
//!
//! Like HLL, CPC has no theta screen; `hot` differs from `distinct` only in
//! staying cache-resident.

use apache_datasketches::cpc::{init, CpcSketch, CpcSketchBuilder};
use std::time::{Duration, Instant};

const LG_K: u8 = 12;
const HOT_KEY_SPACE: u64 = 1 << 10;

fn report(label: &str, items: u64, elapsed: Duration, estimate: f64) {
    let per_op = elapsed.as_secs_f64() * 1e9 / items as f64;
    let rate = items as f64 / elapsed.as_secs_f64() / 1e6;
    println!(
        "{label:9}  {items:>12} items  {:>8.3} s  {per_op:>7.2} ns/op  {rate:>8.1} M/s  \
         (estimate {estimate:.0})",
        elapsed.as_secs_f64()
    );
}

fn build() -> CpcSketch {
    CpcSketchBuilder::new()
        .lg_k(LG_K)
        .build()
        .expect("fixed valid parameters were rejected")
}

fn bench_distinct(items: u64) {
    let mut sketch = build();
    let start = Instant::now();
    for key in 0..items {
        sketch.update_u64(key);
    }
    let elapsed = start.elapsed();
    // Reading the estimate keeps the loop above from being optimised out.
    report("distinct", items, elapsed, sketch.get_estimate());
}

fn bench_hot(items: u64) {
    let mut sketch = build();
    let start = Instant::now();
    for i in 0..items {
        sketch.update_u64(i % HOT_KEY_SPACE);
    }
    let elapsed = start.elapsed();
    report("hot", items, elapsed, sketch.get_estimate());
}

fn main() {
    let items: u64 = std::env::args()
        .nth(1)
        .map(|arg| arg.parse().expect("item count must be a positive integer"))
        .unwrap_or(10_000_000);

    // Off the hot path on purpose -- see the module docs.
    init();

    println!("lg_k={LG_K} items={items}");
    bench_distinct(items);
    bench_hot(items);
}
