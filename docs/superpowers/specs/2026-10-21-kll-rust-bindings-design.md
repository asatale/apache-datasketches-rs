# Apache DataSketches KLL Rust Bindings — Design

## Purpose

Add the KLL quantiles sketch family to the existing
`apache-datasketches`/`apache-datasketches-sys` crates, following the pattern
established by [the HLL design](2026-07-26-hll-rust-bindings-design.md),
[the Theta design](2026-07-26-theta-rust-bindings-design.md) and
[the CPC design](2026-07-27-cpc-rust-bindings-design.md): a `cxx` shim over
the vendored `datasketches-cpp` 5.2.0 sources, a safe idiomatic wrapper, and
a port of the upstream C++ tests. This design covers what is different or new
about KLL; anything not mentioned here (FFI tool choice, sys/safe crate split,
submodule versioning, licensing) carries over unchanged.

KLL is the first *quantiles* family in this repo. Every existing family
estimates distinct counts; KLL answers rank, quantile, PMF and CDF queries
over a stream of numbers.

**Scope:** `kll_sketch<T>` for `T = double` and `T = float`, its sorted view
(`quantiles_sorted_view<T>`), and the Kolmogorov–Smirnov test
(`kolmogorov_smirnov`) over two KLL sketches. Upstream has no separate KLL
union class — merging is a method on the sketch.

**Out of scope:** item types other than `f64`/`f32` (integers, strings,
user-defined `Ord` types via Rust callbacks); the classic Quantiles, REQ and
t-digest families; stream-based (`std::istream`) serialization;
`header_size_bytes` on `serialize`.

## Why `f64` and `f32`

These are the two item types with a matching Java class
(`KllDoublesSketch`, `KllFloatsSketch`), so bytes serialized here are
readable by the Java and Python DataSketches libraries and vice versa. They
cover the overwhelming majority of real quantile workloads (latencies,
sizes, scores). Arbitrary item types would need every comparison to call back
into Rust across FFI and a serde callback — a different, much larger design,
deferred until there is demand.

## Architecture

### Feature and vendoring

- New `kll` feature on both crates. `apache-datasketches` adds it to its
  `default` set (`["hll", "theta", "cpc", "tuple", "kll"]`), to
  `[package.metadata.docs.rs]`, and to the `compile_error!` guard in
  `src/lib.rs` that fires when no family is enabled.
- `apache-datasketches-sys/vendor/datasketches-cpp` gains `kll/include/`.
  Three places must learn about it: the copy commands in
  `apache-datasketches-sys/vendor/README.md`, the `.include(...)` calls in
  `build.rs`, and `tests/vendor_drift_test.rs`.
  `quantiles_sorted_view.hpp` and `kolmogorov_smirnov.hpp` already live in the
  vendored `common/`.

### C++ shim — written once as a template

`apache-datasketches-sys/cpp/kll/kll_sketch_shim.{h,cc}`:

- `template<typename T> class kll_sketch_shim` wraps `kll_sketch<T>`.
- `template<typename T> class kll_sorted_view_shim` wraps
  `quantiles_sorted_view<T>`.
- cxx cannot name a template, so the header exposes concrete aliases —
  `KllDoublesSketchShim`, `KllFloatsSketchShim`, `KllDoublesSortedViewShim`,
  `KllFloatsSortedViewShim` — and the `.cc` explicitly instantiates both
  `T`s. The logic exists once; only the names differ per type.
- Only three operations are free functions — constructor, deserialize and
  the static rank error — thin per-type wrappers named per the global
  uniqueness rule below. Everything else, including clone and the KS
  functions, is a method on the shim, which needs no prefix because the
  receiver type is part of the symbol.
- `serialize` returns `UniquePtr<CxxVector<u8>>` built from upstream's
  `vector_bytes` in one bulk copy (see commit `7768222` for why not
  byte-by-byte).
- Bulk outputs cross as vectors: `get_PMF`/`get_CDF` return
  `UniquePtr<CxxVector<f64>>`; the sketch iterator and the sorted-view
  iterator each fill parallel item/weight vectors in a single call.

### Bridges — one per item type

`apache-datasketches-sys/src/kll_doubles.rs` and `kll_floats.rs`. Each
declares its sketch shim, its sorted-view shim and its KS functions. Sketch
and view must share a bridge because the sketch's `get_sorted_view` returns
the view type. Free-function names carry the type
(`kll_doubles_new`, `kll_doubles_deserialize`,
`kll_floats_normalized_rank_error`, …) so
the two bridges never collide — the `build.rs` duplicate-name check enforces
this. The two files are identical modulo `f64`↔`f32` and
`Doubles`↔`Floats`; keep them that way.

