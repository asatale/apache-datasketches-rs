//! Throughput harness for `HllUnion`'s direct-update path.
//!
//! Run with (release matters — a debug build measures nothing useful):
//!   cargo run --release --example bench_hll_union_update --features hll
//!   cargo run --release --example bench_hll_union_update --features hll -- 100000000
//!   cargo run --release --example bench_hll_union_update --features hll -- --ladder
//!
//! Accepts `[ITEMS] [--reps N] [--ladder]`. Every figure printed is the lower
//! median of `--reps` passes (default 3), with the spread alongside it, so a
//! single noisy pass cannot become a published number. `--ladder` sweeps a
//! range of item counts instead of one, because a family's per-update cost is
//! not constant as the sketch fills.
//!
//! Pair with the native C++ counterpart on the same item count to get the
//! binding's overhead, which is the number worth tracking:
//!
//!   ./benches/cpp_reference/run.sh 10000000
//!
//! Fixed parameters so runs are comparable: `lg_max_k = 12` (matching
//! `bench_hll_update.rs`'s `lg_config_k`), result type `Hll8`. Keep them in
//! sync with `benches/cpp_reference/hll_union_update.cc`; both print the
//! estimate, and the estimates must match.
//!
//! This harness exercises `HllUnion::update_*` directly (feeding items to the
//! union itself, as if every merged-in sketch received them), not
//! `update_sketch`: that is the path the CHANGELOG's open caveat named —
//! `update_str`'s no-alloc guard applies to `HllUnionShim::update_str` too,
//! and it had no dedicated bench on either side.
//!
//! Four scenarios: `distinct`/`hot`/`str`, matching `bench_hll_update.rs`,
//! plus `ser`. There is no `deser` scenario here: a union has no serializable
//! state of its own upstream (only its result sketch does, per
//! `HllUnion::serialize_compact`'s doc comment) and so nothing to deserialize
//! *into* a union — `HllUnion` exposes no `deserialize`. `ser` is kept,
//! measuring `get_result(Hll8).serialize_compact()` together: that pair *is*
//! a real cost a caller pays to get bytes out of a union, even though it is
//! not a cost unique to the union (the `get_result` half is the same
//! `hll_sketch::serialize_compact` that `bench_hll_update.rs::bench_serde`
//! already measures on its own).

use apache_datasketches::hll::{HllUnion, TargetHllType};
use std::hint::black_box;
use std::time::{Duration, Instant};

/// Matches `bench_hll_update.rs`'s `LG_K`, used there as `lg_config_k` and
/// here as `lg_max_k`, so the two harnesses are comparable.
const LG_K: u8 = 12;

/// Result type read out of the union via `get_result`. Matches
/// `bench_hll_update.rs`'s `Hll8`.
const RESULT_TYPE: TargetHllType = TargetHllType::Hll8;

/// The `ser` scenario is the one place where the item count is not the
/// divisor: serialization cost tracks the serialized *size*, and at this
/// harness's `lg_k` the sketch saturates well below the ladder's bottom rung,
/// so the same buffer is produced at 1M items as at 100M. The printed `ns/op`
/// is therefore per call, over a call count fixed here rather than taken from
/// the command line -- otherwise the number would silently mean something
/// different at each rung.
///
/// Keep in step with `bench_common.h`.
const SER_CALLS: u64 = 20_000;

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

/// Item counts for `--ladder`, which exists because a single item count hides
/// the shape: a family's per-update cost is not constant as the sketch fills.
///
/// Starts at 1M rather than lower. Below that the cheap families are still in
/// a warm-up regime -- HLL's coupon list -- so the printed ns/op would be an
/// average taken across a regime change rather than a steady-state cost,
/// which is precisely the kind of number the ladder exists to stop people
/// quoting.
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
/// running the same thing -- most likely a union reused across reps instead
/// of rebuilt, which would quietly lower the ns/op of every rep after the
/// first.
///
/// `ns/op`, `reps` and `estimate` are printed as labelled values rather than
/// as bare numbers in fixed columns, so that reading them back does not mean
/// counting awk fields that shift whenever a column is added.
fn report(label: &str, items: u64, passes: &[Pass]) {
    report_line(label, items, items, passes, String::new());
}

