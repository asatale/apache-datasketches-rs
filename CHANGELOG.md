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
- **`ArrayOfDoublesSketch::update_*` is ~3.3x faster.** At 10M items,
  `lg_k = 12`, `num_values = 3` (macOS, release): 26.3 → 8.0 ns/op for distinct
  keys and 25.8 → 9.9 ns/op for repeated keys. Against native C++ compiled from
  the same vendored headers, that closes the gap from 7.7x to 2.3x (distinct)
  and 3.9x to 1.5x (repeated) — in line with the 1.5–1.9x the other families
  already showed. Two independent causes, listed by size of contribution:

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

[Unreleased]: https://github.com/asatale/apache-datasketches-rs/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/asatale/apache-datasketches-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/asatale/apache-datasketches-rs/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/asatale/apache-datasketches-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/asatale/apache-datasketches-rs/releases/tag/v0.1.0