### Safe layer

`apache-datasketches/src/kll/`:

| File | Contents |
|---|---|
| `mod.rs` | module docs, re-exports |
| `item.rs` | sealed `KllItem` trait, impls for `f64` and `f32` |
| `sketch.rs` | `KllSketch<T>` |
| `sorted_view.rs` | `KllSortedView<T>`, `SortedViewEntry<T>` |
| `kolmogorov_smirnov.rs` | the three KS free functions |

`KllItem` is sealed via a private supertrait: users can name it in bounds but
cannot implement it. Its associated types (`SketchShim`, `ViewShim`) and
associated functions forward to the matching bridge, and are
`#[doc(hidden)]`. All generic code in `sketch.rs`, `sorted_view.rs` and
`kolmogorov_smirnov.rs` is written once against `T::…`; per-type dispatch
lives only in `item.rs`. Adding an item type later is one more trait impl
plus one more bridge.

## Public API

```rust
pub trait KllItem: Copy + PartialOrd + sealed::Sealed { /* hidden */ }
// implemented for f64 and f32 only

pub struct KllSketch<T: KllItem>;           // Clone + Send, NOT Sync

impl<T: KllItem> KllSketch<T> {
    pub fn new(k: u16) -> Result<Self, SketchError>;
    pub fn update(&mut self, item: T);
    pub fn merge(&mut self, other: &KllSketch<T>);

    pub fn is_empty(&self) -> bool;
    pub fn get_n(&self) -> u64;
    pub fn get_k(&self) -> u16;
    pub fn get_num_retained(&self) -> u32;
    pub fn is_estimation_mode(&self) -> bool;
    pub fn get_min_item(&self) -> Option<T>;
    pub fn get_max_item(&self) -> Option<T>;

    pub fn get_quantile(&self, rank: f64, inclusive: bool) -> Result<T, SketchError>;
    pub fn get_rank(&self, item: T, inclusive: bool) -> Result<f64, SketchError>;
    pub fn get_pmf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError>;
    pub fn get_cdf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError>;

    pub fn get_normalized_rank_error(&self, pmf: bool) -> f64;
    pub fn normalized_rank_error_for_k(k: u16, pmf: bool) -> Result<f64, SketchError>;

    pub fn serialize(&self) -> Vec<u8>;
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError>;
    pub fn to_string_summary(&self) -> String;

    pub fn iter(&self) -> impl Iterator<Item = (T, u64)>;   // (item, weight)
    pub fn sorted_view(&self) -> KllSortedView<T>;
}

impl<T: KllItem> Default for KllSketch<T>;  // k = 200

pub struct KllSortedView<T: KllItem>;       // owned, no lifetime; Send + Sync

impl<T: KllItem> KllSortedView<T> {
    pub fn get_quantile(&self, rank: f64, inclusive: bool) -> Result<T, SketchError>;
    pub fn get_rank(&self, item: T, inclusive: bool) -> Result<f64, SketchError>;
    pub fn get_pmf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError>;
    pub fn get_cdf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = SortedViewEntry<T>>;
}

pub struct SortedViewEntry<T> {
    pub item: T,
    pub weight: u64,
    pub cumulative_weight: u64,   // inclusive of this entry
}

pub fn kolmogorov_smirnov_delta<T: KllItem>(a: &KllSketch<T>, b: &KllSketch<T>)
    -> Result<f64, SketchError>;
pub fn kolmogorov_smirnov_threshold<T: KllItem>(a: &KllSketch<T>, b: &KllSketch<T>, p: f64)
    -> Result<f64, SketchError>;
pub fn kolmogorov_smirnov_test<T: KllItem>(a: &KllSketch<T>, b: &KllSketch<T>, p: f64)
    -> Result<bool, SketchError>;
```

Notes:

- **Construction** is `new(k)` plus `Default`, not a builder: KLL has one
  parameter, so it follows `HllSketch::new` rather than the CPC/Theta
  builders. `k` is validated to `8..=65535` (upstream `kll_constants::MIN_K`
  / `MAX_K`) in Rust; `DEFAULT_K` is 200.
