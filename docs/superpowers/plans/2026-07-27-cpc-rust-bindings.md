# CPC Rust Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the CPC (Compressed Probabilistic Counting) sketch family (`cpc_sketch`, `cpc_union`) to `apache-datasketches-sys`/`apache-datasketches`, following the design in `docs/superpowers/specs/2026-07-27-cpc-rust-bindings-design.md`, mirroring the HLL and Theta implementations' patterns.

**Architecture:** Two small non-template C++ shim classes wrap the real templated `datasketches::cpc_sketch_alloc<std::allocator<uint8_t>>` (alias `cpc_sketch`) and `datasketches::cpc_union_alloc<std::allocator<uint8_t>>` (alias `cpc_union`), each bridged to Rust via its own `cxx::bridge` module (two bridge files, matching HLL's file-per-family granularity — CPC has no compact/wrapped split like Theta). `apache-datasketches` wraps these in two idiomatic Rust types (`CpcSketch`, `CpcUnion`) plus their builders (`CpcSketchBuilder`, `CpcUnionBuilder`) and a `cpc::init()` function addressing CPC's global-decompression-table initialization hazard (not present in HLL/Theta).

**Tech Stack:** Rust (stable), `cxx` + `cxx-build` crates, C++17, existing Cargo workspace, existing `datasketches-cpp` git submodule at tag `5.2.0` (already contains `cpc/include/` at the workspace root; not yet copied into `apache-datasketches-sys/vendor/`).

## Execution Setup

This plan **must** be implemented in a new, separate git worktree — not the
`theta-sketch-bindings` worktree used for the Theta plan, which has already
been merged and removed. Before dispatching Task 1, use
`superpowers:using-git-worktrees` to create a fresh worktree (e.g. branch
name `cpc-sketch-bindings`) from the current `main`.

## Global Constraints

- FFI layer uses `cxx`, not `bindgen`/`autocxx` (unchanged from HLL/Theta).
- `datasketches-cpp` submodule stays pinned to tag `5.2.0` at
  `vendor/datasketches-cpp` (already vendored at the workspace root,
  already contains `cpc/include/`; no re-pin needed for CPC — only a copy
  into `apache-datasketches-sys/vendor/` is needed, same as Theta's Task 1).
- Every CPC constructor and (de)serialize call always uses upstream's
  `DEFAULT_SEED` internally; **no type in this plan exposes a seed
  parameter**.
- Two Rust types — `CpcSketch` (mutable, update-and-serialize, unlike
  Theta's split update/compact types) and `CpcUnion` — with builders
  `CpcSketchBuilder`/`CpcUnionBuilder`. No shared query trait between them
  (matches HLL/Theta's no-trait precedent; CPC doesn't even need one since
  `CpcUnion` has no query methods of its own — only `get_result()`).
- `update()` overloads mirror the **full** upstream overload set (wider
  than HLL/Theta's u64/i64/f64/str/bytes subset): `update_u64`/
  `update_i64`/`update_u32`/`update_i32`/`update_u16`/`update_i16`/
  `update_u8`/`update_i8`/`update_f64`/`update_f32`/`update_str`/
  `update_bytes`.
- `get_lower_bound`/`get_upper_bound` take a `u8` parameter named
  `num_std_dev` (matching HLL/Theta's public-API naming convention, even
  though upstream calls the equivalent C++ parameter `kappa`). **Validation
  happens in C++, not pre-checked in Rust**: upstream itself throws
  `std::invalid_argument` for values outside `1..=3`, and the bridge
  declares these methods `Result<f64>` so cxx's automatic exception→
  `Result` conversion handles it — exactly the same pattern as
  `HllSketch::get_lower_bound`/`get_upper_bound`. (This corrects the design
  doc's phrasing, which suggested Rust-side pre-validation; this plan
  follows the established HLL/Theta precedent instead, which validates via
  the existing C++ exception path with no duplicate Rust-side check.)
- `CpcSketch` also exposes `get_lg_k() -> u8` (present upstream but omitted
  from the design doc's method list — needed by the ported
  `cpc_union_test.cpp` cases in Task 7, and a natural, already-public
  upstream query method to expose alongside the others).
- `CpcSketchBuilder`/`CpcUnionBuilder` validate `lg_k` the same way — no
  Rust-side range pre-check; upstream's constructor throws for
  `lg_k` outside `4..=26`, converted to `SketchError::InvalidConfig` via
  the existing `cxx::Exception` → `SketchError` `From` impl.
- Reuses the existing single `SketchError` enum — **no new variant** for
  CPC.
- `cpc::init()` wraps upstream's `cpc_init<std::allocator<uint8_t>>()`,
  addressing the one CPC-specific concurrency hazard (lazy, not-thread-safe
  global decompression-table initialization) that doesn't exist for HLL or
  Theta.
- Every new Rust sketch/union type is `unsafe impl Send`, explicitly not
  `Sync` (matching `HllSketch`/`ThetaSketch` etc.).
- Tests are 1:1-ported from the two upstream Catch2 files listed below,
  same test names/order where practical, each file with a header comment
  linking to its upstream source and disclosing every excluded case's
  rationale — plus new, clearly-marked non-upstream tests for `Send` and
  the `cpc::init()` concurrent-use scenario.
- Both crates already have `default = []` (set during the Theta plan) —
  **no feature-flip needed this time**. `hll = []`, `theta = []`, and the
  new `cpc = []` (sys) / `cpc = ["apache-datasketches-sys/cpc"]` (safe) are
  all explicit opt-in features.
- Both crates already enforce `required-features` on every `[[test]]`/
  `[[example]]` entry (added during the Theta plan's Task 16) — **this
  plan adds the corresponding entry in the same task that introduces each
  new test/example file**, not deferred to a final task.
- Dual MIT/Apache-2.0 license (unchanged).
- No CI in this plan (unchanged from HLL/Theta).

## Reference: real datasketches-cpp CPC API (verified against tag 5.2.0, from `vendor/datasketches-cpp/cpc/include/`)

Constants (`cpc_common.hpp`):

```cpp
namespace datasketches::cpc_constants {
  const uint8_t MIN_LG_K = 4;
  const uint8_t MAX_LG_K = 26;
  const uint8_t DEFAULT_LG_K = 11;
}
```

`cpc_sketch` (`cpc_sketch_alloc<std::allocator<uint8_t>>`, `cpc_sketch.hpp`):

```cpp
template<typename A> void cpc_init(); // global decompression table init; NOT thread-safe if lazily triggered concurrently

class cpc_sketch {
public:
  explicit cpc_sketch(uint8_t lg_k = cpc_constants::DEFAULT_LG_K, uint64_t seed = DEFAULT_SEED, const A& allocator = A());

  uint8_t get_lg_k() const;
  bool is_empty() const;
  double get_estimate() const;
  double get_lower_bound(unsigned kappa) const; // throws std::invalid_argument outside 1..=3
  double get_upper_bound(unsigned kappa) const; // throws std::invalid_argument outside 1..=3

  void update(const std::string& value);
  void update(uint64_t value); void update(int64_t value);
  void update(uint32_t value); void update(int32_t value);
  void update(uint16_t value); void update(int16_t value);
  void update(uint8_t value);  void update(int8_t value);
  void update(double value);   void update(float value);
  void update(const void* value, size_t size);

  string<A> to_string() const;

  vector_bytes serialize(unsigned header_size_bytes = 0) const;
  static cpc_sketch_alloc<A> deserialize(const void* bytes, size_t size, uint64_t seed = DEFAULT_SEED, const A& allocator = A());

  static size_t get_max_serialized_size_bytes(uint8_t lg_k);

  // @private, not exposed by this plan: get_num_coupons(), validate()
};
```

`cpc_union` (`cpc_union_alloc<std::allocator<uint8_t>>`, `cpc_union.hpp`):

```cpp
class cpc_union {
public:
  explicit cpc_union(uint8_t lg_k = cpc_constants::DEFAULT_LG_K, uint64_t seed = DEFAULT_SEED, const A& allocator = A());

  void update(const cpc_sketch_alloc<A>& sketch);
  void update(cpc_sketch_alloc<A>&& sketch); // not used by this plan's shim; the const& overload covers every use

  cpc_sketch_alloc<A> get_result() const;
};
```

Note: `cpc_union` has **no** `get_estimate`/`is_empty`/`reset` convenience
methods of its own upstream (unlike `hll_union`) — callers must go through
`get_result()` to query anything. This plan's `CpcUnion` matches that: no
convenience query methods, no `reset()`.

## Test Inventory (for Tasks 6-7)

`cpc/test/cpc_sketch_test.cpp` — 26 upstream cases; **15 ported, 11
excluded**:
- Ported: `lg k limits`, `empty`, `one value`, `many values`, the five
  `serialize deserialize {empty,sparse,hybrid,pinned,sliding}, bytes`
  cases (using the byte-vector API bodies, which additionally check
  truncated-buffer error behavior), `serialize deserialize sliding huge`
  (lg_k=26, n=10,000,000), `kapp range` (kappa validation), `update int
  equivalence`, `update float equivalence`, `update string equivalence`,
  `max serialized size`.
- Excluded: `overflow bug` (100,000,000 updates — impractically slow for a
  routine local test suite; regression-tests a historical bug already
  covered behaviorally by the ported large-scale cases), the five
  ostream-based `serialize deserialize {empty,sparse,hybrid,pinned,
  sliding}` cases (duplicates of the byte-vector `..., bytes` variants
  above — this crate's public API only has the byte-vector form),
  `serializing deserialize sliding large` (ostream-only, n=3,000,000 —
  redundant with the ported `sliding`/`sliding huge` tiers), `copy` (no
  `Clone` on `CpcSketch`), `serialize deserialize empty, custom seed` (no
  seed parameter exposed), `validate fail` (`validate()` not exposed),
  `serialize both ways` (`header_size_bytes` not exposed).

`cpc/test/cpc_union_test.cpp` — 9 upstream cases; **6 ported, 3 excluded**:
- Ported: `lg k limits`, `empty`, `large` (adapted — see below),
  `reduce k empty`, `reduce k sparse`, `reduce k window`.
- Excluded: `copy` (no `Clone` on `CpcUnion`), `custom seed` (no seed
  exposed), `moving update` (tests a C++-specific move-constructor update
  overload as a copy-avoidance optimization; behaviorally identical to
  updating via a reference, already exercised by every other ported case).
- `large` is **adapted**: upstream additionally asserts
  `r.get_num_coupons() == s.get_num_coupons()`, but `get_num_coupons()` is
  not exposed (`@private` upstream) — port keeps only the estimate
  comparison.

---

### Task 1: Vendor CPC headers into `apache-datasketches-sys` + `cpc` feature/build.rs wiring

**Files:**
- Create: `apache-datasketches-sys/vendor/datasketches-cpp/cpc/include/` (copy from root submodule)
- Modify: `apache-datasketches-sys/vendor/README.md`
- Modify: `apache-datasketches-sys/Cargo.toml`
- Modify: `apache-datasketches-sys/build.rs`

**Interfaces:**
- Produces: a `cpc` Cargo feature (non-default, alongside `hll`/`theta`) on `apache-datasketches-sys`, and a `build.rs` that includes CPC's headers and is wired to compile `cpp/cpc/*_shim.cc` under `cfg!(feature = "cpc")` once those files exist (Task 2+). This task alone does not add any shim files, so the `cpc` feature does not yet compile any new C++ — verified in Task 2.

- [ ] **Step 1: Copy the CPC headers into the crate-local vendor copy**

```bash
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/cpc
cp -R vendor/datasketches-cpp/cpc/include apache-datasketches-sys/vendor/datasketches-cpp/cpc/include
```

- [ ] **Step 2: Update `apache-datasketches-sys/vendor/README.md`'s sync script**

Replace the file's contents with:

```markdown
# Vendored datasketches-cpp headers

This directory is a manual copy of the headers this crate builds against,
taken from the `vendor/datasketches-cpp` git submodule at the repo root.
It exists so `cargo package`/`cargo publish` — which only includes files
inside this crate's own directory — can produce a self-contained tarball.

Only the headers actually compiled (`common/include`, `hll/include`,
`theta/include`, `cpc/include`, `LICENSE`, `NOTICE`) are copied;
`version.hpp.in` is skipped since nothing in the HLL path includes it.

## Updating after bumping the submodule's pinned tag

```bash
rm -rf apache-datasketches-sys/vendor/datasketches-cpp
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/common
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/hll
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/theta
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/cpc
cp -R vendor/datasketches-cpp/common/include apache-datasketches-sys/vendor/datasketches-cpp/common/include
cp -R vendor/datasketches-cpp/hll/include apache-datasketches-sys/vendor/datasketches-cpp/hll/include
cp -R vendor/datasketches-cpp/theta/include apache-datasketches-sys/vendor/datasketches-cpp/theta/include
cp -R vendor/datasketches-cpp/cpc/include apache-datasketches-sys/vendor/datasketches-cpp/cpc/include
rm apache-datasketches-sys/vendor/datasketches-cpp/common/include/version.hpp.in
cp vendor/datasketches-cpp/LICENSE apache-datasketches-sys/vendor/datasketches-cpp/LICENSE
cp vendor/datasketches-cpp/NOTICE apache-datasketches-sys/vendor/datasketches-cpp/NOTICE
```

When a future sketch family needs headers outside `common/`+`hll/`+
`theta/`+`cpc/`, add its `include/` directory to both this script and
`build.rs`.
```

- [ ] **Step 3: Add the `cpc` feature to `apache-datasketches-sys/Cargo.toml`**

In the `[features]` table, change:

```toml
[features]
default = []
hll = []
theta = []
```

to:

```toml
[features]
default = []
hll = []
theta = []
cpc = []
```

- [ ] **Step 4: Wire `build.rs` to include CPC's headers and compile its shims when the feature is on**

Replace the whole file:

```rust
fn main() {
    let mut bridges: Vec<&str> = Vec::new();

    if cfg!(feature = "hll") {
        bridges.push("src/hll.rs");
    }
    if cfg!(feature = "theta") {
        for path in [
            "src/theta_sketch.rs",
            "src/theta_compact.rs",
            "src/theta_wrapped.rs",
            "src/theta_union.rs",
            "src/theta_intersection.rs",
            "src/theta_a_not_b.rs",
            "src/theta_jaccard.rs",
        ] {
            if std::path::Path::new(path).exists() {
                bridges.push(path);
            }
        }
    }
    if cfg!(feature = "cpc") {
        // Same incremental-availability rationale as theta above: these
        // bridge modules are added incrementally by the CPC sketch
        // family plan's tasks; only reference the ones that exist so far
        // so that `--features cpc` keeps building at every intermediate
        // task.
        for path in ["src/cpc_sketch.rs", "src/cpc_union.rs"] {
            if std::path::Path::new(path).exists() {
                bridges.push(path);
            }
        }
    }

    if bridges.is_empty() {
        return;
    }

    // We build against a copy of the needed datasketches-cpp headers
    // vendored into this crate (`vendor/datasketches-cpp`), not the
    // workspace-root git submodule: crates.io packaging only includes files
    // inside the crate directory, so a path escaping it via `../` would be
    // missing from the published tarball. The workspace-root submodule
    // remains the source of truth for updating the pinned version (see
    // vendor/README.md); this copy is refreshed from it manually.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let vendor_dir = manifest_dir.join("vendor/datasketches-cpp");

    // cxx-build generates each bridge's header at
    // OUT_DIR/cxxbridge/include/<pkg-name>/src/<name>.rs.h, but our shim
    // headers include it as a bare "<name>.rs.h", so we need that directory
    // directly on the include path in addition to cxx_build::bridges'
    // default dirs.
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let generated_header_dir = out_dir
        .join("cxxbridge/include")
        .join(env!("CARGO_PKG_NAME"))
        .join("src");

    let mut build = cxx_build::bridges(&bridges);
    build
        .include(vendor_dir.join("common/include"))
        .include(vendor_dir.join("hll/include"))
        .include(vendor_dir.join("theta/include"))
        .include(vendor_dir.join("cpc/include"))
        .include("cpp")
        .include("cpp/hll")
        .include("cpp/theta")
        .include("cpp/cpc")
        .include(generated_header_dir)
        .flag_if_supported("-std=c++17");

    if cfg!(feature = "hll") {
        build
            .file("cpp/hll/hll_sketch_shim.cc")
            .file("cpp/hll/hll_union_shim.cc");
    }
    if cfg!(feature = "theta") {
        for path in [
            "cpp/theta/theta_sketch_shim.cc",
            "cpp/theta/theta_compact_shim.cc",
            "cpp/theta/theta_wrapped_shim.cc",
            "cpp/theta/theta_union_shim.cc",
            "cpp/theta/theta_intersection_shim.cc",
            "cpp/theta/theta_a_not_b_shim.cc",
            "cpp/theta/theta_jaccard_shim.cc",
        ] {
            if std::path::Path::new(path).exists() {
                build.file(path);
            }
        }
    }
    if cfg!(feature = "cpc") {
        for path in ["cpp/cpc/cpc_sketch_shim.cc", "cpp/cpc/cpc_union_shim.cc"] {
            if std::path::Path::new(path).exists() {
                build.file(path);
            }
        }
    }

    build.compile("apache_datasketches_sys");

    println!("cargo:rerun-if-changed=src/hll.rs");
    println!("cargo:rerun-if-changed=cpp/hll/hll_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/hll/hll_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/hll/hll_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/hll/hll_union_shim.cc");
    println!("cargo:rerun-if-changed=src/theta_sketch.rs");
    println!("cargo:rerun-if-changed=src/theta_compact.rs");
    println!("cargo:rerun-if-changed=src/theta_wrapped.rs");
    println!("cargo:rerun-if-changed=src/theta_union.rs");
    println!("cargo:rerun-if-changed=src/theta_intersection.rs");
    println!("cargo:rerun-if-changed=src/theta_a_not_b.rs");
    println!("cargo:rerun-if-changed=src/theta_jaccard.rs");
    println!("cargo:rerun-if-changed=cpp/theta/theta_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_compact_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_compact_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_wrapped_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_wrapped_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_union_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_intersection_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_intersection_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_a_not_b_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_a_not_b_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_jaccard_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_jaccard_shim.cc");
    println!("cargo:rerun-if-changed=src/cpc_sketch.rs");
    println!("cargo:rerun-if-changed=src/cpc_union.rs");
    println!("cargo:rerun-if-changed=cpp/cpc/cpc_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/cpc/cpc_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/cpc/cpc_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/cpc/cpc_union_shim.cc");
}
```

Note: this references two `src/cpc_*.rs` bridge files and two
`cpp/cpc/*_shim.cc` files that don't exist yet — same "will not compile
with `--features cpc` until later tasks populate them" situation as
Theta's Task 1. Do not build with `--features cpc` yet.

- [ ] **Step 5: Confirm the crate still builds with no features (cpc untouched)**

```bash
cargo build -p apache-datasketches-sys
```

Expected: PASS (cpc feature is off by default, so none of the missing files are referenced).

- [ ] **Step 6: Commit**

```bash
git add apache-datasketches-sys/vendor/datasketches-cpp/cpc apache-datasketches-sys/vendor/README.md apache-datasketches-sys/Cargo.toml apache-datasketches-sys/build.rs
git commit -m "Vendor CPC headers and wire cpc feature into build.rs"
```

---

### Task 2: `CpcSketch` C++ shim (ctor, update overloads, query, serialize/deserialize, `cpc_init`) + cxx bridge + link test

**Files:**
- Create: `apache-datasketches-sys/cpp/cpc/cpc_sketch_shim.h`
- Create: `apache-datasketches-sys/cpp/cpc/cpc_sketch_shim.cc`
- Create: `apache-datasketches-sys/src/cpc_sketch.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Modify: `apache-datasketches-sys/Cargo.toml`
- Test: `apache-datasketches-sys/tests/cpc_sketch_link_test.rs`

**Interfaces:**
- Consumes: `datasketches::cpc_sketch` from `vendor/datasketches-cpp/cpc/include/cpc_sketch.hpp` (verified in the Reference section above).
- Produces: C++ class `apache_datasketches_rs::CpcSketchShim`, free functions `new_cpc_sketch`/`cpc_sketch_deserialize`/`cpc_sketch_max_serialized_size_bytes`/`cpc_init`; Rust bridge `apache_datasketches_sys::cpc_sketch::ffi::{CpcSketchShim, new_cpc_sketch, cpc_sketch_deserialize, cpc_sketch_max_serialized_size_bytes, cpc_init}` plus bridged methods, consumed by Task 3 (safe wrapper) and Task 4 (`CpcUnionShim`'s `update_sketch`/`get_result`, via `CpcSketchShim::inner()`).

Unlike HLL/Theta, this bridge has **no `cxx`-shared enum** to declare
(CPC has no target-type/resize-factor analog), so `cpc_sketch_shim.h`
needs none of the forward-declaration dance those files use — it can
include everything it needs directly.

- [ ] **Step 1: Write `cpc_sketch_shim.h`**

```cpp
#pragma once
#include <cstddef>
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "cpc_sketch.hpp"

namespace apache_datasketches_rs {

class CpcSketchShim {
public:
  explicit CpcSketchShim(uint8_t lg_k);
  explicit CpcSketchShim(datasketches::cpc_sketch sketch);

  void update_u64(uint64_t value);
  void update_i64(int64_t value);
  void update_u32(uint32_t value);
  void update_i32(int32_t value);
  void update_u16(uint16_t value);
  void update_i16(int16_t value);
  void update_u8(uint8_t value);
  void update_i8(int8_t value);
  void update_f64(double value);
  void update_f32(float value);
  void update_str(rust::Str value);
  void update_bytes(rust::Slice<const uint8_t> value);

  bool is_empty() const;
  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  uint8_t get_lg_k() const;
  rust::String to_string_summary() const;

  rust::Vec<uint8_t> serialize() const;

  const datasketches::cpc_sketch& inner() const { return sketch_; }

private:
  datasketches::cpc_sketch sketch_;
};

std::unique_ptr<CpcSketchShim> new_cpc_sketch(uint8_t lg_k);
std::unique_ptr<CpcSketchShim> cpc_sketch_deserialize(rust::Slice<const uint8_t> bytes);
size_t cpc_sketch_max_serialized_size_bytes(uint8_t lg_k);
void cpc_init();

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Write `cpc_sketch_shim.cc`**

```cpp
#include "cpc_sketch_shim.h"

namespace apache_datasketches_rs {

CpcSketchShim::CpcSketchShim(uint8_t lg_k) : sketch_(lg_k) {}

CpcSketchShim::CpcSketchShim(datasketches::cpc_sketch sketch)
  : sketch_(std::move(sketch)) {}

void CpcSketchShim::update_u64(uint64_t value) { sketch_.update(value); }
void CpcSketchShim::update_i64(int64_t value) { sketch_.update(value); }
void CpcSketchShim::update_u32(uint32_t value) { sketch_.update(value); }
void CpcSketchShim::update_i32(int32_t value) { sketch_.update(value); }
void CpcSketchShim::update_u16(uint16_t value) { sketch_.update(value); }
void CpcSketchShim::update_i16(int16_t value) { sketch_.update(value); }
void CpcSketchShim::update_u8(uint8_t value) { sketch_.update(value); }
void CpcSketchShim::update_i8(int8_t value) { sketch_.update(value); }
void CpcSketchShim::update_f64(double value) { sketch_.update(value); }
void CpcSketchShim::update_f32(float value) { sketch_.update(value); }
void CpcSketchShim::update_str(rust::Str value) {
  sketch_.update(std::string(value));
}
void CpcSketchShim::update_bytes(rust::Slice<const uint8_t> value) {
  sketch_.update(value.data(), value.size());
}

bool CpcSketchShim::is_empty() const { return sketch_.is_empty(); }
double CpcSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double CpcSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double CpcSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
uint8_t CpcSketchShim::get_lg_k() const { return sketch_.get_lg_k(); }

rust::String CpcSketchShim::to_string_summary() const {
  return rust::String(std::string(sketch_.to_string().c_str()));
}

rust::Vec<uint8_t> CpcSketchShim::serialize() const {
  auto bytes = sketch_.serialize();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

std::unique_ptr<CpcSketchShim> new_cpc_sketch(uint8_t lg_k) {
  return std::make_unique<CpcSketchShim>(lg_k);
}

std::unique_ptr<CpcSketchShim> cpc_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<CpcSketchShim>(
      datasketches::cpc_sketch::deserialize(bytes.data(), bytes.size()));
}

size_t cpc_sketch_max_serialized_size_bytes(uint8_t lg_k) {
  return datasketches::cpc_sketch::get_max_serialized_size_bytes(lg_k);
}

void cpc_init() {
  datasketches::cpc_init<std::allocator<uint8_t>>();
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Write `apache-datasketches-sys/src/cpc_sketch.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("cpc_sketch_shim.h");

        type CpcSketchShim;

        fn new_cpc_sketch(lg_k: u8) -> Result<UniquePtr<CpcSketchShim>>;
        fn cpc_sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<CpcSketchShim>>;
        fn cpc_sketch_max_serialized_size_bytes(lg_k: u8) -> usize;
        fn cpc_init();

        fn update_u64(self: Pin<&mut CpcSketchShim>, value: u64);
        fn update_i64(self: Pin<&mut CpcSketchShim>, value: i64);
        fn update_u32(self: Pin<&mut CpcSketchShim>, value: u32);
        fn update_i32(self: Pin<&mut CpcSketchShim>, value: i32);
        fn update_u16(self: Pin<&mut CpcSketchShim>, value: u16);
        fn update_i16(self: Pin<&mut CpcSketchShim>, value: i16);
        fn update_u8(self: Pin<&mut CpcSketchShim>, value: u8);
        fn update_i8(self: Pin<&mut CpcSketchShim>, value: i8);
        fn update_f64(self: Pin<&mut CpcSketchShim>, value: f64);
        fn update_f32(self: Pin<&mut CpcSketchShim>, value: f32);
        fn update_str(self: Pin<&mut CpcSketchShim>, value: &str);
        fn update_bytes(self: Pin<&mut CpcSketchShim>, value: &[u8]);

        fn is_empty(self: &CpcSketchShim) -> bool;
        fn get_estimate(self: &CpcSketchShim) -> f64;
        fn get_lower_bound(self: &CpcSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &CpcSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_lg_k(self: &CpcSketchShim) -> u8;
        fn to_string_summary(self: &CpcSketchShim) -> String;

        fn serialize(self: &CpcSketchShim) -> Vec<u8>;
    }
}
```

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

Add after the `theta` block:

```rust
#[cfg(feature = "cpc")]
pub mod cpc_sketch;
```

- [ ] **Step 5: Add the link-test entry to `apache-datasketches-sys/Cargo.toml`**

Append:

```toml
[[test]]
name = "cpc_sketch_link_test"
required-features = ["cpc"]
```

- [ ] **Step 6: Build with the `cpc` feature to confirm the shim and bridge compile and link**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features cpc
```

Expected: successful build.

- [ ] **Step 7: Write a link-level smoke test**

```rust
// apache-datasketches-sys/tests/cpc_sketch_link_test.rs
#![cfg(feature = "cpc")]

use apache_datasketches_sys::cpc_sketch::ffi;

#[test]
fn construct_update_estimate_roundtrip() {
    let mut sketch = ffi::new_cpc_sketch(11).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 30.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = ffi::new_cpc_sketch(3);
    assert!(result.is_err());
}
```

- [ ] **Step 8: Run the smoke test**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features cpc
```

Expected: both tests PASS.

- [ ] **Step 9: Commit**

```bash
git add apache-datasketches-sys/cpp/cpc/cpc_sketch_shim.h apache-datasketches-sys/cpp/cpc/cpc_sketch_shim.cc apache-datasketches-sys/src/cpc_sketch.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/Cargo.toml apache-datasketches-sys/tests/cpc_sketch_link_test.rs
git commit -m "Add CpcSketch C++ shim, cxx bridge, and link smoke test"
```

---

### Task 3: Safe `CpcSketchBuilder` and `CpcSketch` wrapper + `cpc::init()` + smoke test

**Files:**
- Create: `apache-datasketches/src/cpc/mod.rs`
- Create: `apache-datasketches/src/cpc/builder.rs`
- Create: `apache-datasketches/src/cpc/sketch.rs`
- Create: `apache-datasketches/src/cpc/init.rs`
- Modify: `apache-datasketches/src/lib.rs`
- Modify: `apache-datasketches/Cargo.toml`
- Test: `apache-datasketches/tests/cpc_sketch_smoke_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches_sys::cpc_sketch::ffi::{CpcSketchShim, new_cpc_sketch, cpc_sketch_deserialize, cpc_sketch_max_serialized_size_bytes, cpc_init}` (Task 2).
- Produces: `apache_datasketches::cpc::{CpcSketchBuilder, CpcSketch, get_max_serialized_size_bytes, init}`, consumed by Task 5 (`CpcUnion::update`, via `CpcSketch`'s `pub(crate) inner` field) and Tasks 6-8 (tests).

- [ ] **Step 1: Write `apache-datasketches/src/cpc/builder.rs`**

```rust
use crate::error::SketchError;

/// Builder for [`crate::cpc::CpcSketch`], mirroring upstream's
/// `cpc_sketch_alloc` constructor. `lg_k` defaults to `11`
/// (`cpc_constants::DEFAULT_LG_K`). The seed is never exposed — every
/// sketch built by this crate always uses upstream's `DEFAULT_SEED`.
#[derive(Debug, Clone, Copy)]
pub struct CpcSketchBuilder {
    lg_k: u8,
}

impl Default for CpcSketchBuilder {
    fn default() -> Self {
        Self { lg_k: 11 }
    }
}

impl CpcSketchBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    pub fn build(self) -> Result<super::CpcSketch, SketchError> {
        super::CpcSketch::from_lg_k(self.lg_k)
    }
}
```

- [ ] **Step 2: Write `apache-datasketches/src/cpc/sketch.rs`**

```rust
use crate::error::SketchError;
use apache_datasketches_sys::cpc_sketch::ffi as sys;
use cxx::UniquePtr;

pub struct CpcSketch {
    pub(crate) inner: UniquePtr<sys::CpcSketchShim>,
}

unsafe impl Send for CpcSketch {}

impl CpcSketch {
    pub(crate) fn from_lg_k(lg_k: u8) -> Result<Self, SketchError> {
        let inner = sys::new_cpc_sketch(lg_k)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::cpc_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn update_u64(&mut self, value: u64) {
        self.inner.pin_mut().update_u64(value);
    }

    pub fn update_i64(&mut self, value: i64) {
        self.inner.pin_mut().update_i64(value);
    }

    pub fn update_u32(&mut self, value: u32) {
        self.inner.pin_mut().update_u32(value);
    }

    pub fn update_i32(&mut self, value: i32) {
        self.inner.pin_mut().update_i32(value);
    }

    pub fn update_u16(&mut self, value: u16) {
        self.inner.pin_mut().update_u16(value);
    }

    pub fn update_i16(&mut self, value: i16) {
        self.inner.pin_mut().update_i16(value);
    }

    pub fn update_u8(&mut self, value: u8) {
        self.inner.pin_mut().update_u8(value);
    }

    pub fn update_i8(&mut self, value: i8) {
        self.inner.pin_mut().update_i8(value);
    }

    pub fn update_f64(&mut self, value: f64) {
        self.inner.pin_mut().update_f64(value);
    }

    pub fn update_f32(&mut self, value: f32) {
        self.inner.pin_mut().update_f32(value);
    }

    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    pub fn get_lg_k(&self) -> u8 {
        self.inner.get_lg_k()
    }

    pub fn to_string_summary(&self) -> String {
        self.inner.to_string_summary()
    }

    pub fn serialize(&self) -> Vec<u8> {
        self.inner.serialize()
    }
}

/// The estimated maximum compressed serialized size, in bytes, of a CPC
/// sketch built with the given `lg_k`. Useful for pre-allocating buffers.
pub fn get_max_serialized_size_bytes(lg_k: u8) -> usize {
    sys::cpc_sketch_max_serialized_size_bytes(lg_k)
}
```

- [ ] **Step 3: Write `apache-datasketches/src/cpc/init.rs`**

```rust
use apache_datasketches_sys::cpc_sketch::ffi as sys;

/// Eagerly initializes CPC's global decompression tables, used during
/// serialization/deserialization.
///
/// Upstream lazily self-initializes these tables on first use, and that
/// lazy path is **not thread-safe**: if two threads race to serialize or
/// deserialize a [`crate::cpc::CpcSketch`] for the first time concurrently,
/// initializing the shared global state is a data race. Call `init()` once,
/// single-threaded, before spawning worker threads that will serialize or
/// deserialize CPC sketches concurrently. Single-threaded callers never
/// need to call this — the lazy self-init is fine there.
pub fn init() {
    sys::cpc_init();
}
```

- [ ] **Step 4: Write `apache-datasketches/src/cpc/mod.rs`**

```rust
mod builder;
mod init;
mod sketch;

pub use builder::CpcSketchBuilder;
pub use init::init;
pub use sketch::{get_max_serialized_size_bytes, CpcSketch};
```

- [ ] **Step 5: Declare the module in `apache-datasketches/src/lib.rs`**

Add after the `theta` block:

```rust
#[cfg(feature = "cpc")]
pub mod cpc;
```

- [ ] **Step 6: Add the `cpc` feature and test entry to `apache-datasketches/Cargo.toml`**

In `[features]`, change:

```toml
[features]
default = []
hll = ["apache-datasketches-sys/hll"]
theta = ["apache-datasketches-sys/theta"]
```

to:

```toml
[features]
default = []
hll = ["apache-datasketches-sys/hll"]
theta = ["apache-datasketches-sys/theta"]
cpc = ["apache-datasketches-sys/cpc"]
```

Append:

```toml
[[test]]
name = "cpc_sketch_smoke_test"
required-features = ["cpc"]
```

- [ ] **Step 7: Build with the `cpc` feature**

```bash
cargo build -p apache-datasketches --no-default-features --features cpc
```

Expected: successful build.

- [ ] **Step 8: Write the smoke test**

```rust
// apache-datasketches/tests/cpc_sketch_smoke_test.rs
use apache_datasketches::cpc::CpcSketchBuilder;

#[test]
fn construct_update_estimate() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 30.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_is_err() {
    assert!(CpcSketchBuilder::new().lg_k(3).build().is_err());
}
```

- [ ] **Step 9: Run the smoke test**

```bash
cargo test -p apache-datasketches --no-default-features --features cpc --test cpc_sketch_smoke_test
```

Expected: both tests PASS.

- [ ] **Step 10: Commit**

```bash
git add apache-datasketches/src/cpc apache-datasketches/src/lib.rs apache-datasketches/Cargo.toml apache-datasketches/tests/cpc_sketch_smoke_test.rs
git commit -m "Add safe CpcSketchBuilder, CpcSketch wrapper, and cpc::init()"
```

---

### Task 4: `CpcUnion` C++ shim (ctor, `update`, `get_result`) + cxx bridge + link test

**Files:**
- Create: `apache-datasketches-sys/cpp/cpc/cpc_union_shim.h`
- Create: `apache-datasketches-sys/cpp/cpc/cpc_union_shim.cc`
- Create: `apache-datasketches-sys/src/cpc_union.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Modify: `apache-datasketches-sys/Cargo.toml`
- Test: `apache-datasketches-sys/tests/cpc_union_link_test.rs`

**Interfaces:**
- Consumes: `datasketches::cpc_union` from `vendor/datasketches-cpp/cpc/include/cpc_union.hpp`; `CpcSketchShim` (Task 2, cross-module reuse via `type CpcSketchShim = crate::cpc_sketch::ffi::CpcSketchShim;`, the same pattern Theta's `theta_union.rs` uses for `ThetaSketchShim`).
- Produces: C++ class `apache_datasketches_rs::CpcUnionShim`, free function `new_cpc_union`; Rust bridge `apache_datasketches_sys::cpc_union::ffi::{CpcUnionShim, new_cpc_union}` plus bridged methods, consumed by Task 5 (safe wrapper).

- [ ] **Step 1: Write `cpc_union_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "cpc_union.hpp"
#include "cpc_sketch_shim.h"

namespace apache_datasketches_rs {

class CpcUnionShim {
public:
  explicit CpcUnionShim(uint8_t lg_k);

  void update_sketch(const CpcSketchShim& sketch);

  std::unique_ptr<CpcSketchShim> get_result() const;

private:
  datasketches::cpc_union u_;
};

std::unique_ptr<CpcUnionShim> new_cpc_union(uint8_t lg_k);

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Write `cpc_union_shim.cc`**

```cpp
#include "cpc_union_shim.h"

namespace apache_datasketches_rs {

CpcUnionShim::CpcUnionShim(uint8_t lg_k) : u_(lg_k) {}

void CpcUnionShim::update_sketch(const CpcSketchShim& sketch) {
  u_.update(sketch.inner());
}

std::unique_ptr<CpcSketchShim> CpcUnionShim::get_result() const {
  return std::make_unique<CpcSketchShim>(u_.get_result());
}

std::unique_ptr<CpcUnionShim> new_cpc_union(uint8_t lg_k) {
  return std::make_unique<CpcUnionShim>(lg_k);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Write `apache-datasketches-sys/src/cpc_union.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("cpc_sketch_shim.h");
        include!("cpc_union_shim.h");

        type CpcSketchShim = crate::cpc_sketch::ffi::CpcSketchShim;

        type CpcUnionShim;

        fn new_cpc_union(lg_k: u8) -> Result<UniquePtr<CpcUnionShim>>;

        fn update_sketch(self: Pin<&mut CpcUnionShim>, sketch: &CpcSketchShim);

        fn get_result(self: &CpcUnionShim) -> UniquePtr<CpcSketchShim>;
    }
}
```

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

Add after the `cpc_sketch` line:

```rust
#[cfg(feature = "cpc")]
pub mod cpc_union;
```

- [ ] **Step 5: Add the link-test entry to `apache-datasketches-sys/Cargo.toml`**

Append:

```toml
[[test]]
name = "cpc_union_link_test"
required-features = ["cpc"]
```

- [ ] **Step 6: Build with the `cpc` feature**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features cpc
```

Expected: successful build.

- [ ] **Step 7: Write a link-level smoke test**

```rust
// apache-datasketches-sys/tests/cpc_union_link_test.rs
#![cfg(feature = "cpc")]

use apache_datasketches_sys::{cpc_sketch::ffi as sketch_ffi, cpc_union::ffi as union_ffi};

#[test]
fn union_of_two_sketches_merges_estimate() {
    let mut a = sketch_ffi::new_cpc_sketch(11).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    let mut b = sketch_ffi::new_cpc_sketch(11).unwrap();
    for i in 500..1000u64 {
        b.pin_mut().update_u64(i);
    }

    let mut u = union_ffi::new_cpc_union(11).unwrap();
    u.pin_mut().update_sketch(&a);
    u.pin_mut().update_sketch(&b);

    let result = u.get_result();
    let estimate = result.get_estimate();
    assert!((estimate - 1000.0).abs() < 30.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = union_ffi::new_cpc_union(3);
    assert!(result.is_err());
}
```

- [ ] **Step 8: Run the smoke test**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features cpc
```

Expected: all 4 tests (2 from Task 2, 2 new) PASS.

- [ ] **Step 9: Commit**

```bash
git add apache-datasketches-sys/cpp/cpc/cpc_union_shim.h apache-datasketches-sys/cpp/cpc/cpc_union_shim.cc apache-datasketches-sys/src/cpc_union.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/Cargo.toml apache-datasketches-sys/tests/cpc_union_link_test.rs
git commit -m "Add CpcUnion C++ shim, cxx bridge, and link smoke test"
```

---

### Task 5: Safe `CpcUnionBuilder` and `CpcUnion` wrapper + smoke test

**Files:**
- Create: `apache-datasketches/src/cpc/union.rs`
- Modify: `apache-datasketches/src/cpc/builder.rs`
- Modify: `apache-datasketches/src/cpc/mod.rs`
- Modify: `apache-datasketches/Cargo.toml`
- Test: `apache-datasketches/tests/cpc_union_smoke_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches_sys::cpc_union::ffi::{CpcUnionShim, new_cpc_union}` (Task 4); `CpcSketch` (Task 3, via its `pub(crate) inner` field, same-crate access).
- Produces: `apache_datasketches::cpc::{CpcUnionBuilder, CpcUnion}`, consumed by Tasks 7-8 (tests).

- [ ] **Step 1: Add `CpcUnionBuilder` to `apache-datasketches/src/cpc/builder.rs`**

Append to the existing file:

```rust
/// Builder for [`crate::cpc::CpcUnion`], mirroring upstream's
/// `cpc_union_alloc` constructor. `lg_k` defaults to `11`
/// (`cpc_constants::DEFAULT_LG_K`). The seed is never exposed, same as
/// [`CpcSketchBuilder`].
#[derive(Debug, Clone, Copy)]
pub struct CpcUnionBuilder {
    lg_k: u8,
}

impl Default for CpcUnionBuilder {
    fn default() -> Self {
        Self { lg_k: 11 }
    }
}

impl CpcUnionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    pub fn build(self) -> Result<super::CpcUnion, SketchError> {
        super::CpcUnion::from_lg_k(self.lg_k)
    }
}
```

- [ ] **Step 2: Write `apache-datasketches/src/cpc/union.rs`**

```rust
use crate::cpc::sketch::CpcSketch;
use crate::error::SketchError;
use apache_datasketches_sys::cpc_union::ffi as sys;
use cxx::UniquePtr;

pub struct CpcUnion {
    inner: UniquePtr<sys::CpcUnionShim>,
}

unsafe impl Send for CpcUnion {}

impl CpcUnion {
    pub(crate) fn from_lg_k(lg_k: u8) -> Result<Self, SketchError> {
        let inner = sys::new_cpc_union(lg_k)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn update(&mut self, sketch: &CpcSketch) {
        self.inner.pin_mut().update_sketch(&sketch.inner);
    }

    pub fn get_result(&self) -> CpcSketch {
        let inner = self.inner.get_result();
        CpcSketch { inner }
    }
}
```

Note: `sketch.rs`'s `mod sketch;` is currently declared as a private
submodule of `cpc/mod.rs` (Step 4 of Task 3 wrote `mod sketch;` without
`pub`), so `crate::cpc::sketch::CpcSketch` is reachable from
`crate::cpc::union` (sibling submodule, same parent) but not from outside
the crate — the crate's actual public path is `crate::cpc::CpcSketch` via
the `pub use sketch::{..., CpcSketch};` re-export already in place.

- [ ] **Step 3: Update `apache-datasketches/src/cpc/mod.rs`**

Replace the whole file:

```rust
mod builder;
mod init;
mod sketch;
mod union;

pub use builder::{CpcSketchBuilder, CpcUnionBuilder};
pub use init::init;
pub use sketch::{get_max_serialized_size_bytes, CpcSketch};
pub use union::CpcUnion;
```

- [ ] **Step 4: Add the test entry to `apache-datasketches/Cargo.toml`**

Append:

```toml
[[test]]
name = "cpc_union_smoke_test"
required-features = ["cpc"]
```

- [ ] **Step 5: Build with the `cpc` feature**

```bash
cargo build -p apache-datasketches --no-default-features --features cpc
```

Expected: successful build.

- [ ] **Step 6: Write the smoke test**

```rust
// apache-datasketches/tests/cpc_union_smoke_test.rs
use apache_datasketches::cpc::{CpcSketchBuilder, CpcUnionBuilder};

#[test]
fn union_merges_two_sketches() {
    let mut a = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    let mut b = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 500..1000u64 {
        b.update_u64(i);
    }

    let mut union = CpcUnionBuilder::new().lg_k(11).build().unwrap();
    union.update(&a);
    union.update(&b);

    let result = union.get_result();
    let estimate = result.get_estimate();
    assert!((estimate - 1000.0).abs() < 30.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_is_err() {
    assert!(CpcUnionBuilder::new().lg_k(3).build().is_err());
}
```

- [ ] **Step 7: Run the smoke test**

```bash
cargo test -p apache-datasketches --no-default-features --features cpc --test cpc_union_smoke_test
```

Expected: both tests PASS.

- [ ] **Step 8: Commit**

```bash
git add apache-datasketches/src/cpc apache-datasketches/Cargo.toml apache-datasketches/tests/cpc_union_smoke_test.rs
git commit -m "Add safe CpcUnionBuilder and CpcUnion wrapper"
```

---

### Task 6: Port `cpc_sketch_test.cpp` (15 of 26 upstream cases)

**Files:**
- Test: `apache-datasketches/tests/cpc_sketch_test.rs`
- Modify: `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: `CpcSketch`, `CpcSketchBuilder`, `get_max_serialized_size_bytes` (Task 3).
- Produces: no new production code — completes the 1:1 test-porting obligation for `cpc_sketch_test.cpp` per the Test Inventory section above.

- [ ] **Step 1: Write `cpc_sketch_test.rs`**

```rust
// apache-datasketches/tests/cpc_sketch_test.rs
//! Ported from cpc/test/cpc_sketch_test.cpp (tag 5.2.0). 15 of 26 upstream
//! cases ported; 11 excluded:
//! - "overflow bug" (100,000,000 updates) — impractically slow for a
//!   routine local test suite; the bug it regression-tests is a specific
//!   historical fix, not a public-API behavior distinct from the
//!   already-ported "many values"/large-scale tests.
//! - "serialize deserialize {empty,sparse,hybrid,pinned,sliding}" (the
//!   ostream-based variants) — duplicates of the byte-vector-based
//!   "..., bytes" variants below, which this crate ports instead since its
//!   public API only exposes the byte-vector serialize()/deserialize()
//!   pair (no ostream overload).
//! - "serializing deserialize sliding large" (ostream-only, n=3,000,000) —
//!   redundant with the "sliding" tier already covered at a smaller,
//!   faster n; the "sliding huge" case below covers the large-scale path.
//! - "copy" — tests C++ copy-constructor/assignment semantics; this
//!   crate's `CpcSketch` doesn't implement `Clone`.
//! - "serialize deserialize empty, custom seed" — no seed parameter is
//!   exposed in this crate's public API (every sketch uses upstream's
//!   `DEFAULT_SEED`).
//! - "validate fail" — `validate()` is not exposed (marked `@private`
//!   upstream, for internal debugging use only).
//! - "serialize both ways" — exercises the `header_size_bytes` parameter
//!   on `serialize()`, which this crate doesn't expose (PostgreSQL
//!   extension-specific, no use case here).
use apache_datasketches::cpc::{get_max_serialized_size_bytes, CpcSketch, CpcSketchBuilder};

const RELATIVE_ERROR_FOR_LG_K_11: f64 = 0.02;

#[test]
fn lg_k_limits() {
    assert!(CpcSketchBuilder::new().lg_k(4).build().is_ok());
    assert!(CpcSketchBuilder::new().lg_k(26).build().is_ok());
    assert!(CpcSketchBuilder::new().lg_k(3).build().is_err());
    assert!(CpcSketchBuilder::new().lg_k(27).build().is_err());
}

#[test]
fn empty() {
    let sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_estimate(), 0.0);
    assert_eq!(sketch.get_lower_bound(1).unwrap(), 0.0);
    assert_eq!(sketch.get_upper_bound(1).unwrap(), 0.0);
}

#[test]
fn one_value() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_u64(1);
    assert!(!sketch.is_empty());
    let estimate = sketch.get_estimate();
    assert!((estimate - 1.0).abs() < RELATIVE_ERROR_FOR_LG_K_11);
    assert!(estimate >= sketch.get_lower_bound(1).unwrap());
    assert!(estimate <= sketch.get_upper_bound(1).unwrap());
}

#[test]
fn many_values() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 10_000u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    assert!(!sketch.is_empty());
    let estimate = sketch.get_estimate();
    assert!((estimate - n as f64).abs() < n as f64 * RELATIVE_ERROR_FOR_LG_K_11);
    assert!(estimate >= sketch.get_lower_bound(1).unwrap());
    assert!(estimate <= sketch.get_upper_bound(1).unwrap());
}

fn round_trip(sketch: &CpcSketch) -> CpcSketch {
    let bytes = sketch.serialize();
    CpcSketch::deserialize(&bytes).unwrap()
}

#[test]
fn serialize_deserialize_empty() {
    let sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let bytes = sketch.serialize();
    let deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());
}

#[test]
fn serialize_deserialize_sparse() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 100u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    // updating again with the same values should not change the sketch
    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn serialize_deserialize_hybrid() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 200u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn serialize_deserialize_pinned() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 2_000u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn serialize_deserialize_sliding() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 20_000u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn serialize_deserialize_sliding_huge() {
    let mut sketch = CpcSketchBuilder::new().lg_k(26).build().unwrap();
    let n = 10_000_000u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - n as f64).abs() < n as f64 * 0.001);
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn kappa_range() {
    let sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    assert_eq!(sketch.get_lower_bound(1).unwrap(), 0.0);
    assert_eq!(sketch.get_upper_bound(1).unwrap(), 0.0);
    assert_eq!(sketch.get_lower_bound(2).unwrap(), 0.0);
    assert_eq!(sketch.get_upper_bound(2).unwrap(), 0.0);
    assert_eq!(sketch.get_lower_bound(3).unwrap(), 0.0);
    assert_eq!(sketch.get_upper_bound(3).unwrap(), 0.0);
    assert!(sketch.get_lower_bound(4).is_err());
    assert!(sketch.get_upper_bound(4).is_err());
}

#[test]
fn update_int_equivalence() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_u64(u64::MAX);
    sketch.update_i64(-1);
    sketch.update_u32(u32::MAX);
    sketch.update_i32(-1);
    sketch.update_u16(u16::MAX);
    sketch.update_i16(-1);
    sketch.update_u8(u8::MAX);
    sketch.update_i8(-1);
    let estimate = sketch.get_estimate();
    assert!((estimate - 1.0).abs() < RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn update_float_equivalence() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_f32(1.0);
    sketch.update_f64(1.0);
    let estimate = sketch.get_estimate();
    assert!((estimate - 1.0).abs() < RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn update_string_equivalence() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_str("a");
    sketch.update_bytes(b"a");
    let estimate = sketch.get_estimate();
    assert!((estimate - 1.0).abs() < RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn max_serialized_size() {
    assert_eq!(get_max_serialized_size_bytes(4), 24 + 40);
    assert_eq!(
        get_max_serialized_size_bytes(26),
        ((0.6 * (1u64 << 26) as f64) as usize) + 40
    );
}
```

- [ ] **Step 2: Add the test entry to `apache-datasketches/Cargo.toml`**

Append:

```toml
[[test]]
name = "cpc_sketch_test"
required-features = ["cpc"]
```

- [ ] **Step 3: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features cpc --test cpc_sketch_test
```

Expected: all 15 PASS. (`serialize_deserialize_sliding_huge` runs 10,000,000 updates — expect it to take a few seconds, not minutes; if it hangs, something is wrong, not just slow.)

- [ ] **Step 4: Commit**

```bash
git add apache-datasketches/tests/cpc_sketch_test.rs apache-datasketches/Cargo.toml
git commit -m "Port cpc_sketch_test.cpp"
```

---

### Task 7: Port `cpc_union_test.cpp` (6 of 9 upstream cases)

**Files:**
- Test: `apache-datasketches/tests/cpc_union_test.rs`
- Modify: `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: `CpcSketch`, `CpcSketchBuilder`, `CpcUnion`, `CpcUnionBuilder` (Tasks 3, 5).
- Produces: no new production code — completes the 1:1 test-porting obligation for `cpc_union_test.cpp` per the Test Inventory section above.

- [ ] **Step 1: Write `cpc_union_test.rs`**

```rust
// apache-datasketches/tests/cpc_union_test.rs
//! Ported from cpc/test/cpc_union_test.cpp (tag 5.2.0). 6 of 9 upstream
//! cases ported; 3 excluded:
//! - "copy" — tests C++ copy-constructor/assignment semantics; this
//!   crate's `CpcUnion` doesn't implement `Clone`.
//! - "custom seed" — no seed parameter is exposed in this crate's public
//!   API.
//! - "moving update" — exercises a C++-specific move-constructor update
//!   overload (`update(cpc_sketch_alloc&&)`) purely as a copy-avoidance
//!   optimization; behaviorally identical to updating via a reference,
//!   which every other ported case already exercises through
//!   `CpcUnion::update(&CpcSketch)`.
//!
//! "large" is adapted: upstream additionally asserts
//! `r.get_num_coupons() == s.get_num_coupons()`, but `get_num_coupons()` is
//! not exposed (marked `@private` upstream, for internal debugging use
//! only) — the estimate-comparison assertion below is kept.
use apache_datasketches::cpc::{CpcSketchBuilder, CpcUnionBuilder};

const RELATIVE_ERROR_FOR_LG_K_11: f64 = 0.02;

#[test]
fn lg_k_limits() {
    assert!(CpcUnionBuilder::new().lg_k(4).build().is_ok());
    assert!(CpcUnionBuilder::new().lg_k(26).build().is_ok());
    assert!(CpcUnionBuilder::new().lg_k(3).build().is_err());
    assert!(CpcUnionBuilder::new().lg_k(27).build().is_err());
}

#[test]
fn empty() {
    let union = CpcUnionBuilder::new().lg_k(11).build().unwrap();
    let result = union.get_result();
    assert!(result.is_empty());
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn large() {
    let mut s = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let mut union = CpcUnionBuilder::new().lg_k(11).build().unwrap();
    let mut key = 0u64;
    for _ in 0..1000 {
        let mut tmp = CpcSketchBuilder::new().lg_k(11).build().unwrap();
        for _ in 0..10_000 {
            s.update_u64(key);
            tmp.update_u64(key);
            key += 1;
        }
        union.update(&tmp);
    }
    let r = union.get_result();
    let expected = s.get_estimate();
    assert!((r.get_estimate() - expected).abs() < expected * RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn reduce_k_empty() {
    let mut s = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..10_000u64 {
        s.update_u64(i);
    }
    let mut union = CpcUnionBuilder::new().lg_k(12).build().unwrap();
    union.update(&s);
    let r = union.get_result();
    assert_eq!(r.get_lg_k(), 11);
    let estimate = r.get_estimate();
    assert!((estimate - 10_000.0).abs() < 10_000.0 * RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn reduce_k_sparse() {
    let mut union = CpcUnionBuilder::new().lg_k(12).build().unwrap();

    let mut s12 = CpcSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..100u64 {
        s12.update_u64(i);
    }
    union.update(&s12);

    let mut s11 = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..1000u64 {
        s11.update_u64(i);
    }
    union.update(&s11);

    let r = union.get_result();
    assert_eq!(r.get_lg_k(), 11);
    let estimate = r.get_estimate();
    assert!((estimate - 1000.0).abs() < 1000.0 * RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn reduce_k_window() {
    let mut union = CpcUnionBuilder::new().lg_k(12).build().unwrap();

    let mut s12 = CpcSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        s12.update_u64(i);
    }
    union.update(&s12);

    let mut s11 = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..1000u64 {
        s11.update_u64(i);
    }
    union.update(&s11);

    let r = union.get_result();
    assert_eq!(r.get_lg_k(), 11);
    let estimate = r.get_estimate();
    assert!((estimate - 1000.0).abs() < 1000.0 * RELATIVE_ERROR_FOR_LG_K_11);
}
```

- [ ] **Step 2: Add the test entry to `apache-datasketches/Cargo.toml`**

Append:

```toml
[[test]]
name = "cpc_union_test"
required-features = ["cpc"]
```

- [ ] **Step 3: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features cpc --test cpc_union_test
```

Expected: all 6 PASS. (`large` runs 10,000,000 updates total across `s`/`tmp` — expect a few seconds, not minutes.)

- [ ] **Step 4: Commit**

```bash
git add apache-datasketches/tests/cpc_union_test.rs apache-datasketches/Cargo.toml
git commit -m "Port cpc_union_test.cpp"
```

---

### Task 8: New Rust-specific tests — `Send` verification + `cpc::init()` concurrent-use scenario

**Files:**
- Test: `apache-datasketches/tests/cpc_concurrency_test.rs`
- Modify: `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: `CpcSketch`, `CpcSketchBuilder`, `CpcUnion`, `cpc::init` (Tasks 3, 5).
- Produces: no new production code — new, non-upstream test coverage for this plan's `Send` convention (matching the HLL/Theta precedent's `concurrency_test.rs`) and the `cpc::init()` hazard described in the design.

- [ ] **Step 1: Write `cpc_concurrency_test.rs`**

```rust
// apache-datasketches/tests/cpc_concurrency_test.rs
//! New, non-upstream tests: `Send` verification (matching this plan's
//! HLL/Theta precedent) and a concurrent-use scenario for `cpc::init()`,
//! which addresses CPC's global-decompression-table initialization
//! hazard documented in `apache_datasketches::cpc::init`.
use apache_datasketches::cpc::{self, CpcSketch, CpcSketchBuilder, CpcUnion};
use std::thread;

fn assert_send<T: Send>() {}

#[test]
fn cpc_sketch_is_send() {
    assert_send::<CpcSketch>();
}

#[test]
fn cpc_union_is_send() {
    assert_send::<CpcUnion>();
}

#[test]
fn cpc_sketch_moves_across_thread_boundary() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..50u64 {
        sketch.update_u64(i);
    }

    let handle = thread::spawn(move || sketch.get_estimate());
    let estimate = handle.join().unwrap();
    assert!((estimate - 50.0).abs() < 10.0);
}

#[test]
fn init_then_concurrent_serialize_deserialize() {
    // cpc::init() must be called single-threaded before any concurrent
    // first-use of serialize/deserialize, since upstream's lazy
    // self-initialization of the global decompression tables is not
    // thread-safe. Calling it here, before spawning, avoids the race.
    cpc::init();

    let handles: Vec<_> = (0..8u64)
        .map(|t| {
            thread::spawn(move || {
                let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
                for i in 0..1000u64 {
                    sketch.update_u64(i + t * 1000);
                }
                let bytes = sketch.serialize();
                let restored = CpcSketch::deserialize(&bytes).unwrap();
                assert_eq!(sketch.get_estimate(), restored.get_estimate());
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
```

- [ ] **Step 2: Add the test entry to `apache-datasketches/Cargo.toml`**

Append:

```toml
[[test]]
name = "cpc_concurrency_test"
required-features = ["cpc"]
```

- [ ] **Step 3: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features cpc --test cpc_concurrency_test
```

Expected: all 4 PASS.

- [ ] **Step 4: Commit**

```bash
git add apache-datasketches/tests/cpc_concurrency_test.rs apache-datasketches/Cargo.toml
git commit -m "Add Send verification and cpc::init() concurrent-use tests"
```

---

### Task 9: README updates + `cpc.rs` example + final full test matrix

**Files:**
- Modify: `apache-datasketches-sys/README.md`
- Modify: `apache-datasketches/README.md`
- Modify: root `README.md`
- Create: `apache-datasketches/examples/cpc.rs`
- Modify: `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: everything built in Tasks 1-8.
- Produces: the finished, feature-complete CPC sketch family, with documentation brought up to date across all 3 READMEs and a runnable example, per the repo's established "keep READMEs in sync" convention.

- [ ] **Step 1: Update `apache-datasketches-sys/README.md`'s Features section**

In the `## Features` section, change:

```markdown
- `hll` — raw bridge to the HLL (HyperLogLog) sketch and union C++ types.
- `theta` — raw bridge to the Theta sketch, union, intersection, a-not-b,
  and Jaccard similarity C++ types.
```

to:

```markdown
- `hll` — raw bridge to the HLL (HyperLogLog) sketch and union C++ types.
- `theta` — raw bridge to the Theta sketch, union, intersection, a-not-b,
  and Jaccard similarity C++ types.
- `cpc` — raw bridge to the CPC (Compressed Probabilistic Counting) sketch
  and union C++ types.
```

and add a fourth `Cargo.toml` snippet after the existing three:

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["cpc"] }
```

- [ ] **Step 2: Update `apache-datasketches/README.md`**

Add a new section after `## Theta sketches` (before `## Sketch families`):

```markdown
## CPC sketches

The CPC (Compressed Probabilistic Counting) sketch family (`cpc` feature)
supports cardinality estimation, like HLL and Theta, with a more compact
serialized form. Unlike Theta, CPC has no set operations beyond union —
no intersection, a-not-b, or Jaccard similarity.

```rust
use apache_datasketches::cpc::CpcSketchBuilder;

let mut sketch = CpcSketchBuilder::new().lg_k(11).build()?;
sketch.update_u64(42);
println!("estimate: {}", sketch.get_estimate());
```

- `CpcSketch` / `CpcSketchBuilder` — the sketch; build with
  `CpcSketchBuilder::new().lg_k(..).build()`. Supports the full upstream
  `update` overload set (`update_u64`/`update_i64`/`update_u32`/
  `update_i32`/`update_u16`/`update_i16`/`update_u8`/`update_i8`/
  `update_f64`/`update_f32`/`update_str`/`update_bytes`), `serialize`/
  `CpcSketch::deserialize`, and `get_lg_k`/`get_lower_bound`/
  `get_upper_bound`/`to_string_summary`.
- `CpcUnion` / `CpcUnionBuilder` — merges multiple sketches; build with
  `CpcUnionBuilder::new().lg_k(..).build()`, feed sketches via `update`,
  and read the merged sketch via `get_result()`.
- `get_max_serialized_size_bytes(lg_k)` — the estimated maximum compressed
  serialized size, in bytes, for a given `lg_k`; useful for pre-allocating
  buffers.
- `cpc::init()` — eagerly initializes CPC's global decompression tables.
  Upstream's lazy self-initialization on first serialize/deserialize is
  **not thread-safe**; call `init()` once, single-threaded, before
  spawning worker threads that will serialize or deserialize CPC sketches
  concurrently. Single-threaded callers never need to call this.

See `examples/cpc.rs` (`cargo run -p apache-datasketches --example cpc
--features cpc`) for a complete runnable demo.
```

Update the crate-level dependency snippets near the top of the file: add
a fourth `Cargo.toml` block (`features = ["cpc"]`) after the existing
three, and update the combined block to show all three features together:

```toml
[dependencies]
apache-datasketches = { version = "0.2", features = ["cpc"] }
```

```toml
[dependencies]
apache-datasketches = { version = "0.2", features = ["hll", "theta", "cpc"] }
```

Update the `## Sketch families` checklist:

```markdown
## Sketch families

- [x] HLL (HyperLogLog) — `hll` feature (sketch + union).
- [x] Theta — `theta` feature (sketch, union, intersection, a-not-b,
  Jaccard similarity).
- [x] CPC (Compressed Probabilistic Counting) — `cpc` feature (sketch +
  union).
```

- [ ] **Step 3: Update root `README.md`**

Update the `## Sketch families` section:

```markdown
## Sketch families

`default = []` for both crates; enable one or more of the following
opt-in features:

- [x] `hll` (HyperLogLog) — cardinality estimation (sketch + union).
- [x] `theta` — cardinality estimation plus set operations: union,
  intersection, a-not-b, and Jaccard similarity.
- [x] `cpc` (Compressed Probabilistic Counting) — cardinality estimation
  with a more compact serialized form (sketch + union; no set operations
  beyond union).
```

Update the `## Examples` section's command list:

```markdown
```bash
cargo run -p apache-datasketches --example hll --features hll
cargo run -p apache-datasketches --example theta --features theta
cargo run -p apache-datasketches --example cpc --features cpc
```
```

- [ ] **Step 4: Write `apache-datasketches/examples/cpc.rs`**

```rust
//! Standalone demo of the CPC (Compressed Probabilistic Counting) sketch
//! and union APIs.
//!
//! Run with: `cargo run -p apache-datasketches --example cpc --features cpc`

use apache_datasketches::cpc::{CpcSketch, CpcSketchBuilder, CpcUnionBuilder};

fn main() {
    // A sketch estimates the number of distinct items seen, using bounded
    // memory regardless of how many items are added. `lg_k` (4..=26)
    // trades memory for accuracy: higher values are more accurate but use
    // more space.
    let mut visitors_day1 = CpcSketchBuilder::new().lg_k(11).build().expect("valid lg_k");
    for id in 0..10_000u64 {
        visitors_day1.update_u64(id);
    }

    let mut visitors_day2 = CpcSketchBuilder::new().lg_k(11).build().expect("valid lg_k");
    for id in 5_000..15_000u64 {
        visitors_day2.update_u64(id);
    }

    println!("Day 1 unique visitors (estimate): {:.0}", visitors_day1.get_estimate());
    println!("Day 2 unique visitors (estimate): {:.0}", visitors_day2.get_estimate());
    println!(
        "Day 1, 95% confidence interval: [{:.0}, {:.0}]",
        visitors_day1.get_lower_bound(2).unwrap(),
        visitors_day1.get_upper_bound(2).unwrap()
    );

    // CpcUnion merges multiple sketches into one, e.g. combining per-day
    // counts into a total distinct count across both days.
    let mut union = CpcUnionBuilder::new().lg_k(11).build().expect("valid lg_k");
    union.update(&visitors_day1);
    union.update(&visitors_day2);
    let total_unique = union.get_result();
    println!(
        "Total unique visitors across both days (true count = 15000): {:.0}",
        total_unique.get_estimate()
    );

    // Sketches can be serialized to bytes (e.g. to store or send over the
    // network) and reconstructed later. CPC's serialized form is always
    // compressed, so there's a single serialize()/deserialize() pair
    // (unlike Theta, which has separate compressed/uncompressed formats).
    let bytes = visitors_day1.serialize();
    let restored = CpcSketch::deserialize(&bytes).expect("valid sketch bytes");
    println!(
        "serialized day-1 sketch to {} bytes and restored successfully (estimate {:.0})",
        bytes.len(),
        restored.get_estimate()
    );
}
```

- [ ] **Step 5: Add the example entry to `apache-datasketches/Cargo.toml`**

Append:

```toml
[[example]]
name = "cpc"
required-features = ["cpc"]
```

- [ ] **Step 6: Run the example**

```bash
cargo run -p apache-datasketches --example cpc --features cpc
```

Expected: prints all five lines above without panicking; estimates are all within a few percent of the true set sizes (10,000 / 10,000 / ~15,000, similarity concept doesn't apply to CPC — no Jaccard here, unlike the Theta example).

- [ ] **Step 7: Run the full workspace test suite across all four feature combinations**

```bash
cargo build --workspace
cargo test --workspace
cargo test --workspace --features apache-datasketches/cpc
cargo test --workspace --features apache-datasketches/hll,apache-datasketches/theta,apache-datasketches/cpc
```

Expected: all PASS. The first two (`default = []`, no features) compile
both crates' library code with no HLL/Theta/CPC types and run 0
hll/theta/cpc tests (thanks to the `required-features` entries already in
place from the Theta plan plus this plan's additions). The third and
fourth run exactly the test binaries whose `required-features` are
satisfied.

- [ ] **Step 8: Commit**

```bash
git add apache-datasketches-sys/README.md apache-datasketches/README.md README.md apache-datasketches/examples/cpc.rs apache-datasketches/Cargo.toml
git commit -m "Update READMEs and add cpc example"
```

---
