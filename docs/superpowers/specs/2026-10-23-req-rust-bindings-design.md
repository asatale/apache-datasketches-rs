# Apache DataSketches REQ Rust Bindings — Design

## Purpose

Add the REQ (Relative Error Quantiles) sketch family to
`apache-datasketches`/`apache-datasketches-sys`. REQ is the first of four
families planned after KLL (REQ, then Bloom filter, Count-Min and t-digest,
each with its own spec → plan → implementation cycle).

REQ answers the same questions as KLL — rank, quantile, PMF, CDF — but its
error is *relative* to the rank rather than a fixed absolute amount, so it
stays accurate at one end of the distribution (p99, p99.9, … by default).

This design is a delta against
[the KLL design](2026-10-21-kll-rust-bindings-design.md): REQ deliberately
mirrors KLL's architecture, and anything not mentioned here — the sealed
item-trait pattern, the C++ class template with concrete aliases, the
error-before-FFI validation style, the randomized-compaction testing rules,
the bench-pair field cross-check — carries over unchanged. Where KLL's design
was corrected during implementation (layout validation on deserialize, the
shim-side rank check, the doc structure from the alignment pass), this design
starts from the corrected state.

**Scope:** `req_sketch<T>` for `T = double` and `T = float`, and its sorted
view.

**Out of scope:** other item types (Python's `req_ints_sketch`, strings,
user types); stream serialization; `header_size_bytes`; cross-type
conversion; a shared `quantiles` module with KLL (see "Relationship to KLL").

## Upstream facts this design rests on

Checked against the vendored `req/include/` (datasketches-cpp 5.2.0):

- `req_sketch(uint16_t k, bool hra = true)`. The doc says `k` "must be even
  and in the range [4, 1024]", but the constructor silently rounds an odd `k`
  down and clamps below 4, and never checks the upper bound.
- `hra` (high rank accuracy, default `true`) concentrates accuracy at high
  ranks; `false` (LRA) at low ranks. `merge` throws `std::invalid_argument`
  when the two sketches' modes differ, and adds `n_ += other.n_` unchecked.
- Rank error is rank-dependent: `get_rank_lower_bound(rank, num_std_dev)`,
  `get_rank_upper_bound(rank, num_std_dev)` (documented "must be 1, 2, or 3"
  but unchecked), and static `get_RSE(k, rank, hra, n)`. There is no
  normalized rank error, so upstream's `kolmogorov_smirnov` does not support
  REQ.
- Queries go through the same `quantiles_sorted_view<T>` type as KLL, cached
  in a `mutable` member by `setup_sorted_view()` — so the same `!Sync`
  reasoning applies.
- Compaction flips `random_utils::random_bit()` — the same unseedable,
  clock-seeded engine as KLL.
- `deserialize` trusts its input. Among other things, a single-level sketch
  flagged non-empty but holding zero items makes it dereference an empty
  range (`*min_it` on `begin == end`) — undefined behaviour from crafted
  bytes.
- Java's `ReqSketch` is `float`-only; Python has float and int variants. So
  `f32` bytes interoperate with Java and Python; `f64` bytes are C++-only.
- Upstream's `req/test/` has no checked-in `.sk` fixtures.

## Architecture

### Feature and vendoring

- New `req` feature on both crates; the safe crate adds it to `default`, to
  `[package.metadata.docs.rs]`, and to the no-family `compile_error!` guard
  (keeping the phrase `no sketch family is enabled`).
- Vendor `req/include/` into `apache-datasketches-sys/vendor/datasketches-cpp`
  and update the three places that list vendored modules: the copy commands
  in `apache-datasketches-sys/vendor/README.md`, `build.rs`'s `.include(...)`
  calls, and `MODULES` in `tests/vendor_drift_test.rs`.

### C++ shim

`apache-datasketches-sys/cpp/req/req_sketch_shim.{h,cc}`:

- `template<typename T> class ReqSketchShim` wraps `req_sketch<T>`;
  `template<typename T> class ReqSortedViewShim` wraps
  `quantiles_sorted_view<T>` (a deliberate copy of KLL's view shim — see
  "Relationship to KLL").
- Concrete aliases `ReqDoublesSketchShim`, `ReqFloatsSketchShim`,
  `ReqDoublesSortedViewShim`, `ReqFloatsSortedViewShim`; explicit
  instantiation of both `T`s in the `.cc`.