- **NaN on update** is silently ignored, as upstream does
  (`check_update_item`). Documented on `update`.
- **`merge`** accepts a sketch with a different `k`, as upstream does. Mixing
  `f64` and `f32` sketches is a compile error, so no runtime type check is
  needed for `merge` or the KS functions.
- **`sorted_view()` is infallible.** Upstream's `get_sorted_view` on an empty
  sketch returns an empty view rather than throwing; queries on that view
  then return `EmptySketch`, same as on the sketch.
- **`iter()`** copies the retained items out across FFI once (at most a few
  thousand for typical `k`) and returns a `std::vec::IntoIter`-backed
  iterator. Items are in the sketch's internal level order, not sorted; the
  docs say so and point at `sorted_view()` for sorted order.
- **Sorted-view weights.** The shim reads each entry's `get_weight()` and
  inclusive `get_cumulative_weight()` from upstream's
  `quantiles_sorted_view::const_iterator` — no arithmetic of its own.

### Thread safety

`KllSketch` is `Send` but **not** `Sync`. Upstream's logically-`const`
queries mutate the sketch: `get_quantile`, `get_rank`, `get_PMF` and
`get_CDF` build and cache a sorted view in `mutable` members
(`setup_sorted_view`), and `get_sorted_view` `const_cast`s to sort level
zero in place. Two threads querying one `&KllSketch` would race in C++. A
`compile_fail` doctest pins `KllSketch<f64>: !Sync`.

`KllSortedView` is immutable after construction, so it is `Send + Sync`. The
rustdoc on `KllSketch` names it as the way to share query access across
threads: build a view once, wrap it in `Arc`.

## Errors

Arguments are validated in Rust before crossing FFI, so callers get a typed
variant instead of a C++ message:

| Condition | Error |
|---|---|
| `k` outside `8..=65535` | `InvalidConfig` |
| `rank` NaN or outside `[0, 1]` | `InvalidConfig` |
| `p` NaN or outside `(0, 1)` (KS) | `InvalidConfig` |
| NaN item passed to `get_rank` | `InvalidConfig` |
| NaN in `split_points`, or not strictly increasing | `InvalidConfig` |
| query on an empty sketch or empty sorted view | **`EmptySketch`** (new) |
| KS with either sketch empty | **`EmptySketch`** |
| bytes that do not parse | `Deserialization` |
| any other C++ exception | `Cpp` |

KS with an empty operand is rejected in Rust because upstream's answers
there carry no information: two empty sketches give `delta = 0` and
`test = false` (upstream's own `kolmogorov-smirnov empty` test pins this),
one empty sketch gives `delta = 1`, and `threshold` is NaN (`0/0`) or inf.
The port of upstream's empty-case test asserts `EmptySketch` instead, with a
comment saying why.

`InvalidConfig` is already the repo's general bad-argument variant (Tuple
uses it for value-array length mismatches), so no new variant is needed for
argument validation.

**Non-empty `f32` bytes deserialized as `KllSketch<f64>` (and vice versa)
must return `Deserialization`, never a sketch with garbage items.**
Upstream's bounds and total-size checks catch it, since the item width
changes the byte count; a test pins this for single-item, `n ≤ k` and
estimation-mode inputs. If any case slips through, the shim adds an explicit
check. *Empty* sketches are the exception by format: the 8-byte empty
preamble carries no item type (the same holds in Java), so empty `f32` bytes
deserialize as an empty `KllSketch<f64>`, and a test pins that as correct.

### `SketchError` changes (breaking)

- New variant `SketchError::EmptySketch`: "operation is undefined for an
  empty sketch". The doc comment on the enum, which currently lists HLL,
  Theta and CPC, is updated to name every family.
- `SketchError` becomes `#[non_exhaustive]`, so future families can add
  variants without another breaking release.

Both break exhaustive `match`es on `SketchError`, so this ships as
`apache-datasketches` **0.3.0**.

## Randomness

KLL compaction is randomized: `kll_helper::randomly_halve_{up,down}` draws
from `random_utils::random_bit`, a `thread_local` engine seeded from the
clock and the thread id, with no seeding hook upstream. Consequences:

- Once `n > k`, quantile/rank/PMF/CDF estimates differ run to run. Tests may
  assert them only within the rank-error bound, as upstream's tests do.
- Deterministic regardless of the random bit: `n`, `k`, min/max,
  `num_retained`, `is_estimation_mode`, serialized size, and *every* query
  result while `n ≤ k` (no compaction has happened). Tests assert these
  exactly.
