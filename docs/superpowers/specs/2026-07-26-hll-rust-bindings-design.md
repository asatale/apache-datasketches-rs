# Apache DataSketches HLL Rust Bindings — Design

## Purpose

Create a new Rust crate (published to crates.io) that provides safe, idiomatic
Rust bindings to [apache/datasketches-cpp](https://github.com/apache/datasketches-cpp),
vendored as a git submodule. The C++ test suite for each wrapped sketch family
is ported to Rust to validate correctness and provide ongoing regression
coverage.

**Scope of this design:** the HyperLogLog (HLL) sketch family only —
`hll_sketch` and `hll_union`. This is the first sketch family; the
architecture is chosen so additional families (Theta, KLL, CPC, etc.) can be
added later as a repeatable pattern rather than a redesign.

## FFI approach

**`cxx`** is used for the Rust/C++ boundary, not `bindgen` or `autocxx`.

Rationale: datasketches-cpp's sketch types are C++ templates, so no FFI tool
avoids writing a small C++ shim of concrete (non-template) wrapper classes.
Given that a shim is unavoidable, `cxx` was chosen over `bindgen` because it
gives compile-time type safety across the boundary (distinct opaque types per
sketch family, rather than bindgen's untyped `void*`), automatic
exception-to-`Result` conversion, and native bridging of `Vec<u8>` / `String`
/ `UniquePtr` — all of which avoid hand-rolled, per-function translation code
that would otherwise be repeated for every method, in every sketch family, as
the project grows to cover more of datasketches-cpp. `autocxx` was considered
but rejected due to risk of hitting unsupported template/CRTP patterns in
datasketches-cpp with a smaller, less mature toolchain to fall back on.

## Repo layout

```
apache-rust-sketch-wrapper/
├── vendor/
│   └── datasketches-cpp/            git submodule, pinned to a tagged release
├── apache-datasketches-sys/         raw cxx bridge crate
│   ├── Cargo.toml
│   ├── build.rs                     cxx_build, gated per Cargo feature
│   ├── cpp/
│   │   ├── common/                  shared shim utilities (error/byte helpers)
│   │   └── hll/
│   │       ├── hll_sketch_shim.h/.cc
│   │       └── hll_union_shim.h/.cc
│   ├── src/
│   │   ├── lib.rs
│   │   └── hll.rs                   #[cxx::bridge] module for HLL
│   └── tests/                       link-level smoke tests
├── apache-datasketches/             safe, idiomatic crate (what users depend on)
│   ├── Cargo.toml                   depends on apache-datasketches-sys
│   ├── src/
│   │   ├── lib.rs
│   │   ├── error.rs                 SketchError (single type, shared across families)
│   │   └── hll/
│   │       ├── sketch.rs            HllSketch
│   │       └── union.rs             HllUnion
│   └── tests/                       ported C++ test suite, 1:1 file mirror
│       ├── hll_sketch_test.rs       mirrors hll_sketch_test.cpp
│       └── hll_union_test.rs        mirrors hll_union_test.cpp
├── LICENSE-MIT
├── LICENSE-APACHE
└── README.md
```

## Crate structure: sys + safe split

Two published crates, the standard convention for wrapping a native library
(cf. `libz-sys`/`flate2`, `libsqlite3-sys`/`rusqlite`):

- **`apache-datasketches-sys`** — raw bridge layer. Owns the submodule
  reference, the C++ shim classes, the `#[cxx::bridge]` declarations, and the
  `build.rs` compilation step. Thin, mechanical, not meant to be pleasant to
  use directly.
- **`apache-datasketches`** — safe, idiomatic API that users actually depend
  on. Wraps the sys crate's raw bridged types in ergonomic Rust types with
  proper error handling, no C++ knowledge required by callers.

## Scaling to future sketch families

Both crates remain **single crates**, not one crate pair per sketch family.
Each family (HLL, Theta, KLL, CPC, ...) is gated behind a Cargo feature flag
(`hll`, `theta`, `kll`, ...). `build.rs` only compiles the C++ shim files for
enabled features; the safe crate only compiles the corresponding Rust module
for enabled features. `hll` is a default feature so `cargo add
apache-datasketches` works out of the box today.

This was chosen over a crate-pair-per-family layout because the latter (~18
crates at full scope) multiplies publishing/versioning/submodule-pinning
overhead without a corresponding benefit for a project maintained by one or a
few people, and shared C++ utility code (hashing, serialization helpers used
across families) would otherwise need its own additional shared crate.
Splitting a feature out into its own crate later is a much smaller migration
than merging many crates back into one, if that ever became necessary.

## Data flow

Rust call (safe crate) → input validation/conversion → call into sys crate's
bridged function → cxx-generated C++ glue → shim method → real templated
`datasketches::hll_sketch`/`hll_union`. C++ exceptions are either caught in
the shim and converted to an error return, or propagate through cxx's
automatic exception→`Result` boundary for bridged functions declared to
return `Result<T>`. The safe crate maps `cxx::Exception`/error into
`SketchError`. Serialization (`serialize()` returning `std::vector<uint8_t>`)
bridges directly to `Vec<u8>` — no manual buffer/length handling.

## Error handling

A single `SketchError` enum in the safe crate, shared across all sketch
families (not one error type per family), to keep the public API small and
consistent. Variants cover invalid configuration, serialization failures, and
a catch-all wrapping C++ exception messages. Construction-time validation
that Rust can check cheaply (e.g. `lg_k` range) happens before crossing the
FFI boundary, avoiding an unnecessary round-trip through C++ exceptions.

## Concurrency

`HllSketch` and `HllUnion` are `unsafe impl Send` (sound: the underlying C++
object holds no shared/global state, and ownership transfers cleanly across
threads) but explicitly **not** `Sync`. This matches the underlying C++
library's own thread-safety semantics — a sketch is not safe to mutate
concurrently from multiple threads without external synchronization. Users
needing shared concurrent access wrap a sketch in a `Mutex`/`RwLock`
themselves. This is documented in the crate docs and verified with a test
that moves a sketch across threads.

## Testing

- **1:1 file mirror**: `apache-datasketches/tests/hll_sketch_test.rs` ports
  `hll_sketch_test.cpp` test-by-test, same names/order where practical, with
  a comment linking back to the upstream file. Same for `hll_union_test.rs`.
  Binary test fixtures used by the C++ suite (if any) are copied alongside.
- Additional Rust-specific tests not present in the C++ suite: `Send`
  verification (moving a sketch across a thread boundary), `SketchError`
  conversion behavior, and feature-flag compilation (crate builds correctly
  with only the `hll` feature enabled).
- Link-level smoke tests in the `-sys` crate, separate from the safe crate's
  behavioral test suite.

## Submodule versioning

The `datasketches-cpp` submodule is pinned to a specific tagged release (not
tracking a branch), so builds are reproducible. Upgrades are deliberate,
reviewed commits that bump the submodule pointer. The pinned version is
recorded in the sys crate's README/Cargo.toml metadata.

## API surface (HLL, v1)

- `update()` supports full parity with the C++ overload set: `u64`, `i64`,
  `f64`, `&str`, and `&[u8]` (distinct methods, e.g. `update_u64`,
  `update_str`, `update_bytes`, mirroring the C++ overloads since Rust has no
  overloading).
- Both `hll_sketch` and `hll_union` are in scope for v1 (not deferred),
  including HLL target type (`HLL_4`/`HLL_6`/`HLL_8`) and configurable
  `lg_k`.

## Licensing

Dual MIT/Apache-2.0, the Rust ecosystem convention, compatible with the
vendored datasketches-cpp submodule's Apache-2.0 license.

## CI

None for v1. Builds and tests are run locally. CI may be added in a later
iteration.

## Out of scope for this design

- Sketch families other than HLL (Theta, KLL, CPC, Tuple, REQ, Frequent
  Items, VarOpt/Sampling, etc.) — each is a follow-up design/implementation
  cycle using the feature-flag pattern established here.
- CI/CD pipeline setup.
- Prebuilt binary distribution (crates.io native-dependency builds are
  source-only via `build.rs`/`cxx-build` for v1).
