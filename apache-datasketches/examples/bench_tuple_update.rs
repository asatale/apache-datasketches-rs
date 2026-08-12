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
//!   cargo run --release --example bench_tuple_update --features tuple -- --ladder
//!
//! Accepts `[ITEMS] [--reps N] [--ladder]`. Every figure printed is the lower
//! median of `--reps` passes (default 3), with the spread alongside it, so a
//! single noisy pass cannot become a published number. `--ladder` sweeps a
//! range of item counts instead of one, because a family's per-update cost is
//! not constant as the sketch fills.
//!
//! Fixed parameters, so numbers are comparable across runs: `lg_k = 12`,
//! `num_values = 3`, `resize_factor` and `p` at their defaults. The item
//! count defaults to 10M.
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

use apache_datasketches::tuple::{ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder};
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

/// Item counts for `--ladder`, which exists because a single item count hides
/// the shape: a family's per-update cost is not constant as the sketch fills.
///
/// Starts at 1M rather than lower. Below that the cheap families are still in
/// a warm-up regime -- HLL's coupon list, CPC's flavour transitions -- so the
/// printed ns/op would be an average taken across a regime change rather than
/// a steady-state cost, which is precisely the kind of number the ladder
/// exists to stop people quoting.
const LADDER: [u64; 3] = [1_000_000, 10_000_000, 100_000_000];
const DEFAULT_ITEMS: u64 = 10_000_000;
const DEFAULT_REPS: usize = 3;

/// Parses `[ITEMS] [--reps N] [--ladder]`. Hand-rolled: three flags do not
/// justify pulling an argument crate into a bench example.
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
    // Rejected rather than resolved by precedence: silently ignoring an
    // explicit item count would make a mis-typed invocation look like it
    // measured what was asked for.
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

/// Prints the lower median of the passes plus the spread, so a published
/// figure is never a single noisy point -- the AGENTS.md rule that a
/// performance claim rest on a median of at least three runs is enforced here
/// rather than left to whoever happens to be running it.
///
/// Lower median (`sorted[(n - 1) / 2]`), not the average of the two middle
/// values: every number printed is then one that an actual pass produced. At
/// the default `reps = 3` the two definitions agree; this only matters for an
/// even `--reps`.
///
/// The estimates are asserted equal across reps rather than merely reported.
/// These workloads are deterministic, so a disagreement means the reps are not
/// running the same thing -- most likely a sketch reused across reps instead
/// of rebuilt, which would quietly lower the ns/op of every rep after the
/// first.
///
/// `ns/op`, `reps` and `estimate` are printed as labelled values rather than
/// as bare numbers in fixed columns, so that reading them back does not mean
/// counting awk fields that shift whenever a column is added.
fn report(label: &str, items: u64, passes: &[Pass]) {
    for (i, pass) in passes.iter().enumerate() {
        assert_eq!(
            pass.estimate, passes[0].estimate,
            "rep {i} estimated {} but rep 0 estimated {}: the reps are not running \
             the same workload",
            pass.estimate, passes[0].estimate
        );
    }
    let mut ns_per_op: Vec<f64> = passes
        .iter()
        .map(|p| p.elapsed.as_secs_f64() * 1e9 / items as f64)
        .collect();
    ns_per_op.sort_by(f64::total_cmp);
    let median = ns_per_op[(ns_per_op.len() - 1) / 2];
    let (min, max) = (ns_per_op[0], ns_per_op[ns_per_op.len() - 1]);
    let rate = 1000.0 / median;
    let (reps, estimate) = (passes.len(), passes[0].estimate);
    println!(
        "{label:9} {items:>12} items  {median:>7.2} ns/op  min {min:>7.2}  max {max:>7.2}  \
         {rate:>8.1} M/s  reps={reps} estimate={estimate:.0}"
    );
}

/// One timed pass over `items` updates, and the estimate the sketch held
/// afterwards. Reading the estimate also keeps the update loop from being
/// optimised out.
struct Pass {
    elapsed: Duration,
    estimate: f64,
}

fn build() -> ArrayOfDoublesSketch {
    ArrayOfDoublesSketchBuilder::new()
        .lg_k(LG_K)
        .num_values(NUM_VALUES)
        .build()
        .expect("builder rejected fixed valid parameters")
}

/// Each rep rebuilds the sketch: a reused one would already be full, so every
/// rep after the first would measure a different workload. `report` asserts
/// the per-rep estimates agree, which is what catches that if it regresses.
fn bench_distinct(items: u64, reps: usize) {
    let mut passes = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut sketch = build();
        let start = Instant::now();
        for key in 0..items {
            sketch
                .update_u64(key, &VALUES)
                .expect("update rejected a correctly-sized value slice");
        }
        let elapsed = start.elapsed();
        passes.push(Pass {
            elapsed,
            estimate: sketch.get_estimate(),
        });
    }
    report("distinct", items, &passes);
}

fn bench_hot(items: u64, reps: usize) {
    let mut passes = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut sketch = build();
        let start = Instant::now();
        for i in 0..items {
            sketch
                .update_u64(i % HOT_KEY_SPACE, &VALUES)
                .expect("update rejected a correctly-sized value slice");
        }
        let elapsed = start.elapsed();
        passes.push(Pass {
            elapsed,
            estimate: sketch.get_estimate(),
        });
    }
    report("hot", items, &passes);
}

fn bench_str(items: u64, reps: usize) {
    let keys = string_keys();
    let mut passes = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut sketch = build();
        let start = Instant::now();
        for i in 0..items {
            sketch
                .update_str(&keys[(i % STR_KEY_SPACE) as usize], &VALUES)
                .expect("update rejected a correctly-sized value slice");
        }
        let elapsed = start.elapsed();
        passes.push(Pass {
            elapsed,
            estimate: sketch.get_estimate(),
        });
    }
    report("str", items, &passes);
}

fn main() {
    let (counts, reps) = parse_args();
    for items in counts {
        println!("lg_k={LG_K} num_values={NUM_VALUES} items={items} reps={reps}");
        bench_distinct(items, reps);
        bench_hot(items, reps);
        bench_str(items, reps);
    }
}