- The benchmark agreement check (below) compares only deterministic fields.

## Testing

All in `apache-datasketches/tests/` unless noted. Each case runs for both
`f64` and `f32` through a small generic helper.

- **`kll_sketch_smoke_test.rs`** — happy paths: construct, update, query,
  merge, serialize round trip.
- **`kll_sketch_test.rs`** — port of upstream `kll/test/kll_sketch_test.cpp`
  (one `TEST_CASE`, many `SECTION`s). Exact assertions for the deterministic
  quantities above; bound-based assertions past `k`. Sections exercising
  custom item types, `std::istream` serialization or `header_size_bytes`
  are skipped with a comment naming them.
- **`kll_validation_test.rs`** — every row of the error table above. Not a
  port of upstream `kll_sketch_validation.cpp`: that test only compiles under
  `-DKLL_VALIDATION`, which replaces the random compaction bit with a
  deterministic one, and the binding builds without it.
- **`kll_serialization_compat_test.rs`** — round trips at each size class;
  upstream's checked-in fixture `kll/test/kll_sketch_float_one_item_v1.sk`
  (copied into the test tree); golden bytes for deterministic sketches
  (`n ≤ k`); the cross-type deserialization rejection. The Java-generated
  `.sk` files read by upstream's
  `kll_sketch_deserialize_from_java_test.cpp` are not in the submodule, so
  the Java-compatibility claim rests on upstream's guarantee that C++ and
  Java bytes are identical, plus the fixture above.
- **`kll_sorted_view_test.rs`** — for a range of sketches, every view query
  equals the same query on the sketch; `iter()` is sorted and its
  `cumulative_weight` ends at `n`.
- **`kolmogorov_smirnov_test.rs`** — port of upstream's 5 KS cases, plus the
  empty-sketch rejection.
- **`kll_concurrency_test.rs`** — a sketch moved across threads; a
  `KllSortedView` shared through `Arc` and queried concurrently.
- **`apache-datasketches-sys/tests/kll_doubles_link_test.rs`** and
  **`kll_floats_link_test.rs`** — link tests behind `--features kll`, like
  every other family.

## Benchmarks

Required by the reproducibility rule in `AGENTS.md`, and for parity with
every other family:

- `apache-datasketches/examples/bench_kll_update.rs` and
  `benches/cpp_reference/kll_update.cc`, `f64`, `k = 200`, same item counts
  and `--reps`/`--ladder` handling as the existing pairs via
  `bench_common.h`.
- Scenarios: `distinct`, `hot`, `quantile` (repeated `get_quantile` on a
  filled sketch), `ser`, `deser`. No `str` scenario — KLL has no string
  update.
- **Agreement check.** Quantile estimates are randomized, so the harnesses
  print the deterministic labels `n=`, `min=`, `max=`, `retained=` and (for
  `ser`) `bytes=` instead of `estimate=`. `compare.py` already compares every
  `name=value` label generically, so it needs no change. `compare.sh` gains
  the new pair; the `benchmark harnesses run` CI job then covers it.
- `benches/cpp_reference/README.md` documents the pair and explains why KLL
  checks `retained` rather than an estimate.

## CI

`.github/workflows/ci.yml`: add `--no-default-features --features kll` to
the per-family feature matrix. The no-family-enabled guard, the package
check and the all-features jobs need no change beyond what `--all-features`
already picks up.

## Documentation

- Rustdoc on every public item (`missing_docs` stays clean), including a
  module-level example in `kll/mod.rs`.
- `apache-datasketches/examples/kll.rs`, runnable with
  `cargo run -p apache-datasketches --example kll --features kll`.
- README: a KLL usage section; the family list goes from four to five.
- `AGENTS.md`: family list, feature list, example and bench commands.

## Release

- `apache-datasketches` **0.3.0** — breaking (`#[non_exhaustive]`,
  `EmptySketch`), plus the new family.
- `apache-datasketches-sys` **0.2.3** — adding a `kll` feature and new
  bridge modules is additive, and the sys crate is versioned independently.
  The safe crate's dependency becomes `version = "0.2.3"`.
- CHANGELOG entry with a **Breaking** subsection for the two `SketchError`
  changes, and a migration note (add a `_ =>` arm).
- Annotated tag `v0.3.0` at the published commit, its annotation stating
  both crates' versions.
