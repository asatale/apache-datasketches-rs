//! Throughput harness for `HllSketch::update_u64`.
//!
//! Run with (release matters — a debug build measures nothing useful):
//!   cargo run --release --example bench_hll_update --features hll
//!   cargo run --release --example bench_hll_update --features hll -- 100000000
//!
//! Pair with the native C++ counterpart on the same item count to get the
//! binding's overhead, which is the number worth tracking:
//!
//!   ./benches/cpp_reference/run.sh 10000000
//!
//! Fixed parameters so runs are comparable: `lg_config_k = 12`, `Hll8`. Keep
//! them in sync with `benches/cpp_reference/hll_update.cc`; both print the
//! estimate, and the estimates must match.
//!
//! `Hll8` rather than the more compact `Hll4`: it is the fastest to update, so
//! it puts the most weight on per-call binding overhead rather than on
//! upstream's bucket packing — which is what this harness is for.
//!
//! Two scenarios, matching the other harnesses. Unlike Theta and Tuple, HLL has
//! no theta screen, so both scenarios do the full update; `hot` differs only in
//! touching one bucket repeatedly and so staying cache-resident.
//!
//! `str` covers the string update path, which crosses the boundary as a
//! borrowed `(pointer, length)` pair rather than an integer. Its C++
//! counterpart calls the same `(data, length)` overload the shim does, so the
//! difference between them is binding overhead and not a choice of overload.

use apache_datasketches::hll::{HllSketch, TargetHllType};
use std::time::{Duration, Instant};

const LG_K: u8 = 12;
const HOT_KEY_SPACE: u64 = 1 << 10;

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
    let per_op = elapsed.as_secs_f64() * 1e9 / items as f64;
    let rate = items as f64 / elapsed.as_secs_f64() / 1e6;
    println!(
        "{label:9}  {items:>12} items  {:>8.3} s  {per_op:>7.2} ns/op  {rate:>8.1} M/s  \
         (estimate {estimate:.0})",
        elapsed.as_secs_f64()
    );
}

fn build() -> HllSketch {
    HllSketch::new(LG_K, TargetHllType::Hll8).expect("fixed valid parameters were rejected")
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

fn bench_str(items: u64) {
    let mut sketch = build();
    let keys = string_keys();
    let start = Instant::now();
    for i in 0..items {
        sketch.update_str(&keys[(i % STR_KEY_SPACE) as usize]);
    }
    let elapsed = start.elapsed();
    report("str", items, elapsed, sketch.get_estimate());
}

fn main() {
    let items: u64 = std::env::args()
        .nth(1)
        .map(|arg| arg.parse().expect("item count must be a positive integer"))
        .unwrap_or(10_000_000);

    println!("lg_config_k={LG_K} target=Hll8 items={items}");
    bench_distinct(items);
    bench_hot(items);
    bench_str(items);
}