- Both `get_quantile` paths reject a NaN or out-of-range rank with
  `std::invalid_argument` before upstream sees it (as KLL's shim does).
- `validate_req_layout(bytes)` runs before upstream `deserialize` in both
  `req_*_deserialize` functions; see "Hardening".

### Bridges

`apache-datasketches-sys/src/req_doubles.rs` and `req_floats.rs`, identical
modulo `f64`↔`f32` and `Doubles`↔`Floats` (ranks, `p`-like values, RSE and
PMF/CDF masses stay `f64` in both). Free functions carry the prefix:
`req_doubles_new(k: u16, hra: bool)`, `req_doubles_deserialize(&[u8])`,
`req_doubles_rse(k: u16, rank: f64, hra: bool, n: u64) -> f64`, and the
`req_floats_` equivalents. Everything else is a method (no prefix needed).
Calls that can throw on user or untrusted input return `Result`.

### Safe layer

`apache-datasketches/src/req/`:

| File | Contents |
|---|---|
| `mod.rs` | module docs, re-exports |
| `checks.rs` | argument validation (k, rank, num_std_dev, NaN, split points) |
| `item.rs` | sealed `ReqItem` trait, `impl_sealed!` for `f64` and `f32` |
| `sketch.rs` | `ReqSketch<T>`, `RankAccuracy` |
| `sorted_view.rs` | `ReqSortedView<T>`, `ReqSortedViewEntry<T>` |

No Kolmogorov–Smirnov module (unsupported upstream for REQ).

### Relationship to KLL

REQ mirrors KLL with separate types rather than sharing a `quantiles` layer,
even though both wrap the same upstream `quantiles_sorted_view<T>`. This
keeps the `kll` and `req` features fully independent (no shared bridge, no
`cfg(any(...))` wiring) and leaves merged KLL code untouched. The cost is a
small, mechanical duplication: the view shim template and `sorted_view.rs`.
If a third quantiles family (classic Quantiles) is ever bound, extracting a
shared layer then is a contained refactor.

The entry type is named `ReqSortedViewEntry` (not `SortedViewEntry`) so it
can be imported alongside KLL's `SortedViewEntry` without a clash.

## Public API

```rust
/// Which end of the rank domain gets REQ's relative-error guarantee.
pub enum RankAccuracy { High, Low }

pub trait ReqItem: sealed::Sealed + Copy + PartialOrd + Debug + Send + Sync + 'static {}
// implemented for f64 and f32 only

pub struct ReqSketch<T: ReqItem>;           // Clone + Send, NOT Sync

impl<T: ReqItem> ReqSketch<T> {
    pub fn new(k: u16, accuracy: RankAccuracy) -> Result<Self, SketchError>;
    pub fn update(&mut self, item: T);
    pub fn merge(&mut self, other: &ReqSketch<T>) -> Result<(), SketchError>;

    pub fn is_empty(&self) -> bool;
    pub fn get_n(&self) -> u64;
    pub fn get_k(&self) -> u16;
    pub fn get_num_retained(&self) -> u32;
    pub fn is_estimation_mode(&self) -> bool;
    pub fn rank_accuracy(&self) -> RankAccuracy;
    pub fn get_min_item(&self) -> Option<T>;
    pub fn get_max_item(&self) -> Option<T>;

    pub fn get_quantile(&self, rank: f64, inclusive: bool) -> Result<T, SketchError>;
    pub fn get_rank(&self, item: T, inclusive: bool) -> Result<f64, SketchError>;
    pub fn get_pmf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError>;
    pub fn get_cdf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError>;

    pub fn get_rank_lower_bound(&self, rank: f64, num_std_dev: u8) -> Result<f64, SketchError>;
    pub fn get_rank_upper_bound(&self, rank: f64, num_std_dev: u8) -> Result<f64, SketchError>;
    pub fn relative_standard_error(k: u16, rank: f64, accuracy: RankAccuracy, n: u64)
        -> Result<f64, SketchError>;

    pub fn serialize(&self) -> Vec<u8>;
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError>;
    pub fn to_string_summary(&self) -> String;

    pub fn iter(&self) -> impl Iterator<Item = (T, u64)>;   // (item, weight)
    pub fn sorted_view(&self) -> ReqSortedView<T>;
}

impl<T: ReqItem> Default for ReqSketch<T>;  // k = 12, RankAccuracy::High

pub struct ReqSortedView<T: ReqItem>;       // owned, no lifetime; Send + Sync

impl<T: ReqItem> ReqSortedView<T> {
    pub fn get_quantile(&self, rank: f64, inclusive: bool) -> Result<T, SketchError>;
    pub fn get_rank(&self, item: T, inclusive: bool) -> Result<f64, SketchError>;
    pub fn get_pmf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError>;
    pub fn get_cdf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = ReqSortedViewEntry<T>>;
}

pub struct ReqSortedViewEntry<T> {
    pub item: T,
    pub weight: u64,
    pub cumulative_weight: u64,   // inclusive of this entry
}
```

Notes:

- **`k`** is validated in Rust: even and in `4..=1024`, else
  `InvalidConfig` — upstream's documented contract, rather than its silent
  rounding. **`Default`** uses `k = 12` (Java's and Python's default; about
  1% relative error at 95% confidence) and `RankAccuracy::High` (upstream's
  `hra = true` default).
- **`RankAccuracy`** is a named enum rather than upstream's `bool`, for the
  same readability reason HLL has `TargetHllType`. It derives
  `Debug, Clone, Copy, PartialEq, Eq` and maps to/from the bridge `bool` in
  `item.rs` or `sketch.rs`.
- **`merge`** returns `Result`, unlike KLL's: mixing accuracy modes is an
  ordinary, recoverable caller mistake. Both failure cases are checked in
  Rust before crossing FFI — differing `RankAccuracy`, and `get_n() +
  other.get_n()` overflowing `u64` — and return `InvalidConfig`. If the
  hardening task finds a lower safe limit on the combined `n` (as KLL's
  `2^63`), that limit is enforced here too, still as an `Err`.
- **`get_rank_lower_bound`/`get_rank_upper_bound`** validate `rank` in
  `[0, 1]` and `num_std_dev` in `1..=3`, and return `EmptySketch` on an empty
  sketch. **`relative_standard_error`** validates `k` and `rank`.
- **NaN** updates are silently ignored (upstream `check_update_item`).
- **`iter()`** and **`sorted_view()`** behave exactly as KLL's: copy-out,
  internal order vs sorted, the view is a snapshot.

### Thread safety

`ReqSketch` is `Send` but not `Sync` (upstream caches a sorted view in a
`mutable` member on query; a `compile_fail` doctest pins `!Sync`).
`ReqSortedView` is `Send + Sync`: upstream `quantiles_sorted_view` has no
mutable state and stores arithmetic items by value, so the view owns its
data and outlives the sketch safely.

## Errors

| Condition | Error |
|---|---|
| `k` odd or outside `4..=1024` | `InvalidConfig` |
| `rank` NaN or outside `[0, 1]` | `InvalidConfig` |
| `num_std_dev` outside `1..=3` | `InvalidConfig` |
| NaN item passed to `get_rank` | `InvalidConfig` |
| NaN in `split_points`, or not strictly increasing | `InvalidConfig` |
| `merge` with differing `RankAccuracy` | `InvalidConfig` |
| `merge` whose combined `n` would overflow (or exceed a found safe limit) | `InvalidConfig` |
| query or rank bound on an empty sketch or empty view | `EmptySketch` |
| bytes that do not parse, or that `validate_req_layout` rejects | `Deserialization` |
| any other C++ exception | `Cpp` |

Empty bytes carry no item type (as with KLL), so they deserialize as either
`f32` or `f64`; non-empty bytes of the other item type must be rejected.

## Hardening

Upstream `deserialize` trusts its input, and KLL's review showed crafted
bytes can reach aborts, hangs and heap corruption in later operations. For
REQ this work is planned up front, as its own task, rather than discovered in
review:

1. Derive `validate_req_layout` from upstream `req_sketch::deserialize` and
   `req_compactor::deserialize`: a bounds-checked parse of the header and
   every compactor before upstream reads anything. It rejects at minimum: a
   non-empty sketch with zero items; level and per-compactor item counts that
   do not fit the buffer; `n` inconsistent with the compactors' weights;
   `k` odd or outside `4..=1024`; unknown flag combinations upstream would
   misread; NaN items (checked after decode if simpler, as KLL does).
2. Audit upstream's other paths that crafted-but-consistent bytes can reach
   — compactor `merge`, `grow`, `compress`, sorted-view construction — and add
   a check for any abort or UB found, each with a regression test.
3. **A committed fuzz regression test** (`req_deserialize_fuzz_test.rs`):
   deterministic, fixed-seed mutation of genuine serialized sketches (both
   item types, raw-items and multi-level shapes), asserting each mutation
   either returns `Err` from `deserialize` or yields a sketch that survives
   `merge` in both directions, queries, `sorted_view`, `iter` and a
   re-serialize round trip without aborting. Iteration count bounded so the
   test stays fast in CI.

## Randomness

As KLL: compaction is randomized and unseedable. Tests assert exact
quantiles/ranks only while no compaction has happened (`is_estimation_mode()
== false`), and within upstream's error bounds otherwise. `n`, `k`, min, max
and serialized size for a given input are deterministic. Whether
`num_retained` is deterministic for a given input — upstream's section sizing
suggests it is — must be confirmed before the bench pair relies on it; if it
is not, the bench cross-checks only the fields that are.

## Testing

All in `apache-datasketches/tests/` unless noted; each case runs for both
`f64` and `f32` through a shared generic helper (`tests/req_common/mod.rs`,
mirroring `kll_common`).

- **`req_sketch_smoke_test.rs`** — happy paths.
- **`req_sketch_test.rs`** — port of upstream `req/test/req_sketch_test.cpp`
  (`TEST_CASE`s: empty, single value LRA, repeated values, exact mode,
  estimation mode, byte serialize/deserialize for empty / single item /
  exact / estimation, merge into empty, merge, merge multiple, merge
  incompatible HRA and LRA, get_rank equivalence). Stream-serialization,
  type-conversion and custom-type cases are skipped, each named in the file
  header with the reason.
- **`req_validation_test.rs`** — every row of the error table.
- **`req_serialization_compat_test.rs`** — round trips at each size class;
  golden bytes derived by hand from upstream `serialize` (never from the
  binding's output) for empty, raw-items (`n <= 4`) and a multi-level sketch
  in exact mode; cross-type rejection for non-empty bytes; empty bytes
  accepted as either type. With no upstream `.sk` fixtures, the Java/Python
  claim for `f32` rests on upstream's guarantee that C++ and Java bytes are
  identical plus these goldens.
- **`req_sorted_view_test.rs`** — view answers equal the sketch's; entries
  sorted with cumulative weights summing to `n`; empty view; snapshot
  semantics.
- **`req_deserialize_fuzz_test.rs`** — see "Hardening".
- **`req_concurrency_test.rs`** — sketch moved across threads; per-thread
  sketches merged; a `ReqSortedView` shared through `Arc`.
- **`apache-datasketches-sys/tests/req_{doubles,floats}_link_test.rs`** —
  link tests, including the shim's rank rejection on both `get_quantile`
  paths.

## Benchmarks

`apache-datasketches/examples/bench_req_update.rs` and
`benches/cpp_reference/req_update.cc`: `f64`, `k = 12`, high rank accuracy;
scenarios `distinct`, `hot`, `quantile`, `ser`, `deser`; same
`--reps`/`--ladder` handling. They print the deterministic fields
(`n`/`min`/`max`/`retained`, plus `bytes` on `ser`/`deser`) via
`bench::report_fields` — subject to the determinism check in "Randomness".
Registered in `run.sh` and `compare.sh`; documented in
`benches/cpp_reference/README.md` next to the KLL section, including that the
Rust `deser` pays for layout validation the C++ side does not.

## CI

Add `--no-default-features --features req` to the per-family feature matrix
and update the default-leg comment's family count. The `benches` job picks up
the new pair through `compare.sh`.

## Documentation

Shaped like the KLL docs after their alignment pass:

- `apache-datasketches/README.md`: a "REQ sketches" section with a snippet,
  a per-type list with key methods, a "what differs from the other families"
  list (relative error and `RankAccuracy`, fallible `merge`, `!Sync`,
  randomized compaction, `f32`-only Java/Python compatibility), and the
  example's coverage; plus the families list and family count.
- `apache-datasketches/src/lib.rs` crate docs and `req/mod.rs` module docs
  with a doctest example.
- `apache-datasketches-sys/README.md`: `req` bullet and toml snippet.
- Root `README.md`: families list, example command, family count.
- `AGENTS.md`: family list and count, commands (test, example, bench), the
  vendored-modules list, and a short REQ paragraph next to KLL's.
- `apache-datasketches/examples/req.rs`: runnable demo.
- `CHANGELOG.md`: an entry under `[Unreleased]`.

## Release

REQ is additive. It goes under `[Unreleased]`: if `apache-datasketches`
0.3.0 is still unpublished when REQ lands, it ships in 0.3.0 alongside KLL;
otherwise it becomes 0.3.1 (and sys 0.2.4). Versions are bumped and the
crates published only with the maintainer's go-ahead, then tagged per the
repo's release rule.
