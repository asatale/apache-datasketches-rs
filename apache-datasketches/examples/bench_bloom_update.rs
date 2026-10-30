//! Throughput harness for `BloomFilter`: update, query, union and serialization.
//!
//! Run with (release matters -- a debug build measures nothing useful):
//!   cargo run --release --example bench_bloom_update --features bloom
//!   cargo run --release --example bench_bloom_update --features bloom -- --ladder
//!
//! Accepts `[ITEMS] [--reps N] [--ladder]`, like every other harness. Pair
//! with `benches/cpp_reference/bloom_update.cc` via `compare.sh`.
//!
//! Fixed parameters so runs are comparable: `max_distinct_items = 1_000_000`,
//! `target_fpp = 0.01`, `seed = 9001`. Keep them in sync with the C++
//! counterpart.
//!
//! A Bloom filter has no estimate, but it is deterministic, so this harness
//! cross-checks exact fields instead: `bits_used` after each scenario,
//! `hits` for `query`, and `bytes` for `ser`/`deser`. They must match across
//! reps and across languages.

use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder};
use std::hint::black_box;
use std::time::{Duration, Instant};

const MAX_DISTINCT_ITEMS: u64 = 1_000_000;
const TARGET_FPP: f64 = 0.01;
const SEED: u64 = 9001;

/// Size of the pre-built string-key pool. See `string_keys`.
const STR_KEY_SPACE: u64 = 1 << 16;

/// Keep in step with `bench_common.h`; see the comments there.
const SER_CALLS: u64 = 20_000;
const DESER_CALLS: u64 = 5_000;
const OP_CALLS: u64 = 5_000;

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

/// Built once, outside every timed region: formatting a key costs more than
/// the update does. Keep the format identical to `bench::string_keys`.
fn string_keys() -> Vec<String> {
    (0..STR_KEY_SPACE).map(|i| format!("key_{i:010}")).collect()
}

/// One timed pass, and the deterministic fields the filter reported after it.
struct Pass {
    elapsed: Duration,
    fields: String,
}

fn fields(filter: &mut BloomFilter) -> String {
    format!(" bits_used={}", filter.bits_used())
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

fn build() -> BloomFilter {
    BloomFilterBuilder::by_accuracy(MAX_DISTINCT_ITEMS, TARGET_FPP)
        .seed(SEED)
        .build()
        .expect("fixed valid parameters were rejected")
}

fn filled(begin: u64, end: u64) -> BloomFilter {
    let mut filter = build();
    for i in begin..end {
        filter.update_u64(i);
    }
    filter
}

fn bench_distinct(items: u64, reps: usize) {
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let mut filter = build();
            let start = Instant::now();
            for i in 0..items {
                filter.update_u64(i);
            }
            let elapsed = start.elapsed();
            Pass {
                elapsed,
                fields: fields(&mut filter),
            }
        })
        .collect();
    report("distinct", items, items, &passes);
}

/// Queries `0..items` (all hits) then `items..2 * items` (misses, bar false
/// positives). ns/op is per query.
fn bench_query(items: u64, reps: usize) {
    let mut filter = filled(0, items);
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut hits = 0u64;
            for i in 0..2 * items {
                hits += u64::from(filter.query_u64(i));
            }
            let elapsed = start.elapsed();
            black_box(hits);
            Pass {
                elapsed,
                fields: format!("{} hits={hits}", fields(&mut filter)),
            }
        })
        .collect();
    report("query", items, 2 * items, &passes);
}

fn bench_str(items: u64, reps: usize) {
    let keys = string_keys();
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let mut filter = build();
            let start = Instant::now();
            for i in 0..items {
                filter.update_str(&keys[(i % STR_KEY_SPACE) as usize]);
            }
            let elapsed = start.elapsed();
            Pass {
                elapsed,
                fields: fields(&mut filter),
            }
        })
        .collect();
    report("str", items, items, &passes);
}

/// Union cost tracks the bit-array size, not the item count, so this is a
/// fixed call count. Each call unions into a fresh copy: reusing one
/// accumulator would make every call after the first a no-op merge.
fn bench_union(items: u64, reps: usize) {
    let half = items / 2;
    let a = filled(0, half);
    let b = filled(half, items);
    let union_once = || {
        let mut u = a.clone();
        u.union_with(&b)
            .expect("same-parameter filters are compatible");
        u
    };
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..OP_CALLS {
                black_box(union_once().capacity());
            }
            let elapsed = start.elapsed();
            Pass {
                elapsed,
                fields: fields(&mut union_once()),
            }
        })
        .collect();
    report("union", items, OP_CALLS, &passes);
}

fn bench_serde(items: u64, reps: usize) {
    let mut filter = filled(0, items);
    let reference = filter.serialize();
    let bytes = reference.len();

    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut total = 0usize;
            for _ in 0..SER_CALLS {
                total += black_box(filter.serialize()).len();
            }
            let elapsed = start.elapsed();
            black_box(total);
            Pass {
                elapsed,
                fields: format!("{} bytes={bytes}", fields(&mut filter)),
            }
        })
        .collect();
    report("ser", items, SER_CALLS, &passes);

    // Includes the shim's up-front validation of the bytes, which the C++
    // side does not do.
    let deserialize =
        || BloomFilter::deserialize(&reference).expect("the bytes came from serialize");
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut total = 0u64;
            for _ in 0..DESER_CALLS {
                total += deserialize().capacity();
            }
            let elapsed = start.elapsed();
            black_box(total);
            Pass {
                elapsed,
                fields: format!("{} bytes={bytes}", fields(&mut deserialize())),
            }
        })
        .collect();
    report("deser", items, DESER_CALLS, &passes);
}

fn main() {
    let (counts, reps) = parse_args();
    for items in counts {
        println!(
            "max_distinct_items={MAX_DISTINCT_ITEMS} target_fpp={TARGET_FPP:.2} seed={SEED} \
             items={items} reps={reps}"
        );
        bench_distinct(items, reps);
        bench_query(items, reps);
        bench_str(items, reps);
        bench_union(items, reps);
        bench_serde(items, reps);
    }
}
