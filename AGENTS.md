# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## What this is

Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp) (the C++ library),
built via the `cxx` crate over a pinned git submodule. Two-crate workspace:

- `apache-datasketches-sys` — raw `cxx` FFI bridge. Unstable, low-level; not meant to be used directly.
- `apache-datasketches` — safe, idiomatic Rust API that wraps `-sys`. This is what consumers depend on.

Both crates have `default = []`; every sketch family is opt-in via Cargo features: `hll`, `theta`, `cpc`, `tuple`.

## Setup

The C++ sources are vendored as a git submodule and must be checked out before building:

```bash
git submodule update --init --recursive
```

A C++17 compiler must be on `PATH` (used by `cc`/`cxx-build`).

## Common commands

```bash
# Build / test everything (no sketch families enabled — mostly a compile check)
cargo build --workspace
cargo test --workspace

# Build / test with specific sketch families enabled — do this when working
# on a specific family, since most code is gated behind these features
cargo test -p apache-datasketches --features hll
cargo test -p apache-datasketches --features theta
cargo test -p apache-datasketches --features cpc
cargo test -p apache-datasketches --features tuple
cargo test --workspace --features hll,theta,cpc,tuple

# Run a single test
cargo test -p apache-datasketches --features hll hll_sketch_test::some_test_name

# Run link tests in the -sys crate (verify the FFI bridge compiles/links; require their feature)
cargo test -p apache-datasketches-sys --features hll

# Run the runnable examples
cargo run -p apache-datasketches --example hll --features hll
cargo run -p apache-datasketches --example theta --features theta
cargo run -p apache-datasketches --example cpc --features cpc
cargo run -p apache-datasketches --example tuple --features tuple

# Lint (missing_docs is enforced — see below)
cargo clippy --workspace --all-features
```

## Architecture

### Layering: shim (C++) → bridge (`-sys`) → safe wrapper (`apache-datasketches`)

Each sketch type/operation (e.g. HLL sketch, HLL union, Theta intersection) is implemented as three
parallel files across the two crates, all following the same naming convention:

1. **C++ shim** (`apache-datasketches-sys/cpp/<family>/<name>_shim.{h,cc}`) — a thin C++ wrapper around
   the vendored `datasketches-cpp` template classes, adapted to a `cxx`-friendly, non-templated
   interface (e.g. exposing separate shim types instead of C++ templates).
2. **`cxx::bridge` module** (`apache-datasketches-sys/src/<name>.rs`) — declares the FFI surface
   (`unsafe extern "C++"` block) for that shim: opaque types, methods, and any shared enums. Included
   in the build only when its family's feature is enabled, both in `src/lib.rs` (`#[cfg(feature = ...)]`)
   and in `build.rs` (which lists which bridge files and `.cc` files to compile per feature).
3. **Safe Rust wrapper** (`apache-datasketches/src/<family>/<name>.rs`) — the public, idiomatic API.
   Converts `cxx::Exception` into `SketchError` (see `apache-datasketches/src/error.rs`), converts
   between the safe-layer's own public enums (e.g. `TargetHllType`) and the `-sys` crate's bridge
   enums via `From` impls, and otherwise should be the only place callers interact with.

When adding a new sketch operation, expect to touch all three layers in parallel, plus `build.rs` (to
register the new bridge module/`.cc` file under the right feature) and the family's `mod.rs` (to
re-export the new public type).

### Vendored C++ sources — two copies, one source of truth

- `vendor/datasketches-cpp` (repo root) — the git submodule, pinned to a tag (see its `README.md`).
  This is the source of truth when bumping the pinned version.
- `apache-datasketches-sys/vendor/datasketches-cpp` — a **manual copy** of just the headers actually
  compiled (`common/`, `hll/`, `theta/`, `cpc/`, `tuple/` `include/` dirs, plus `LICENSE`/`NOTICE`). This copy is
  what `build.rs` actually builds against, and what ships in the published `-sys` crate tarball,
  because `cargo package` only includes files inside the crate's own directory (a `../` path escaping
  it would be missing from the published package).

  After bumping the submodule's pinned tag, refresh this copy manually — see
  `apache-datasketches-sys/vendor/README.md` for the exact copy commands. If a future sketch family
  needs headers outside the four directories above, add its `include/` dir to both that script and
  `build.rs`'s `.include(...)` calls.

### `build.rs` conditional compilation

`apache-datasketches-sys/build.rs` only adds a bridge file to `cxx_build::bridges(...)` and its
corresponding shim `.cc` to the C++ build if both the feature is enabled *and* the file exists on disk
(`Path::new(path).exists()`). This lets a sketch family's bindings be built up incrementally across
multiple tasks/commits without breaking `--features <family>` at intermediate states — don't remove
these existence checks when adding new files unless the whole family is already complete.

### Rustdoc coverage is enforced

Both crates set `#![warn(missing_docs)]` at the crate root. Every public item (types, variants, fields,
methods, modules) needs a `///`/`//!` doc comment. Keep this clean when adding public API.

### Design docs

Feature work is planned under `docs/superpowers/`: `specs/` holds design docs
(`YYYY-MM-DD-<feature>-design.md`), `plans/` holds implementation plans (`YYYY-MM-DD-<feature>.md`).
Check these before starting on a new sketch family for prior design decisions and rationale.