/// As [`report`], plus the serialized size, and dividing by an explicit `ops`
/// rather than by the item count.
///
/// The size is worth printing for its own sake -- it is the quantity a `ser`
/// `ns/op` is proportional to, so without it the timing cannot be interpreted
/// -- but it is also a check the estimate cannot make. Two sides can agree
/// exactly on the estimate while one compacts ordered and the other does not,
/// or serializes a different format; the byte count differs the moment they
/// do.
fn report_bytes(label: &str, items: u64, ops: u64, passes: &[Pass], bytes: usize) {
    report_line(label, items, ops, passes, format!(" bytes={bytes}"));
}

fn report_line(label: &str, items: u64, ops: u64, passes: &[Pass], suffix: String) {
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
        .map(|p| p.elapsed.as_secs_f64() * 1e9 / ops as f64)
        .collect();
    ns_per_op.sort_by(f64::total_cmp);
    let median = ns_per_op[(ns_per_op.len() - 1) / 2];
    let (min, max) = (ns_per_op[0], ns_per_op[ns_per_op.len() - 1]);
    let rate = 1000.0 / median;
    let (reps, estimate) = (passes.len(), passes[0].estimate);
    println!(
        "{label:9} {items:>12} items  {median:>7.2} ns/op  min {min:>7.2}  max {max:>7.2}  \
         {rate:>8.1} M/s  reps={reps} estimate={estimate:.0}{suffix}"
    );
}

/// One timed pass over `items` updates, and the estimate the union held
/// afterwards. Reading the estimate also keeps the update loop from being
/// optimised out.
struct Pass {
    elapsed: Duration,
    estimate: f64,
}

fn build() -> HllUnion {
    HllUnion::new(LG_K).expect("fixed valid parameters were rejected")
}

/// Each rep rebuilds the union: a reused one would already be full, so every
/// rep after the first would measure a different workload. `report` asserts
/// the per-rep estimates agree, which is what catches that if it regresses.
fn bench_distinct(items: u64, reps: usize) {
    let mut passes = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut union = build();
        let start = Instant::now();
        for key in 0..items {
            union.update_u64(key);
        }
        let elapsed = start.elapsed();
        passes.push(Pass {
            elapsed,
            estimate: union.get_estimate(),
        });
    }
    report("distinct", items, &passes);
}

fn bench_hot(items: u64, reps: usize) {
    let mut passes = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut union = build();
        let start = Instant::now();
        for i in 0..items {
            union.update_u64(i % HOT_KEY_SPACE);
        }
        let elapsed = start.elapsed();
        passes.push(Pass {
            elapsed,
            estimate: union.get_estimate(),
        });
    }
    report("hot", items, &passes);
}

fn bench_str(items: u64, reps: usize) {
    let keys = string_keys();
    let mut passes = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut union = build();
        let start = Instant::now();
        for i in 0..items {
            union.update_str(&keys[(i % STR_KEY_SPACE) as usize]);
        }
        let elapsed = start.elapsed();
        passes.push(Pass {
            elapsed,
            estimate: union.get_estimate(),
        });
    }
    report("str", items, &passes);
}

/// Serialization, measured per call rather than per item: its cost tracks the
/// serialized size, which at `lg_k = 12` is the same at every ladder rung.
///
/// The union is built once and shared by every rep. Serializing does not
/// mutate it, so unlike the update scenarios there is no state that a second
/// rep would find already dirtied -- and rebuilding at the 100M rung would
/// cost more than the measurement itself.
///
/// Unlike `bench_hll_update.rs::bench_serde`, there is no `deser` half: a
/// union has no `deserialize` (see this file's header comment), so this
/// measures only `get_result(RESULT_TYPE).serialize_compact()`, taken
/// together as the one round trip a caller of `HllUnion` can actually make.
fn bench_ser(items: u64, reps: usize) {
    let mut union = build();
    for key in 0..items {
        union.update_u64(key);
    }
    let reference = union.serialize_compact(RESULT_TYPE);

    let mut passes = Vec::with_capacity(reps);
    for _ in 0..reps {
        let start = Instant::now();
        let mut total = 0usize;
        for _ in 0..SER_CALLS {
            total += black_box(union.serialize_compact(RESULT_TYPE)).len();
        }
        let elapsed = start.elapsed();
        black_box(total);
        passes.push(Pass {
            elapsed,
            estimate: union.get_estimate(),
        });
    }
    report_bytes("ser", items, SER_CALLS, &passes, reference.len());
}

fn main() {
    let (counts, reps) = parse_args();
    for items in counts {
        println!("lg_max_k={LG_K} result_type=Hll8 items={items} reps={reps}");
        bench_distinct(items, reps);
        bench_hot(items, reps);
        bench_str(items, reps);
        bench_ser(items, reps);
    }
}
