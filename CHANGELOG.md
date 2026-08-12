# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

`apache-datasketches` and `apache-datasketches-sys` are versioned
independently; each entry names the crate it applies to. Both are built
against **Apache DataSketches C++ 5.2.0**, vendored and pinned.

Entries for 0.1.0 through 0.2.0 were reconstructed from git history after the
fact — this file was introduced during 0.2.1.

## [Unreleased]

`apache-datasketches` only; `apache-datasketches-sys` is untouched and stays at
0.2.1.

### Added
- **Benchmark harnesses and native C++ counterparts for HLL, Theta and CPC**, so
  every family now has both. This replaces the one figure in this file that no
  committed code could reproduce: 0.2.2 quoted "1.5–1.9x" for these three,
  taken from a report rather than measured. Measured (10M items, `lg_k = 12`,
  `Hll8`, macOS release, median of three passes):

  | family | scenario | Rust | C++ | ratio | overhead |
  |---|---|---|---|---|---|
  | HLL | distinct | 5.27 | 3.51 | 1.50x | +1.76 ns |
  | HLL | repeated | 4.83 | 3.01 | 1.60x | +1.82 ns |
  | Theta | distinct | 6.01 | 3.60 | 1.67x | +2.41 ns |
  | Theta | repeated | 6.97 | 4.76 | 1.46x | +2.21 ns |
  | CPC | distinct | 6.82 | 5.17 | 1.32x | +1.65 ns |
  | CPC | repeated | 6.82 | 5.14 | 1.33x | +1.68 ns |

  So the reported range was roughly right for HLL and Theta, and pessimistic for
  CPC, which comes in at 1.3x. Nothing reaches 1.9x.

  The more useful reading is the last column. **The binding costs a roughly
  constant 1.7–2.4 ns per update call across all four families**; the ratio
  varies only because the families' own updates differ in cost, so it flatters
  CPC (slowest update, 5.1 ns) and punishes HLL (fastest, 3.0 ns) while saying
  nothing about the binding itself. `AGENTS.md` now asks for the absolute figure
  first for that reason, and for a median of three runs — consecutive passes
  varied by 10–20%.

  No per-call allocation was found in these three families' shims: with no
  per-key summary to carry, their numeric `update_*` are direct pass-throughs.
  The remaining overhead is the FFI crossing itself, which is not removable
  without inlining across the language boundary.
- A `benchmark harnesses run` CI job. The Rust benches are examples, so
  `cargo test`/`clippy --all-targets` compiled them but never ran them, and the
  native C++ reference is not a Cargo target at all — nothing in CI so much as
  compiled it. It would have rotted silently the next time the vendored headers
  moved, which is a poor property for the files whose job is backing up the
  numbers in this changelog. Runs everything at 100k items; deliberately not a
  perf gate, since shared runners are far too noisy for ns/op thresholds.
- A rule in `AGENTS.md`: an "Nx native C++" claim requires a committed
  counterpart in `benches/cpp_reference/`, and paths that cannot have one (the
  generic Tuple sketch calls back into Rust, which has no C++ equivalent) quote
  the concrete-vs-generic ratio instead of an absolute figure.

### Changed
- **`update_str` no longer heap-copies the key on any family.** All six string
  update paths — `HllSketch`, `HllUnion`, `ThetaSketch`, `CpcSketch`,
  `ArrayOfDoublesSketch` and the generic `TupleSketch` — built a `std::string`
  from the borrowed `rust::Str` on every call, allocating a copy of bytes the
  caller already owned. Upstream's `std::string` overload does nothing with it
  but forward `(data, length)` onward, so the shims now forward the borrowed
  pointer and length directly.

  This is the same mistake as the two before it — work performed ahead of
  upstream's `if (hash == 0) return;` screen, so a key C++ discards still paid
  for the allocation — and it is the last instance of it on an update path.

  The empty-string no-op had to be replicated explicitly at each site: it lives
  in the `std::string` overload being bypassed, and the `(data, length)`
  overload does not stand in for it. HLL's screens on a *null pointer* rather
  than a zero length, and a `rust::Str` for `""` is non-null with length 0 —
  so an unguarded forward would hash a zero-length payload and record an item
  where upstream records nothing. `tests/update_str_empty_test.rs` asserts the
  no-op on all six paths, and that each guard still admits a real key.

  For `ArrayOfDoublesSketch` the values-length check deliberately stays *ahead*
  of the empty-key guard, matching upstream, which validates the summary before
  discarding an empty key.

