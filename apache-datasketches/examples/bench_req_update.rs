//! Throughput harness for `ReqSketch<f64>`: update, query and serialization.
//!
//! Run with (release matters -- a debug build measures nothing useful):
//!   cargo run --release --example bench_req_update --features req
//!   cargo run --release --example bench_req_update --features req -- --ladder
//!
//! Accepts `[ITEMS] [--reps N] [--ladder]`, like every other harness. Pair
//! with `benches/cpp_reference/req_update.cc` via `compare.sh`.
//!
//! Fixed parameters so runs are comparable: `k = 12`, high-rank accuracy. Keep
//! them in sync with the C++ counterpart.
//!
//! REQ compaction is randomized, so unlike the other harnesses this one
//! cannot cross-check an estimate. It prints `n`, `min`, `max` and
//! `retained` (plus `bytes` for ser/deser) instead: they depend only on the
//! input and `k`, so they must match across reps and across languages.

use apache_datasketches::req::{RankAccuracy, ReqSketch};
use std::hint::black_box;
use std::time::{Duration, Instant};

const K: u16 = 12;
const HOT_KEY_SPACE: u64 = 1 << 10;

/// Keep in step with `bench_common.h`; see the comments there.
const SER_CALLS: u64 = 20_000;
const DESER_CALLS: u64 = 5_000;

/// `get_quantile` cost tracks the retained-item count, not the item count,
/// so this is a fixed call count. Keep in step with `req_update.cc`.
const QUERY_CALLS: u64 = 1_000_000;

const LADDER: [u64; 3] = [1_000_000, 10_000_000, 100_000_000];
const DEFAULT_ITEMS: u64 = 10_000_000;
const DEFAULT_REPS: usize = 3;

/// Parses `[ITEMS] [--reps N] [--ladder]`. Same rules as the other harnesses.
fn parse_args() -> (Vec<u64>, usize) {
    let mut items = None;
    let mut reps = DEFAULT_REPS;
    let mut ladder = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ladder" => ladder = true,
            "--reps" => {
                reps = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|&n| n > 0)
                    .expect("--reps needs a positive integer")
            }
            other => {
                let n = other
                    .parse()
                    .expect("item count must be a positive integer");
                assert!(n > 0, "item count must be a positive integer");
                items = Some(n);
            }
        }
    }
    assert!(
        !(ladder && items.is_some()),
        "pass an item count or --ladder, not both"
    );
    let counts = if ladder {
        LADDER.to_vec()
    } else {
        vec![items.unwrap_or(DEFAULT_ITEMS)]
    };
    (counts, reps)
}

/// One timed pass, and the deterministic fields the sketch reported after it.
struct Pass {
    elapsed: Duration,
    fields: String,
}

fn fields(sketch: &ReqSketch<f64>) -> String {
    format!(
        " n={} min={:.0} max={:.0} retained={}",
        sketch.get_n(),
        sketch
            .get_min_item()
            .expect("benched sketches are non-empty"),
        sketch
            .get_max_item()
            .expect("benched sketches are non-empty"),
        sketch.get_num_retained()
    )
}

/// Lower median plus spread, as every harness prints; the per-rep fields are
/// asserted equal. Keep the format identical to `bench::report_fields`.
fn report(label: &str, items: u64, ops: u64, passes: &[Pass]) {
    for (i, pass) in passes.iter().enumerate() {
        assert_eq!(
            pass.fields, passes[0].fields,
            "rep {i} reported '{}' but rep 0 reported '{}': the reps are not running \
             the same workload",
            pass.fields, passes[0].fields
        );
    }
    let mut ns_per_op: Vec<f64> = passes
        .iter()
        .map(|p| p.elapsed.as_secs_f64() * 1e9 / ops as f64)
        .collect();
    ns_per_op.sort_by(f64::total_cmp);
    let median = ns_per_op[(ns_per_op.len() - 1) / 2];
    let (min, max) = (ns_per_op[0], ns_per_op[ns_per_op.len() - 1]);
    let rate = 1000.0 / median;
    let (reps, fields) = (passes.len(), &passes[0].fields);
    println!(
        "{label:9} {items:>12} items  {median:>7.2} ns/op  min {min:>7.2}  max {max:>7.2}  \
         {rate:>8.1} M/s  reps={reps}{fields}"
    );
}

fn build() -> ReqSketch<f64> {
    ReqSketch::new(K, RankAccuracy::High).expect("fixed valid k was rejected")
}

fn filled(items: u64) -> ReqSketch<f64> {
    let mut sketch = build();
    for i in 0..items {
        sketch.update(i as f64);
    }
    sketch
}

fn bench_distinct(items: u64, reps: usize) {
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let mut sketch = build();
            let start = Instant::now();
            for i in 0..items {
                sketch.update(i as f64);
            }
            let elapsed = start.elapsed();
            Pass {
                elapsed,
                fields: fields(&sketch),
            }
        })
        .collect();
    report("distinct", items, items, &passes);
}

fn bench_hot(items: u64, reps: usize) {
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let mut sketch = build();
            let start = Instant::now();
            for i in 0..items {
                sketch.update((i % HOT_KEY_SPACE) as f64);
            }
            let elapsed = start.elapsed();
            Pass {
                elapsed,
                fields: fields(&sketch),
            }
        })
        .collect();
    report("hot", items, items, &passes);
}

fn bench_quantile(items: u64, reps: usize) {
    let sketch = filled(items);
    // Builds the sketch's cached sorted view outside the timed loop.
    black_box(sketch.get_quantile(0.5, true).unwrap());
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut total = 0.0;
            for i in 0..QUERY_CALLS {
                total += sketch
                    .get_quantile((i % 1000) as f64 / 1000.0, true)
                    .unwrap();
            }
            let elapsed = start.elapsed();
            black_box(total);
            Pass {
                elapsed,
                fields: fields(&sketch),
            }
        })
        .collect();
    report("quantile", items, QUERY_CALLS, &passes);
}

fn bench_serde(items: u64, reps: usize) {
    let sketch = filled(items);
    let reference = sketch.serialize();
    let with_bytes = |s: &ReqSketch<f64>| format!("{} bytes={}", fields(s), reference.len());

    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut total = 0usize;
            for _ in 0..SER_CALLS {
                total += black_box(sketch.serialize()).len();
            }
            let elapsed = start.elapsed();
            black_box(total);
            Pass {
                elapsed,
                fields: with_bytes(&sketch),
            }
        })
        .collect();
    report("ser", items, SER_CALLS, &passes);

    let deserialize =
        || ReqSketch::<f64>::deserialize(&reference).expect("the bytes came from serialize");
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut total = 0u64;
            for _ in 0..DESER_CALLS {
                total += u64::from(deserialize().get_num_retained());
            }
            let elapsed = start.elapsed();
            black_box(total);
            Pass {
                elapsed,
                fields: with_bytes(&deserialize()),
            }
        })
        .collect();
    report("deser", items, DESER_CALLS, &passes);
}

fn main() {
    let (counts, reps) = parse_args();
    for items in counts {
        println!("k={K} items={items} reps={reps}");
        bench_distinct(items, reps);
        bench_hot(items, reps);
        bench_quantile(items, reps);
        bench_serde(items, reps);
    }
}
