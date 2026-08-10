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
cargo run -p apache-datasketches --example tuple_generic --features tuple

# Lint (missing_docs is enforced — see below)
cargo clippy --workspace --all-features

# Format a single file — `cargo fmt -- <file>` does NOT scope to that path in
# this repo (cargo fmt reformats the whole workspace regardless), so use
# rustfmt directly:
rustfmt --edition 2021 <file>
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

The `tuple` feature holds a second family alongside ArrayOfDoubles: the type-erased generic sketches
(`apache-datasketches/src/tuple/generic/`), whose C++ layer calls *back* into Rust to clone and combine
summaries via the `extern "Rust"` opaque type `RustSummary`. That inverts two of the layering habits
above. First, cxx allows an `extern "Rust"` opaque type to be declared in exactly one bridge module per
crate and offers no alias for it, so every bridge fn whose signature mentions `RustSummary` — the
sketch shim's `update_*` and the compact shim's `entry_summary` — must share the single bridge
`apache-datasketches-sys/src/tuple_generic.rs`, rather than getting one bridge file per shim. (Union,
intersection, a-not-b and Jaccard never name `RustSummary`, so they keep their own bridge files; the
global name-uniqueness rule below still applies to all of them.) Second, the shim headers must *not*
include the cxx-generated header — that is a real include cycle — so they forward-declare the
trampolines and `#include` the generated header only from the `.cc`; forward declarations have to match
cxx's emitted signatures exactly, `noexcept` included.

### `cxx::bridge` names must be globally unique across the sys crate

Every `#[cxx::bridge]` free-function name, and every shared struct/enum name, must be globally unique
across all bridges in the sys crate. cxx keys the generated `extern "C"` trampoline on the namespace
and the name only — not on parameter types and not on the bridge module — so two bridges declaring the
same free-function name emit the same symbol and the linker silently picks one, which manifests as a
crash or wrong results rather than a link error. Prefix with the family name when a name would
otherwise repeat (`tuple_jaccard_*`, `TupleResizeFactor`, `TupleJaccardBoundsFfi`). Methods are safe
because the receiver type is part of the symbol, but do not rely on that when refactoring a method into
a free function. `build.rs` runs a check at build time that scans the bridges being compiled and panics
on a duplicate definition — see the comment there for what it does and does not catch.

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

`apache-datasketches-sys/build.rs` adds a bridge file to `cxx_build::bridges(...)` and its
corresponding shim `.cc` to the C++ build based on the feature flag alone; each per-family file list
is asserted exhaustive via `require_exists(path)`, which panics with a clear message if a listed file
is missing (typo, or a file moved/deleted without updating the list). A family under active
construction across multiple tasks/commits may temporarily replace a `require_exists` call with the
old skip-if-absent check so `--features <family>` keeps building at intermediate states — but the
commit that completes the family must restore the hard assertion before landing.

### Rustdoc coverage is enforced

`apache-datasketches/src/lib.rs` sets `#![warn(missing_docs)]` at the crate root (the sys crate does
not, since its FFI surface is unstable and not meant to be consumed directly). Every public item (types, variants, fields,
methods, modules) needs a `///`/`//!` doc comment. Keep this clean when adding public API.

### Design docs

Feature work is planned under `docs/superpowers/`: `specs/` holds design docs
(`YYYY-MM-DD-<feature>-design.md`), `plans/` holds implementation plans (`YYYY-MM-DD-<feature>.md`).
Check these before starting on a new sketch family for prior design decisions and rationale.

### CI and branch protection

`.github/workflows/ci.yml` runs on every push and pull request: rustfmt, then
build/test/clippy/docs on Linux and macOS, the per-family feature matrix, an
inverted check that a build with no family enabled is rejected, and a check
that the packaged sys crate builds with all features. The `CI success` job
aggregates the rest and is the only status check branch protection requires —
matrix leg names embed their parameters, so requiring them directly would need
protection edited whenever the matrix changes.

`main` is protected: a pull request is required to merge (zero approvals, so a
solo maintainer is not blocked), `CI success` must pass, the branch must be up
to date, and force pushes and deletions are refused. Admin enforcement is off,
so a repository admin can still push directly when needed — prefer a PR.

Note the workflow deliberately does not set `RUSTFLAGS: -D warnings` globally:
that applies to dependencies too, so one warning in a future release of `cxx`
would fail CI for something this repo cannot fix. Warnings in our own code are
caught by the clippy step across every target, and by `RUSTDOCFLAGS` for docs.