- **Generic Tuple `TupleSketch::update_*` is ~2.1x faster, reaching parity with
  the concrete ArrayOfDoubles path.** At 10M items, `lg_k = 12` (macOS,
  release): 17.1 → 8.0 ns/op distinct and 19.1 → 10.9 ns/op repeated, against
  ArrayOfDoubles' 8.3 / 10.3 on the same run.

  `update_*` built its erased summary with `erase(S::create(value))`, which
  heap-allocates a `Box<dyn RawSummaryOps>` per call. That happens in Rust
  before the FFI crossing, so before upstream's `if (hash == 0) return;` screen
  — a key C++ discarded still paid for the box. This was the third instance of
  the same mistake in this family: work performed ahead of the theta screen.

  `TupleSketch<S>` now holds one erased summary and overwrites its contents per
  update (`summary::refill`), so the update path allocates nothing. Inserting a
  new entry still costs one allocation — that is C++ cloning into its table, and
  is inherent.

  Sound because C++ only borrows the value for the duration of the call:
  upstream reads it to clone into a fresh entry or combine into an existing one,
  and never stores it, so one box can back every update. Implemented by
  upcasting `dyn RawSummaryOps` to its `Any` supertrait — stable since Rust
  1.86 — rather than adding an `as_any_mut` to that public trait, which would
  have been a breaking change to an already-published crate for no benefit.

  Note this keeps one summary alive for the sketch's lifetime, and `reset()`
  does not release it: reset clears the C++ table but keeps the scratch box,
  since the sketch is likely to be updated again. It is freed on drop.

  Supersedes an earlier claim that the generic path's ~2x gap over
  ArrayOfDoubles was structural — the trampoline and a box per *entry* are
  inherent, but the box per *update* was not, and it was almost the entire gap.

## [0.2.2] — 2026-08-11

`apache-datasketches` 0.2.2 and `apache-datasketches-sys` 0.2.1. The
performance work below lives in the sys crate's C++ shims, so
`apache-datasketches` raises its dependency requirement to `0.2.1` — without
that, Cargo could resolve the older sys crate and silently drop the fixes.

### Added
- `vendor_drift_test`: detects divergence between the workspace-root
  `datasketches-cpp` submodule and the copy vendored into
  `apache-datasketches-sys`, which `build.rs` actually compiles. The copy is
  refreshed by hand, so a missed refresh previously left the repo claiming one
  upstream version while users got another, with nothing failing.
- GitHub Actions CI: build, test, clippy and docs on Linux and macOS; the
  per-family feature matrix; an inverted assertion that a build with no family
  enabled is rejected; a check that the packaged sys crate builds with all
  features; and `cargo fmt --all -- --check`.
- `examples/bench_tuple_update.rs`: throughput harness for the ArrayOfDoubles
  update path, at fixed `lg_k = 12` / `num_values = 3` so runs are comparable.
  An example rather than a `[[bench]]` because a `harness = false` bench target
  is *run* by `cargo test`, which would put a multi-million-item loop in CI.
- A sys-crate test that a short values slice terminates the process instead of
  reading out of bounds. This was previously untested, and the change below
  promotes the shim's length check from defence-in-depth to the only guard.
- `examples/bench_tuple_generic_update.rs`: throughput harness for the generic
  Tuple path. Separate from the ArrayOfDoubles one because the cost structures
  differ — the generic path boxes every summary and reaches Rust through a
  trampoline. No native C++ reference exists for it: the callback design has no
  C++ equivalent, so the number to watch is the harness against itself.
- A test pinning the update path's exact clone count — one for a new key, zero
  for a key already present. That is what the generic Tuple change below buys,
  and an off-by-one is precisely the regression worth catching.
- `benches/cpp_reference/`: native C++ counterparts to the Rust benches, built
  against the same vendored headers via `run.sh`. Every "Nx native C++" claim
  below is a Rust bench divided by one of these; until now the reference
  program was ad hoc, so those ratios were not reproducible from a clean
  checkout. Deliberately outside both crate directories, since `cargo package`
  would otherwise ship the `.cc` in the published tarball.

### Changed
- Bumped `actions/checkout` from v4 to v5 in CI. v5.0.0 is the release that
  moves the action to Node 24; GitHub was force-running v4 on Node 24 anyway
  and annotating every job with a deprecation warning. Stopped at v5 rather
  than the current v7 deliberately: v6 changes how credentials are persisted
  and v7 blocks fork-PR checkout for `pull_request_target`/`workflow_run`,
  neither of which this workflow needs.
- Reformatted four cxx bridge files that had been rustfmt-dirty since they
  were written. Mechanical only. The workspace is now rustfmt-clean, so CI can
  enforce formatting.
- **Generic Tuple `TupleSketch::update_*` is ~2.3x faster.** At 10M items,
  `lg_k = 12` (macOS, release): 39.1 → 17.1 ns/op for distinct keys and
  41.5 → 19.1 ns/op for repeated keys.

  The shim wrapped every borrowed summary in a `DynSummary` before handing it
  to upstream, and that wrapper cloned a `Box` unconditionally. Because the
  wrapper was built as the *argument* to `update()`, it was paid before
  upstream's `if (hash == 0) return;` screen — so a key theta rejected cloned a
  summary that was never looked at. The insert path cloned twice: once into the
  wrapper, then again to populate the entry.

  `DynUpdatePolicy::update` now has an overload taking the borrowed
  `const RustSummary&` directly, so the wrapper is gone. A rejected key clones
  zero times, a repeated key clones zero times (it combines in place), and a
  new key clones once. This is the optimisation the shim's own comment had
  described as deliberately deferred; it turned out not to need the
  non-owning-pointer `DynSummary` variant that comment envisaged, because
  upstream's `update` is a forwarding reference and never requires the value to
  be the sketch's summary type — the same property the ArrayOfDoubles fix uses
  to pass a bare `const double*`.

  Sound because upstream only ever *reads* the update value — cloning from it
  into a fresh entry, or combining from it into an existing one — and never
  moves or stores it, so the borrow cannot outlive the call
  (`tuple_sketch_impl.hpp:213-223`). No behaviour change: estimates and
  retained counts are identical, and the user-visible contract on
  `TupleSummary` is unchanged. Summaries are cloned strictly fewer times, which
  only matters if a `Clone` impl has side effects — it must be total anyway,
  since a panic there aborts.
- **`ArrayOfDoublesSketch::update_*` is ~3.3x faster.** At 10M items,
  `lg_k = 12`, `num_values = 3` (macOS, release): 26.3 → 8.0 ns/op for distinct
  keys and 25.8 → 9.9 ns/op for repeated keys. Against native C++ compiled from
  the same vendored headers, that closes the gap from 7.7x to 2.3x (distinct)
  and 3.9x to 1.5x (repeated) — comparable to the other families. (This
  originally cited "the 1.5–1.9x the other families already showed", a figure
  taken from a report rather than measured here; the three families have since
  been benchmarked, and the measured range is 1.3–1.7x. See Unreleased.) Two
  independent causes, listed by size of contribution:

  - The shim copied the caller's slice into a fresh `std::vector<double>` on
    every update, so every call heap-allocated. Upstream screens the key first
    (`if (hash == 0) return;`) and never reads the values for a key theta
    rejects, so native C++ allocates nothing per update. The shim now passes
    `values.data()` directly: upstream's update policy is templated on anything
    with indexed access and its own comment blesses `double*`. This accounted
    for most of the improvement — on the distinct-key case, 26.3 → 11.8 ns/op
    of the total 26.3 → 8.0.
  - `check_values` in the safe wrapper read `num_values` back from C++ before
    every update, costing a full FFI crossing per call. It is now cached in the
    Rust struct, which is sound because the value is fixed for the sketch's
    lifetime. This accounted for the remainder, 11.8 → 8.0 ns/op. (Run-to-run
    variance is around 5%, so treat the split as approximate; the ~3.3x
    headline is well outside it.)

  The tell that this was per-call overhead ahead of the theta screen: before
  the fix, the distinct and repeated-key cases cost the same (26.3 vs 25.8),
  while in native C++ distinct is nearly twice as *fast* as repeated (3.5 vs
  6.6) precisely because screened keys return immediately.

  No behaviour change — estimates and retained counts are identical before and
  after. Note the tradeoff: a bare pointer carries no length, so the shim's
  `check_values_len` is now the only thing preventing an out-of-bounds read for
  direct sys-crate callers, and because the update bridge fns are declared
  without `Result` it terminates the process rather than returning an `Err`.
  Users of the safe `apache-datasketches` API are unaffected — that layer
  validates the length and returns `InvalidConfig`.

## [0.2.1] — 2026-08-10

### Changed
- **`apache-datasketches`: all four sketch families are now enabled by
  default.** 0.2.0 shipped `default = []`, so `apache-datasketches = "0.2"`
  compiled cleanly and exposed nothing but `SketchError` — no error, no
  warning, no hint that a feature was needed.

  This only adds APIs, so it cannot break existing code. For the minimal
  build, use `default-features = false` and name the families you want. Note
  that Cargo unifies features across a dependency graph, so opting out only
  holds if nothing else in the graph pulls this crate with default features.

  The cost is C++ compile time, not binary size — the linker drops families
  you never call. Cold debug builds of the FFI layer run about 4.5s with no
  families and about 20s with all four; `theta` (~+4s) and `tuple` (~+7s)
  account for nearly all of it, while `hll` and `cpc` are free.

### Added
- Disabling default features without naming at least one family is now a
  `compile_error!` explaining what to do, rather than a crate that silently
  exposes nothing.

`apache-datasketches-sys` was unchanged and remains at 0.2.0.

## [0.2.0] — 2026-08-10

Both crates. Three families landed in this release; it was staged over a long
period and published all at once.

### Added
- **Theta sketch family** (`theta` feature): sketch, compact and wrapped-compact
  forms, union, intersection, a-not-b, and Jaccard similarity.
- **Tuple sketch family, ArrayOfDoubles form** (`tuple` feature): a fixed-width
  array of `f64` per distinct key, summed on collision, with union,
  intersection, a-not-b, Jaccard similarity, serialization and entry iteration.
- **Tuple sketch family, generic form** (`tuple::generic`, same `tuple`
  feature): a summary type you define in Rust, carried per distinct key.
  Implement the `TupleSummary` trait and C++ calls back into Rust to clone and
  combine summaries. Provides `TupleSketch<S>`, `CompactTupleSketch<S>` with
  `entries()` yielding `(hash, S)`, `TupleUnion<S>`, `TupleIntersection<S>`,
  `TupleAnotB<S>` and `tuple_jaccard_similarity`.

  Sharp edges, all documented and tested: a panic in `union_combine`,
  `intersection_combine` or `Clone::clone` aborts the process, because panics
  cannot cross the FFI boundary — make those total. A panic in `create` is an
  ordinary catchable Rust panic. Every generic type is `Send` but not `Sync`.
  Serialization of generic sketches is not supported; use the ArrayOfDoubles
  form to persist a sketch.
- `build.rs` now fails loudly on a missing bridge or shim file instead of
  silently skipping it, and rejects duplicate cxx bridge names across the
  crate.

### Changed
- **Breaking (`apache-datasketches`): `default = []`.** Prior versions defaulted
  to `features = ["hll"]`, so consumers upgrading from 0.1.x had to name
  `features = ["hll"]` explicitly to keep HLL. Superseded by 0.2.1, which
  enables all families by default.

### Fixed
- `theta`: corrected a doc comment that claimed `trim()` does not change the
  estimate.

## [0.1.1] — 2026-07-26

### Added
- Per-crate `README.md` files, so both crates.io pages render documentation.
  This was the entire purpose of the release.

## [0.1.0] — 2026-07-26

### Added
- Initial release of both crates: safe Rust bindings for the **HLL sketch
  family** (`hll` feature) — `HllSketch` and `HllUnion`, with serialization —
  over vendored `datasketches-cpp`, bridged with `cxx`.

[Unreleased]: https://github.com/asatale/apache-datasketches-rs/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/asatale/apache-datasketches-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/asatale/apache-datasketches-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/asatale/apache-datasketches-rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/asatale/apache-datasketches-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/asatale/apache-datasketches-rs/releases/tag/v0.1.0
