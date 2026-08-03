# ArrayOfDoubles (Tuple) Sketch Rust Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add safe Rust bindings for upstream datasketches-cpp's `array_of_doubles_sketch` — the concrete Tuple sketch whose per-entry summary is a fixed-width array of `f64` — behind a new `tuple` Cargo feature, with sketch, compact sketch, union, intersection, a-not-b, and Jaccard similarity.

**Architecture:** Three layers per component, identical to the existing Theta family: a C++ shim class in `apache-datasketches-sys/cpp/tuple/*_shim.{h,cc}` that wraps one upstream template instantiation and exposes only cxx-compatible signatures; a `#[cxx::bridge]` module in `apache-datasketches-sys/src/array_of_doubles_*.rs`; and a safe wrapper in `apache-datasketches/src/tuple/*.rs` that owns a `UniquePtr<...Shim>`, maps `cxx::Exception` to `SketchError`, and performs the two validations upstream C++ does not do itself (per-update slice length, cross-operand `num_values` equality).

**Tech Stack:** Rust 2021, `cxx` 1.x / `cxx-build` 1.x, C++17, vendored `datasketches-cpp` headers (header-only), `thiserror` 1.x.

## Global Constraints

- Scope is `array_of_doubles_sketch` **only**. The fully generic `tuple_sketch<Summary, Update, Policy, Allocator>` and any type-erasure/callback framework are explicitly out of scope (a separate future design). Do not add a `TupleSummary` trait, `DynSummary`, or any Rust-callback plumbing.
- New Cargo feature is named `tuple` (not `array-of-doubles`), additive with `hll`/`theta`/`cpc`, and `default = []` stays unchanged in both crates.
- Only two Rust sketch types: `ArrayOfDoublesSketch` (mutable) and `CompactArrayOfDoublesSketch` (immutable/serializable). There is **no** "wrapped" zero-copy type — upstream provides none for this family.
- The seed is never exposed. Every construction uses upstream's `DEFAULT_SEED`.
- Errors reuse the existing `crate::error::SketchError` enum. **Do not add a new variant.** `values.len() != num_values` and cross-operand `num_values` mismatch both map to `SketchError::InvalidConfig`; a missing intersection result maps to the existing `SketchError::EmptyIntersection`.
- Intersection uses a **sum** combine-on-collision policy (`default_array_of_doubles_union_policy`), which is also what upstream's own test file uses. No other policies in v1.
- Every public item needs rustdoc — `apache-datasketches/src/lib.rs` has `#![warn(missing_docs)]` and it must stay clean. This includes every enum variant and struct field.
- All five owned types (`ArrayOfDoublesSketch`, `CompactArrayOfDoublesSketch`, `ArrayOfDoublesUnion`, `ArrayOfDoublesIntersection`, `ArrayOfDoublesAnotB`) get `unsafe impl Send`, and **not** `Sync`.
- C++ shim code lives in `namespace apache_datasketches_rs`, matching every existing shim.
- Commit messages must **not** include a `Co-Authored-By` trailer.
- Never edit anything under `vendor/datasketches-cpp/` (the root submodule) or `apache-datasketches-sys/vendor/` other than by the documented copy script.

### Cross-cutting naming decisions (read before starting any task)

These names are fixed across tasks; using a different one breaks a neighbouring task.

- The cxx **shared enum** for the resize factor is named `TupleResizeFactor` (not `ResizeFactor`). Reason: cxx emits a C++ definition into `namespace apache_datasketches_rs` for each shared type, and the Theta bridge already emits `apache_datasketches_rs::ResizeFactor` there. With both `theta` and `tuple` features enabled those would collide, so the tuple one gets a distinct name. Its C++→upstream converter is the free function `to_cpp_tuple_resize_factor`.
- The cxx **shared struct** for Jaccard results is named `TupleJaccardBoundsFfi`, for the same collision reason (`JaccardBoundsFfi` is already taken by the Theta bridge).
- The safe crate re-exports its own `tuple::ResizeFactor` and `tuple::JaccardBounds` — distinct Rust types from `theta::ResizeFactor`/`theta::JaccardBounds`, with the same shape. This is intentional; do not try to share them.
- Set operations dispatch over both input types via a sealed trait `ArrayOfDoublesInput` (safe crate, `src/tuple/input.rs`) backed by a plain enum `ArrayOfDoublesInputRef` (sys crate, `src/array_of_doubles_input.rs`). This mirrors Theta's `ThetaInput`/`ThetaInputRef` exactly. **Note on the design doc:** the design says "a `ThetaInput`-style sealed trait isn't needed here". That remark is about *cross-family* dispatch (mixing tuple and theta sketches), which this plan does not do. A two-variant dispatch is still required, because the design also specifies single-method signatures — one `AnotB::compute(a, b, ordered)` and one `array_of_doubles_jaccard_similarity(a, b)` — which cannot accept both a mutable and a compact sketch without it. The alternative (four differently-named `compute_*` methods and four free jaccard functions) would violate those specified signatures, so the trait is the faithful choice.
- `entries()` returns `impl Iterator<Item = (u64, Vec<f64>)>`, **not** the design's literal `impl Iterator<Item = (u64, &[f64])>`. Reason: the values must be copied out of C++ in one FFI call (cxx cannot hand back a live C++ iterator), and returning borrowed slices from a `Vec` created inside the method would be a self-referential borrow that Rust rejects. Owned `Vec<f64>` per entry is the closest implementable form.
- `CompactArrayOfDoublesSketch` has a single `serialize() -> Vec<u8>`, unlike Theta's `serialize_compact`/`serialize_compressed` pair. Upstream's `compact_array_tuple_sketch` has exactly one, hand-rolled, non-templated serialization format (no compressed variant, and no `serde<>` specialization is needed — its own `serialize`/`deserialize` overloads shadow the generic templated ones in `compact_tuple_sketch`).

---

### Task 1: Feature scaffolding and vendored tuple headers

Pure plumbing: get `--features tuple` to be a valid, buildable feature with the upstream tuple headers on the include path, before any shim exists. `build.rs` guards every per-file reference with a `Path::exists()` check (the same trick the theta and cpc blocks already use), so `--features tuple` keeps building at every intermediate task in this plan.

**Files:**
- Create (by copy): `apache-datasketches-sys/vendor/datasketches-cpp/tuple/include/` (whole directory)
- Modify: `apache-datasketches-sys/vendor/README.md`
- Modify: `apache-datasketches-sys/Cargo.toml:17-21` (`[features]`)
- Modify: `apache-datasketches/Cargo.toml:12-21` (`[features]` and `[package.metadata.docs.rs]`)
- Modify: `apache-datasketches-sys/build.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: Cargo features `tuple` (both crates); include paths `vendor/datasketches-cpp/tuple/include` and `cpp/tuple`; a `cfg!(feature = "tuple")` bridge-list block and `.file()` block in `build.rs` that later tasks add filenames to.

- [ ] **Step 1: Copy the upstream tuple headers into the crate's vendor directory**

The crate builds against `apache-datasketches-sys/vendor/datasketches-cpp`, not the repo-root submodule, so that `cargo package` produces a self-contained tarball. Run from the repo root:

```bash
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/tuple
cp -R vendor/datasketches-cpp/tuple/include apache-datasketches-sys/vendor/datasketches-cpp/tuple/include
```

- [ ] **Step 2: Verify the headers this family needs are present**

Run: `ls apache-datasketches-sys/vendor/datasketches-cpp/tuple/include/`
Expected: the listing includes at least `array_of_doubles_sketch.hpp`, `array_tuple_sketch.hpp`, `array_tuple_sketch_impl.hpp`, `array_tuple_union.hpp`, `array_tuple_intersection.hpp`, `array_tuple_a_not_b.hpp`, `tuple_sketch.hpp`, `tuple_sketch_impl.hpp`, `tuple_union.hpp`, `tuple_intersection.hpp`, `tuple_a_not_b.hpp`, `tuple_jaccard_similarity.hpp`.

No `theta/` or `common/` copying is needed — the tuple headers include `theta_update_sketch_base.hpp`, `theta_jaccard_similarity_base.hpp`, `serde.hpp` and friends, and both `theta/include` and `common/include` are already vendored and already unconditionally on the include path.

- [ ] **Step 3: Extend the vendor copy script documentation**

In `apache-datasketches-sys/vendor/README.md`, change the sentence listing what is copied, and add the two `tuple` lines to the shell block.

Replace:

```
Only the headers actually compiled (`common/include`, `hll/include`,
`theta/include`, `cpc/include`, `LICENSE`, `NOTICE`) are copied;
`version.hpp.in` is skipped since none of the compiled headers include it.
```

with:

```
Only the headers actually compiled (`common/include`, `hll/include`,
`theta/include`, `cpc/include`, `tuple/include`, `LICENSE`, `NOTICE`) are
copied; `version.hpp.in` is skipped since none of the compiled headers
include it.
```

Then, in the shell block, after the `mkdir -p ...cpc` line add:

```bash
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/tuple
```

and after the `cp -R vendor/datasketches-cpp/cpc/include ...` line add:

```bash
cp -R vendor/datasketches-cpp/tuple/include apache-datasketches-sys/vendor/datasketches-cpp/tuple/include
```

Finally replace the closing paragraph:

```
When a future sketch family needs headers outside `common/`+`hll/`+
`theta/`+`cpc/`, add its `include/` directory to both this script and
`build.rs`.
```

with:

```
When a future sketch family needs headers outside `common/`+`hll/`+
`theta/`+`cpc/`+`tuple/`, add its `include/` directory to both this script
and `build.rs`.
```

- [ ] **Step 4: Add the `tuple` feature to the sys crate**

In `apache-datasketches-sys/Cargo.toml`, change the `[features]` block to:

```toml
[features]
default = []
hll = []
theta = []
cpc = []
tuple = []
```

- [ ] **Step 5: Add the `tuple` feature to the safe crate**

In `apache-datasketches/Cargo.toml`, change the `[features]` and `[package.metadata.docs.rs]` blocks to:

```toml
[features]
default = []
hll = ["apache-datasketches-sys/hll"]
theta = ["apache-datasketches-sys/theta"]
cpc = ["apache-datasketches-sys/cpc"]
tuple = ["apache-datasketches-sys/tuple"]

[package.metadata.docs.rs]
# default = [] means docs.rs's default build would show an empty crate;
# build with every sketch family enabled instead.
features = ["hll", "theta", "cpc", "tuple"]
```

- [ ] **Step 6: Add the tuple bridge list to `build.rs`**

In `apache-datasketches-sys/build.rs`, immediately after the closing brace of the `if cfg!(feature = "cpc") { ... }` bridge block (the one containing `src/cpc_sketch.rs`), insert:

```rust
    if cfg!(feature = "tuple") {
        // Same incremental-availability rationale as theta and cpc above:
        // these bridge modules are added one per task by the
        // ArrayOfDoubles (Tuple) plan, so only reference the ones that
        // exist so far and keep `--features tuple` building throughout.
        // Note: src/array_of_doubles_input.rs is deliberately absent — it
        // is a plain Rust module, not a cxx bridge.
        for path in [
            "src/array_of_doubles_sketch.rs",
            "src/array_of_doubles_compact.rs",
            "src/array_of_doubles_union.rs",
            "src/array_of_doubles_intersection.rs",
            "src/array_of_doubles_a_not_b.rs",
            "src/array_of_doubles_jaccard.rs",
        ] {
            if std::path::Path::new(path).exists() {
                bridges.push(path);
            }
        }
    }
```

- [ ] **Step 7: Add the tuple include paths to `build.rs`**

In the same file, in the `build` configuration chain, add the two tuple include lines so the chain reads:

```rust
    let mut build = cxx_build::bridges(&bridges);
    build
        .include(vendor_dir.join("common/include"))
        .include(vendor_dir.join("hll/include"))
        .include(vendor_dir.join("theta/include"))
        .include(vendor_dir.join("cpc/include"))
        .include(vendor_dir.join("tuple/include"))
        .include("cpp")
        .include("cpp/hll")
        .include("cpp/theta")
        .include("cpp/cpc")
        .include("cpp/tuple")
        .include(generated_header_dir)
        .flag_if_supported("-std=c++17")
        // Upstream datasketches-cpp declares virtual destructors on a couple
        // of `final` classes (e.g. hll_sketch_alloc, AuxHashMap) — harmless,
        // but noisy under clang. Silenced here rather than patched in the
        // vendored headers so we don't diverge from upstream.
        .flag_if_supported("-Wno-unnecessary-virtual-specifier");
```

These are unconditional (not feature-gated), exactly like the existing per-family include lines — an unused include directory is harmless.

- [ ] **Step 8: Add the tuple `.cc` compilation block to `build.rs`**

Immediately after the closing brace of the `if cfg!(feature = "cpc") { ... }` block that calls `build.file(path)`, insert:

```rust
    if cfg!(feature = "tuple") {
        for path in [
            "cpp/tuple/array_of_doubles_sketch_shim.cc",
            "cpp/tuple/array_of_doubles_compact_shim.cc",
            "cpp/tuple/array_of_doubles_union_shim.cc",
            "cpp/tuple/array_of_doubles_intersection_shim.cc",
            "cpp/tuple/array_of_doubles_a_not_b_shim.cc",
            "cpp/tuple/array_of_doubles_jaccard_shim.cc",
        ] {
            if std::path::Path::new(path).exists() {
                build.file(path);
            }
        }
    }
```

- [ ] **Step 9: Add the tuple `rerun-if-changed` lines to `build.rs`**

At the end of `main`, after the last `cargo:rerun-if-changed=cpp/cpc/cpc_union_shim.cc` line, insert:

```rust
    println!("cargo:rerun-if-changed=src/array_of_doubles_sketch.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_compact.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_input.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_union.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_intersection.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_a_not_b.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_jaccard.rs");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_compact_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_compact_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_union_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_intersection_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_intersection_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_a_not_b_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_a_not_b_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_jaccard_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_jaccard_shim.cc");
```

Printing `rerun-if-changed` for files that do not exist yet is harmless and matches the existing unconditional style.

- [ ] **Step 10: Verify the new feature builds and nothing regressed**

Run: `cargo build --workspace --features apache-datasketches/tuple`
Expected: succeeds. (No bridges exist yet, so `bridges` is empty and `build.rs` returns before compiling any C++.)

Run: `cargo build --workspace --features apache-datasketches/theta,apache-datasketches/cpc,apache-datasketches/hll,apache-datasketches/tuple`
Expected: succeeds.

Run: `cargo test --workspace --features apache-datasketches/theta`
Expected: all existing theta tests still pass.

- [ ] **Step 11: Commit**

```bash
git add apache-datasketches-sys/vendor apache-datasketches-sys/Cargo.toml apache-datasketches-sys/build.rs apache-datasketches/Cargo.toml
git commit -m "feat(tuple): add tuple Cargo feature and vendor upstream tuple headers"
```

---

### Task 2: `ArrayOfDoublesSketch` and its builder

The mutable, update-only sketch. Deliberately does **not** include `compact()` — that arrives in Task 3 with the compact type, so this task's C++ compiles standalone.

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_sketch_shim.h`
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_sketch_shim.cc`
- Create: `apache-datasketches-sys/src/array_of_doubles_sketch.rs`
- Create (test): `apache-datasketches-sys/tests/array_of_doubles_sketch_link_test.rs`
- Create: `apache-datasketches/src/tuple/mod.rs`
- Create: `apache-datasketches/src/tuple/builder.rs`
- Create: `apache-datasketches/src/tuple/sketch.rs`
- Create (test): `apache-datasketches/tests/tuple_sketch_smoke_test.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Modify: `apache-datasketches/src/lib.rs`
- Modify: `apache-datasketches-sys/Cargo.toml` (add `[[test]]`)
- Modify: `apache-datasketches/Cargo.toml` (add `[[test]]`)

**Interfaces:**
- Consumes: Task 1's `tuple` feature, include paths, and `build.rs` blocks.
- Produces:
  - C++: `apache_datasketches_rs::ArrayOfDoublesSketchShim` with `const datasketches::update_array_of_doubles_sketch& inner() const`; free functions `datasketches::resize_factor to_cpp_tuple_resize_factor(TupleResizeFactor)` (**defined here, forward-declared elsewhere**) and `std::unique_ptr<ArrayOfDoublesSketchShim> new_array_of_doubles_sketch(uint8_t, TupleResizeFactor, float, uint8_t)`.
  - Rust (sys): `apache_datasketches_sys::array_of_doubles_sketch::ffi` with shared enum `TupleResizeFactor { X1, X2, X4, X8 }` and opaque `ArrayOfDoublesSketchShim`.
  - Rust (safe): `apache_datasketches::tuple::{ResizeFactor, ArrayOfDoublesSketchBuilder, ArrayOfDoublesSketch}`. `ArrayOfDoublesSketch` has `pub(crate) inner: UniquePtr<sys::ArrayOfDoublesSketchShim>` and `pub(crate) fn from_parts(lg_k: u8, rf: ResizeFactor, p: f32, num_values: u8) -> Result<Self, SketchError>`. Public methods: `update_u64/i64/u32/i32/u16/i16/u8/i8/f64(key, values: &[f64]) -> Result<(), SketchError>`, `update_str(&str, &[f64]) -> Result<(), SketchError>`, `update_bytes(&[u8], &[f64]) -> Result<(), SketchError>`, `trim()`, `reset()`, `get_estimate() -> f64`, `get_lower_bound(u8) -> Result<f64, SketchError>`, `get_upper_bound(u8) -> Result<f64, SketchError>`, `is_empty() -> bool`, `is_estimation_mode() -> bool`, `is_ordered() -> bool`, `get_theta() -> f64`, `get_num_retained() -> u32`, `get_num_values() -> u8`, `entries() -> impl Iterator<Item = (u64, Vec<f64>)>`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/array_of_doubles_sketch_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_sketch::ffi;

#[test]
fn construct_update_estimate() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &[1.0]);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
    assert_eq!(sketch.get_num_values(), 1);
    assert_eq!(sketch.get_num_retained(), 1000);
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = ffi::new_array_of_doubles_sketch(4, ffi::TupleResizeFactor::X8, 1.0, 1);
    assert!(result.is_err());
}

#[test]
fn entries_expose_hashes_and_values() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 2).unwrap();
    sketch.pin_mut().update_u64(1, &[3.0, 4.0]);
    sketch.pin_mut().update_u64(1, &[3.0, 4.0]);
    assert_eq!(sketch.get_num_retained(), 1);
    let hashes = sketch.entry_hashes();
    let values = sketch.entry_values();
    assert_eq!(hashes.len(), 1);
    // Two updates of the same key sum their values.
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], 6.0);
    assert_eq!(values[1], 8.0);
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    sketch.pin_mut().update_u64(1, &[1.0]);
    assert!(!sketch.is_empty());
    sketch.pin_mut().reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 0);
}
```

Register it in `apache-datasketches-sys/Cargo.toml` by appending:

```toml
[[test]]
name = "array_of_doubles_sketch_link_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_sketch_link_test`
Expected: FAIL — compile error, `unresolved import apache_datasketches_sys::array_of_doubles_sketch`.

- [ ] **Step 3: Write the C++ shim header**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_sketch_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"

namespace apache_datasketches_rs {

// Forward declaration matching the enum generated by cxx from
// src/array_of_doubles_sketch.rs. We deliberately do NOT
// `#include "array_of_doubles_sketch.rs.h"` here, for the same reason
// documented in theta_sketch_shim.h: the generated header's own `include!`
// directive re-enters this header while it is still being processed. The full
// enum definition is pulled in by array_of_doubles_sketch_shim.cc after this
// header.
//
// The enum is named TupleResizeFactor rather than ResizeFactor because cxx
// emits one C++ definition per shared type into this namespace, and the theta
// bridge already emits apache_datasketches_rs::ResizeFactor there.
enum class TupleResizeFactor : std::uint8_t;

// Defined once, in array_of_doubles_sketch_shim.cc; forward-declared in every
// other tuple shim header that needs it (currently the union shim). Both
// translation units land in the same static library, so a single definition
// satisfies the One Definition Rule. Same pattern as theta's
// to_cpp_resize_factor.
datasketches::resize_factor to_cpp_tuple_resize_factor(TupleResizeFactor rf);

class ArrayOfDoublesSketchShim {
public:
  explicit ArrayOfDoublesSketchShim(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values);

  void update_u64(uint64_t key, rust::Slice<const double> values);
  void update_i64(int64_t key, rust::Slice<const double> values);
  void update_u32(uint32_t key, rust::Slice<const double> values);
  void update_i32(int32_t key, rust::Slice<const double> values);
  void update_u16(uint16_t key, rust::Slice<const double> values);
  void update_i16(int16_t key, rust::Slice<const double> values);
  void update_u8(uint8_t key, rust::Slice<const double> values);
  void update_i8(int8_t key, rust::Slice<const double> values);
  void update_f64(double key, rust::Slice<const double> values);
  void update_str(rust::Str key, rust::Slice<const double> values);
  void update_bytes(rust::Slice<const uint8_t> key, rust::Slice<const double> values);

  void trim();
  void reset();

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;
  uint8_t get_num_values() const;

  rust::Vec<uint64_t> entry_hashes() const;
  rust::Vec<double> entry_values() const;

  const datasketches::update_array_of_doubles_sketch& inner() const { return sketch_; }

private:
  datasketches::update_array_of_doubles_sketch sketch_;
};

std::unique_ptr<ArrayOfDoublesSketchShim> new_array_of_doubles_sketch(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values);

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the C++ shim implementation**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_sketch_shim.cc`:

```cpp
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_sketch.rs.h" // generated by cxx from src/array_of_doubles_sketch.rs; provides the full TupleResizeFactor enum definition

namespace apache_datasketches_rs {

datasketches::resize_factor to_cpp_tuple_resize_factor(TupleResizeFactor rf) {
  switch (rf) {
    case TupleResizeFactor::X1: return datasketches::resize_factor::X1;
    case TupleResizeFactor::X2: return datasketches::resize_factor::X2;
    case TupleResizeFactor::X4: return datasketches::resize_factor::X4;
    case TupleResizeFactor::X8: return datasketches::resize_factor::X8;
    default: throw std::invalid_argument("unknown TupleResizeFactor");
  }
}

namespace {

// The builder's single constructor argument is the update policy, which is
// what carries num_values. lg_k/resize_factor/p validation is inherited from
// theta_base_builder and throws std::invalid_argument, which cxx turns into
// Result::Err on the Rust side.
datasketches::update_array_of_doubles_sketch build_sketch(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values) {
  datasketches::update_array_of_doubles_sketch::builder builder(
      datasketches::default_array_of_doubles_update_policy(num_values));
  builder.set_lg_k(lg_k);
  builder.set_resize_factor(to_cpp_tuple_resize_factor(rf));
  builder.set_p(p);
  return builder.build();
}

// Upstream's default_array_tuple_update_policy indexes the supplied value
// blindly for i in [0, num_values), with no bounds check of its own. The safe
// Rust wrapper rejects a wrong-length slice before it ever gets here; this is
// a second line of defence for direct sys-crate callers.
void check_values_len(const datasketches::update_array_of_doubles_sketch& sketch, rust::Slice<const double> values) {
  if (values.size() != sketch.get_num_values()) {
    throw std::invalid_argument("values length does not match num_values");
  }
}

std::vector<double> to_vector(rust::Slice<const double> values) {
  return std::vector<double>(values.begin(), values.end());
}

} // namespace

ArrayOfDoublesSketchShim::ArrayOfDoublesSketchShim(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values)
  : sketch_(build_sketch(lg_k, rf, p, num_values)) {}

void ArrayOfDoublesSketchShim::update_u64(uint64_t key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_i64(int64_t key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_u32(uint32_t key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_i32(int32_t key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_u16(uint16_t key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_i16(int16_t key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_u8(uint8_t key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_i8(int8_t key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_f64(double key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key, to_vector(values));
}
void ArrayOfDoublesSketchShim::update_str(rust::Str key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(std::string(key), to_vector(values));
}
void ArrayOfDoublesSketchShim::update_bytes(rust::Slice<const uint8_t> key, rust::Slice<const double> values) {
  check_values_len(sketch_, values);
  sketch_.update(key.data(), key.size(), to_vector(values));
}

void ArrayOfDoublesSketchShim::trim() { sketch_.trim(); }
void ArrayOfDoublesSketchShim::reset() { sketch_.reset(); }

double ArrayOfDoublesSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double ArrayOfDoublesSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double ArrayOfDoublesSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool ArrayOfDoublesSketchShim::is_empty() const { return sketch_.is_empty(); }
bool ArrayOfDoublesSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool ArrayOfDoublesSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double ArrayOfDoublesSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t ArrayOfDoublesSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }
uint8_t ArrayOfDoublesSketchShim::get_num_values() const { return sketch_.get_num_values(); }

rust::Vec<uint64_t> ArrayOfDoublesSketchShim::entry_hashes() const {
  rust::Vec<uint64_t> out;
  for (const auto& entry : sketch_) out.push_back(entry.first);
  return out;
}

rust::Vec<double> ArrayOfDoublesSketchShim::entry_values() const {
  rust::Vec<double> out;
  for (const auto& entry : sketch_) {
    for (uint8_t i = 0; i < entry.second.size(); ++i) out.push_back(entry.second[i]);
  }
  return out;
}

std::unique_ptr<ArrayOfDoublesSketchShim> new_array_of_doubles_sketch(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values) {
  return std::make_unique<ArrayOfDoublesSketchShim>(lg_k, rf, p, num_values);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 5: Write the cxx bridge module**

Create `apache-datasketches-sys/src/array_of_doubles_sketch.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    /// Named `TupleResizeFactor` rather than `ResizeFactor` because cxx emits
    /// one C++ definition per shared type into the bridge namespace, and the
    /// theta bridge already emits `apache_datasketches_rs::ResizeFactor`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TupleResizeFactor {
        X1,
        X2,
        X4,
        X8,
    }

    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");

        type ArrayOfDoublesSketchShim;

        fn new_array_of_doubles_sketch(lg_k: u8, rf: TupleResizeFactor, p: f32, num_values: u8) -> Result<UniquePtr<ArrayOfDoublesSketchShim>>;

        fn update_u64(self: Pin<&mut ArrayOfDoublesSketchShim>, key: u64, values: &[f64]);
        fn update_i64(self: Pin<&mut ArrayOfDoublesSketchShim>, key: i64, values: &[f64]);
        fn update_u32(self: Pin<&mut ArrayOfDoublesSketchShim>, key: u32, values: &[f64]);
        fn update_i32(self: Pin<&mut ArrayOfDoublesSketchShim>, key: i32, values: &[f64]);
        fn update_u16(self: Pin<&mut ArrayOfDoublesSketchShim>, key: u16, values: &[f64]);
        fn update_i16(self: Pin<&mut ArrayOfDoublesSketchShim>, key: i16, values: &[f64]);
        fn update_u8(self: Pin<&mut ArrayOfDoublesSketchShim>, key: u8, values: &[f64]);
        fn update_i8(self: Pin<&mut ArrayOfDoublesSketchShim>, key: i8, values: &[f64]);
        fn update_f64(self: Pin<&mut ArrayOfDoublesSketchShim>, key: f64, values: &[f64]);
        fn update_str(self: Pin<&mut ArrayOfDoublesSketchShim>, key: &str, values: &[f64]);
        fn update_bytes(self: Pin<&mut ArrayOfDoublesSketchShim>, key: &[u8], values: &[f64]);

        fn trim(self: Pin<&mut ArrayOfDoublesSketchShim>);
        fn reset(self: Pin<&mut ArrayOfDoublesSketchShim>);

        fn get_estimate(self: &ArrayOfDoublesSketchShim) -> f64;
        fn get_lower_bound(self: &ArrayOfDoublesSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &ArrayOfDoublesSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &ArrayOfDoublesSketchShim) -> bool;
        fn is_estimation_mode(self: &ArrayOfDoublesSketchShim) -> bool;
        fn is_ordered(self: &ArrayOfDoublesSketchShim) -> bool;
        fn get_theta(self: &ArrayOfDoublesSketchShim) -> f64;
        fn get_num_retained(self: &ArrayOfDoublesSketchShim) -> u32;
        fn get_num_values(self: &ArrayOfDoublesSketchShim) -> u8;

        fn entry_hashes(self: &ArrayOfDoublesSketchShim) -> Vec<u64>;
        fn entry_values(self: &ArrayOfDoublesSketchShim) -> Vec<f64>;
    }
}
```

The `update_*` methods are declared infallible here even though the shim's `check_values_len` can throw, because the safe wrapper validates first and the sys crate is documented as internal/unstable. If a raw sys-crate caller passes a wrong-length slice the process aborts via cxx's uncaught-exception path — acceptable for this crate's stated "do not use directly" contract, and strictly better than the out-of-bounds read that no check at all would produce.

Add to `apache-datasketches-sys/src/lib.rs`, after the cpc block:

```rust

#[cfg(feature = "tuple")]
pub mod array_of_doubles_sketch;
```

- [ ] **Step 6: Run the link test to verify it passes**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_sketch_link_test`
Expected: PASS (4 tests).

- [ ] **Step 7: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_sketch_smoke_test.rs`:

```rust
use apache_datasketches::tuple::ArrayOfDoublesSketchBuilder;

#[test]
fn construct_update_estimate() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i, &[1.0]).unwrap();
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
    assert_eq!(sketch.get_num_values(), 1);
}

#[test]
fn invalid_lg_k_is_err() {
    assert!(ArrayOfDoublesSketchBuilder::new().lg_k(4).build().is_err());
}

#[test]
fn num_values_zero_is_err() {
    assert!(ArrayOfDoublesSketchBuilder::new().num_values(0).build().is_err());
}

#[test]
fn wrong_length_values_is_err() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    assert!(sketch.update_u64(1, &[1.0]).is_err());
    assert!(sketch.update_u64(1, &[1.0, 2.0, 3.0]).is_err());
    assert!(sketch.update_u64(1, &[1.0, 2.0]).is_ok());
}

#[test]
fn entries_yields_hash_and_values() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().num_values(3).build().unwrap();
    sketch.update_u64(7, &[1.0, 2.0, 3.0]).unwrap();
    sketch.update_u64(7, &[1.0, 2.0, 3.0]).unwrap();
    let entries: Vec<(u64, Vec<f64>)> = sketch.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, vec![2.0, 4.0, 6.0]);
}

#[test]
fn sketch_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<apache_datasketches::tuple::ArrayOfDoublesSketch>();
}
```

Register it in `apache-datasketches/Cargo.toml` by appending:

```toml
[[test]]
name = "tuple_sketch_smoke_test"
required-features = ["tuple"]
```

- [ ] **Step 8: Run the smoke test to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_sketch_smoke_test`
Expected: FAIL — compile error, `could not find tuple in apache_datasketches`.

- [ ] **Step 9: Write the safe-crate builder**

Create `apache-datasketches/src/tuple/builder.rs`:

```rust
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sys;

/// Controls how aggressively an ArrayOfDoubles sketch's internal hash table
/// grows. Mirrors upstream's `datasketches::resize_factor`. Default is `X8`,
/// matching `theta_constants::DEFAULT_RESIZE_FACTOR` (the tuple family
/// inherits Theta's builder defaults).
///
/// This is a distinct type from the theta module's `ResizeFactor` with the
/// same shape — the two sketch families are independently feature-gated and
/// do not share types. (Deliberately not an intra-doc link: `theta` may not
/// be compiled in.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeFactor {
    /// Grow by 1x (i.e. never resize past the initial allocation).
    X1,
    /// Grow by 2x each time the hash table fills.
    X2,
    /// Grow by 4x each time the hash table fills.
    X4,
    /// Grow by 8x each time the hash table fills. The default.
    X8,
}

impl Default for ResizeFactor {
    fn default() -> Self {
        ResizeFactor::X8
    }
}

impl From<ResizeFactor> for sys::TupleResizeFactor {
    fn from(rf: ResizeFactor) -> Self {
        match rf {
            ResizeFactor::X1 => sys::TupleResizeFactor::X1,
            ResizeFactor::X2 => sys::TupleResizeFactor::X2,
            ResizeFactor::X4 => sys::TupleResizeFactor::X4,
            ResizeFactor::X8 => sys::TupleResizeFactor::X8,
        }
    }
}

/// Builder for [`crate::tuple::ArrayOfDoublesSketch`], mirroring upstream's
/// `update_array_of_doubles_sketch::builder`. `lg_k` defaults to `12`,
/// `resize_factor` to [`ResizeFactor::X8`], `p` to `1.0` (no sampling), and
/// `num_values` to `1` (matching upstream's
/// `default_array_tuple_update_policy` default). The seed is never exposed —
/// every sketch built by this crate uses upstream's `DEFAULT_SEED`.
#[derive(Debug, Clone, Copy)]
pub struct ArrayOfDoublesSketchBuilder {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
    num_values: u8,
}

impl Default for ArrayOfDoublesSketchBuilder {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::default(),
            p: 1.0,
            num_values: 1,
        }
    }
}

impl ArrayOfDoublesSketchBuilder {
    /// Creates a new builder with default settings (`lg_k = 12`,
    /// `resize_factor = X8`, `p = 1.0`, `num_values = 1`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the target number of retained entries.
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Sets the hash table's growth [`ResizeFactor`].
    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    /// Sets the sampling probability. `1.0` (the default) disables sampling;
    /// values below `1.0` put the sketch into estimation mode from the start.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Sets the fixed number of `f64` values each retained entry carries.
    /// Must be at least `1`. Every sketch that will later be unioned,
    /// intersected, or differenced with this one must use the same value.
    pub fn num_values(mut self, num_values: u8) -> Self {
        self.num_values = num_values;
        self
    }

    /// Builds the sketch. Returns
    /// [`SketchError::InvalidConfig`](crate::SketchError::InvalidConfig) if
    /// `lg_k` is out of range, `p` is outside `(0, 1]`, or `num_values` is
    /// `0`.
    pub fn build(self) -> Result<super::ArrayOfDoublesSketch, crate::error::SketchError> {
        super::ArrayOfDoublesSketch::from_parts(
            self.lg_k,
            self.resize_factor,
            self.p,
            self.num_values,
        )
    }
}
```

- [ ] **Step 10: Write the safe-crate sketch wrapper**

Create `apache-datasketches/src/tuple/sketch.rs`:

```rust
use crate::error::SketchError;
use crate::tuple::builder::ResizeFactor;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sys;
use cxx::UniquePtr;

/// A mutable, update-only ArrayOfDoubles Tuple sketch: estimates the number
/// of distinct keys added via `update_*`, and carries a fixed-width array of
/// `f64` values per retained key, summed on collision. Build one with
/// [`ArrayOfDoublesSketchBuilder`](super::ArrayOfDoublesSketchBuilder).
pub struct ArrayOfDoublesSketch {
    pub(crate) inner: UniquePtr<sys::ArrayOfDoublesSketchShim>,
}

unsafe impl Send for ArrayOfDoublesSketch {}

impl ArrayOfDoublesSketch {
    pub(crate) fn from_parts(
        lg_k: u8,
        rf: ResizeFactor,
        p: f32,
        num_values: u8,
    ) -> Result<Self, SketchError> {
        if num_values == 0 {
            return Err(SketchError::InvalidConfig(
                "num_values must be at least 1".to_string(),
            ));
        }
        let inner = sys::new_array_of_doubles_sketch(lg_k, rf.into(), p, num_values)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Validates that `values` has exactly [`Self::get_num_values`] elements.
    ///
    /// This check cannot be delegated to the C++ layer's exceptions the way
    /// `lg_k`/`num_std_dev` validation is: upstream's update policy indexes
    /// the supplied values blindly for `i in 0..num_values`, so a short slice
    /// would be an out-of-bounds read rather than a graceful failure.
    fn check_values(&self, values: &[f64]) -> Result<(), SketchError> {
        let expected = self.inner.get_num_values() as usize;
        if values.len() != expected {
            return Err(SketchError::InvalidConfig(format!(
                "expected {expected} values, got {}",
                values.len()
            )));
        }
        Ok(())
    }

    /// Adds a `u64` key with its associated values. Returns
    /// [`SketchError::InvalidConfig`] unless
    /// `values.len() == self.get_num_values()`.
    pub fn update_u64(&mut self, key: u64, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_u64(key, values);
        Ok(())
    }

    /// Adds an `i64` key with its associated values. See [`Self::update_u64`].
    pub fn update_i64(&mut self, key: i64, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_i64(key, values);
        Ok(())
    }

    /// Adds a `u32` key with its associated values. See [`Self::update_u64`].
    pub fn update_u32(&mut self, key: u32, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_u32(key, values);
        Ok(())
    }

    /// Adds an `i32` key with its associated values. See [`Self::update_u64`].
    pub fn update_i32(&mut self, key: i32, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_i32(key, values);
        Ok(())
    }

    /// Adds a `u16` key with its associated values. See [`Self::update_u64`].
    pub fn update_u16(&mut self, key: u16, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_u16(key, values);
        Ok(())
    }

    /// Adds an `i16` key with its associated values. See [`Self::update_u64`].
    pub fn update_i16(&mut self, key: i16, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_i16(key, values);
        Ok(())
    }

    /// Adds a `u8` key with its associated values. See [`Self::update_u64`].
    pub fn update_u8(&mut self, key: u8, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_u8(key, values);
        Ok(())
    }

    /// Adds an `i8` key with its associated values. See [`Self::update_u64`].
    pub fn update_i8(&mut self, key: i8, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_i8(key, values);
        Ok(())
    }

    /// Adds an `f64` key with its associated values. See [`Self::update_u64`].
    pub fn update_f64(&mut self, key: f64, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_f64(key, values);
        Ok(())
    }

    /// Adds a string key with its associated values. See [`Self::update_u64`].
    pub fn update_str(&mut self, key: &str, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_str(key, values);
        Ok(())
    }

    /// Adds an arbitrary byte-slice key with its associated values. See
    /// [`Self::update_u64`].
    pub fn update_bytes(&mut self, key: &[u8], values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_bytes(key, values);
        Ok(())
    }

    /// Compacts the internal hash table down to its target size, discarding
    /// entries above the current theta threshold. Does not change the
    /// sketch's estimate.
    pub fn trim(&mut self) {
        self.inner.pin_mut().trim();
    }

    /// Resets this sketch to its initial, empty state. `num_values` is
    /// preserved.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns the current estimate of the number of distinct keys added.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`], for the given number of standard deviations
    /// (`1`, `2`, or `3`, corresponding to roughly 67%, 95%, and 99%
    /// confidence). Returns [`SketchError::InvalidConfig`] for any other
    /// value.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See [`Self::get_lower_bound`] for the meaning
    /// of `num_std_dev`.
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if no keys have been added to this sketch.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if this sketch's theta threshold is below `1.0`
    /// (i.e. it has begun sampling and [`Self::get_estimate`] is a
    /// statistical estimate rather than an exact count).
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if this sketch's retained entries are sorted by hash
    /// value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold (`1.0` until sampling begins).
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of entries currently retained by this sketch.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }

    /// Returns the fixed number of `f64` values each retained entry carries,
    /// as configured at build time.
    pub fn get_num_values(&self) -> u8 {
        self.inner.get_num_values()
    }

    /// Iterates the retained entries as `(hash, values)` pairs, where
    /// `values.len() == self.get_num_values()`.
    ///
    /// The entries are copied out of C++ in two FFI calls up front (cxx
    /// cannot hand back a live C++ iterator), so each item owns its `Vec`
    /// rather than borrowing from the sketch. Iteration order is unspecified
    /// for an update sketch; compact it with `ordered = true` for
    /// hash-ordered iteration.
    pub fn entries(&self) -> impl Iterator<Item = (u64, Vec<f64>)> {
        let num_values = self.inner.get_num_values() as usize;
        let hashes: Vec<u64> = self.inner.entry_hashes().into_iter().collect();
        let values: Vec<f64> = self.inner.entry_values().into_iter().collect();
        let grouped: Vec<Vec<f64>> = if num_values == 0 {
            Vec::new()
        } else {
            values.chunks(num_values).map(|c| c.to_vec()).collect()
        };
        hashes.into_iter().zip(grouped)
    }
}
```

- [ ] **Step 11: Wire up the safe-crate module**

Create `apache-datasketches/src/tuple/mod.rs`:

```rust
//! ArrayOfDoubles Tuple sketch family: cardinality estimation where each
//! retained key also carries a fixed-width array of `f64` values, summed on
//! collision.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::tuple::ArrayOfDoublesSketchBuilder;
//!
//! let mut sketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build()?;
//! sketch.update_u64(42, &[1.0, 2.5])?;
//! println!("estimate: {}", sketch.get_estimate());
//! # Ok(())
//! # }
//! ```
//!
//! - [`ArrayOfDoublesSketch`] / [`ArrayOfDoublesSketchBuilder`] — the
//!   updatable sketch.

mod builder;
mod sketch;

pub use builder::{ArrayOfDoublesSketchBuilder, ResizeFactor};
pub use sketch::ArrayOfDoublesSketch;
```

Add to `apache-datasketches/src/lib.rs`, after the `cpc` module declaration:

```rust

#[cfg(feature = "tuple")]
pub mod tuple;
```

- [ ] **Step 12: Run the smoke test to verify it passes**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_sketch_smoke_test`
Expected: PASS (6 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no `missing_docs` warnings.

- [ ] **Step 13: Commit**

```bash
git add apache-datasketches-sys/cpp/tuple apache-datasketches-sys/src/array_of_doubles_sketch.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/tests/array_of_doubles_sketch_link_test.rs apache-datasketches-sys/Cargo.toml apache-datasketches/src/tuple apache-datasketches/src/lib.rs apache-datasketches/tests/tuple_sketch_smoke_test.rs apache-datasketches/Cargo.toml
git commit -m "feat(tuple): add ArrayOfDoublesSketch and its builder"
```

---

### Task 3: `CompactArrayOfDoublesSketch`, `ArrayOfDoublesSketch::compact`, and serialization

The immutable, serializable snapshot. This task also retrofits `compact()` onto the sketch from Task 2 (both the C++ shim and the safe wrapper), following the same split Theta uses: the free function `array_of_doubles_sketch_compact` lives in the compact shim, and `ArrayOfDoublesSketchShim::compact` just forwards to it.

Upstream's `compact_array_tuple_sketch` defines its own non-templated `serialize()`/`deserialize()` overloads that shadow the generic `SerDe`-based ones in `compact_tuple_sketch`, so there is exactly one serialization format and no `serde<array<double>>` specialization is needed.

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_compact_shim.h`
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_compact_shim.cc`
- Create: `apache-datasketches-sys/src/array_of_doubles_compact.rs`
- Create (test): `apache-datasketches-sys/tests/array_of_doubles_compact_link_test.rs`
- Create: `apache-datasketches/src/tuple/compact.rs`
- Create (test): `apache-datasketches/tests/tuple_compact_smoke_test.rs`
- Modify: `apache-datasketches-sys/cpp/tuple/array_of_doubles_sketch_shim.h` (add forward decl + `compact` declaration)
- Modify: `apache-datasketches-sys/cpp/tuple/array_of_doubles_sketch_shim.cc` (add include + `compact` definition)
- Modify: `apache-datasketches-sys/src/array_of_doubles_sketch.rs` (add the compact type alias + `compact` method)
- Modify: `apache-datasketches-sys/src/lib.rs`
- Modify: `apache-datasketches/src/tuple/mod.rs`
- Modify: `apache-datasketches/src/tuple/sketch.rs` (add `compact`)
- Modify: `apache-datasketches-sys/Cargo.toml`, `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: `ArrayOfDoublesSketchShim` and its `inner()` accessor (Task 2); `sys::ArrayOfDoublesSketchShim`; `crate::tuple::ArrayOfDoublesSketch`.
- Produces:
  - C++: `apache_datasketches_rs::CompactArrayOfDoublesSketchShim` with `const datasketches::compact_array_of_doubles_sketch& inner() const`; free functions `std::unique_ptr<CompactArrayOfDoublesSketchShim> array_of_doubles_sketch_compact(const ArrayOfDoublesSketchShim&, bool ordered)` and `std::unique_ptr<CompactArrayOfDoublesSketchShim> compact_array_of_doubles_sketch_deserialize(rust::Slice<const uint8_t>)`.
  - Rust (sys): `apache_datasketches_sys::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim`.
  - Rust (safe): `apache_datasketches::tuple::CompactArrayOfDoublesSketch` with `pub(crate) inner: UniquePtr<sys::CompactArrayOfDoublesSketchShim>`, `pub(crate) fn from_shim(UniquePtr<sys::CompactArrayOfDoublesSketchShim>) -> Self`, `pub fn deserialize(&[u8]) -> Result<Self, SketchError>`, `pub fn serialize(&self) -> Vec<u8>`, the same eight queries as the sketch, `get_num_values() -> u8`, and `entries() -> impl Iterator<Item = (u64, Vec<f64>)>`. Plus `ArrayOfDoublesSketch::compact(&self, ordered: bool) -> CompactArrayOfDoublesSketch`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/array_of_doubles_compact_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_compact::ffi as compact_ffi;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;

#[test]
fn compact_via_free_function_and_method() {
    let mut sketch =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &[1.0, 2.0]);
    }

    let via_free_fn = compact_ffi::array_of_doubles_sketch_compact(&sketch, true);
    assert!((via_free_fn.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(via_free_fn.get_num_values(), 2);
    assert!(via_free_fn.is_ordered());

    let via_method = sketch.compact(true);
    assert_eq!(via_method.get_num_retained(), via_free_fn.get_num_retained());
}

#[test]
fn serialize_deserialize_round_trip() {
    let mut sketch =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &[1.0, 2.0]);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize();
    assert!(!bytes.is_empty());

    let restored = compact_ffi::compact_array_of_doubles_sketch_deserialize(&bytes).unwrap();
    assert_eq!(restored.get_num_retained(), compact.get_num_retained());
    assert_eq!(restored.get_num_values(), compact.get_num_values());
    assert_eq!(restored.get_estimate(), compact.get_estimate());
    // Collect into std Vecs: cxx::Vec does not implement PartialEq.
    let restored_hashes: Vec<u64> = restored.entry_hashes().into_iter().collect();
    let compact_hashes: Vec<u64> = compact.entry_hashes().into_iter().collect();
    assert_eq!(restored_hashes, compact_hashes);
    let restored_values: Vec<f64> = restored.entry_values().into_iter().collect();
    let compact_values: Vec<f64> = compact.entry_values().into_iter().collect();
    assert_eq!(restored_values, compact_values);
}

#[test]
fn deserialize_garbage_returns_err() {
    let bytes = [0u8; 8];
    assert!(compact_ffi::compact_array_of_doubles_sketch_deserialize(&bytes).is_err());
}
```

Register it in `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "array_of_doubles_compact_link_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_compact_link_test`
Expected: FAIL — compile error, `unresolved import apache_datasketches_sys::array_of_doubles_compact`.

- [ ] **Step 3: Write the compact shim header**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_compact_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "array_of_doubles_sketch_shim.h"

namespace apache_datasketches_rs {

class CompactArrayOfDoublesSketchShim {
public:
  explicit CompactArrayOfDoublesSketchShim(datasketches::compact_array_of_doubles_sketch sketch);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;
  uint8_t get_num_values() const;

  rust::Vec<uint64_t> entry_hashes() const;
  rust::Vec<double> entry_values() const;

  rust::Vec<uint8_t> serialize() const;

  const datasketches::compact_array_of_doubles_sketch& inner() const { return sketch_; }

private:
  datasketches::compact_array_of_doubles_sketch sketch_;
};

// Used by array_of_doubles_sketch_shim.cc to implement
// ArrayOfDoublesSketchShim::compact().
std::unique_ptr<CompactArrayOfDoublesSketchShim> array_of_doubles_sketch_compact(const ArrayOfDoublesSketchShim& sketch, bool ordered);

std::unique_ptr<CompactArrayOfDoublesSketchShim> compact_array_of_doubles_sketch_deserialize(rust::Slice<const uint8_t> bytes);

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the compact shim implementation**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_compact_shim.cc`:

```cpp
#include "array_of_doubles_compact_shim.h"

namespace apache_datasketches_rs {

CompactArrayOfDoublesSketchShim::CompactArrayOfDoublesSketchShim(datasketches::compact_array_of_doubles_sketch sketch)
  : sketch_(std::move(sketch)) {}

double CompactArrayOfDoublesSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double CompactArrayOfDoublesSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double CompactArrayOfDoublesSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool CompactArrayOfDoublesSketchShim::is_empty() const { return sketch_.is_empty(); }
bool CompactArrayOfDoublesSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool CompactArrayOfDoublesSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double CompactArrayOfDoublesSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t CompactArrayOfDoublesSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }
uint8_t CompactArrayOfDoublesSketchShim::get_num_values() const { return sketch_.get_num_values(); }

rust::Vec<uint64_t> CompactArrayOfDoublesSketchShim::entry_hashes() const {
  rust::Vec<uint64_t> out;
  for (const auto& entry : sketch_) out.push_back(entry.first);
  return out;
}

rust::Vec<double> CompactArrayOfDoublesSketchShim::entry_values() const {
  rust::Vec<double> out;
  for (const auto& entry : sketch_) {
    for (uint8_t i = 0; i < entry.second.size(); ++i) out.push_back(entry.second[i]);
  }
  return out;
}

rust::Vec<uint8_t> CompactArrayOfDoublesSketchShim::serialize() const {
  auto bytes = sketch_.serialize();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> array_of_doubles_sketch_compact(const ArrayOfDoublesSketchShim& sketch, bool ordered) {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(sketch.inner().compact(ordered));
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> compact_array_of_doubles_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(
      datasketches::compact_array_of_doubles_sketch::deserialize(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 5: Add `compact` to the sketch shim**

In `apache-datasketches-sys/cpp/tuple/array_of_doubles_sketch_shim.h`, after the `enum class TupleResizeFactor : std::uint8_t;` line insert the forward declaration:

```cpp
// Forward declaration; defined in array_of_doubles_compact_shim.h.
class CompactArrayOfDoublesSketchShim;
```

and in the `ArrayOfDoublesSketchShim` public section, immediately after `uint8_t get_num_values() const;` insert:

```cpp
  std::unique_ptr<CompactArrayOfDoublesSketchShim> compact(bool ordered) const;
```

In `apache-datasketches-sys/cpp/tuple/array_of_doubles_sketch_shim.cc`, change the include block at the top to:

```cpp
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"
#include "array_of_doubles_sketch.rs.h" // generated by cxx from src/array_of_doubles_sketch.rs; provides the full TupleResizeFactor enum definition
```

and add this definition just before the closing `} // namespace apache_datasketches_rs`:

```cpp
std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesSketchShim::compact(bool ordered) const {
  return array_of_doubles_sketch_compact(*this, ordered);
}
```

- [ ] **Step 6: Write the compact bridge module and add `compact` to the sketch bridge**

Create `apache-datasketches-sys/src/array_of_doubles_compact.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");

        type ArrayOfDoublesSketchShim = crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;

        type CompactArrayOfDoublesSketchShim;

        fn array_of_doubles_sketch_compact(sketch: &ArrayOfDoublesSketchShim, ordered: bool) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn compact_array_of_doubles_sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<CompactArrayOfDoublesSketchShim>>;

        fn get_estimate(self: &CompactArrayOfDoublesSketchShim) -> f64;
        fn get_lower_bound(self: &CompactArrayOfDoublesSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &CompactArrayOfDoublesSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &CompactArrayOfDoublesSketchShim) -> bool;
        fn is_estimation_mode(self: &CompactArrayOfDoublesSketchShim) -> bool;
        fn is_ordered(self: &CompactArrayOfDoublesSketchShim) -> bool;
        fn get_theta(self: &CompactArrayOfDoublesSketchShim) -> f64;
        fn get_num_retained(self: &CompactArrayOfDoublesSketchShim) -> u32;
        fn get_num_values(self: &CompactArrayOfDoublesSketchShim) -> u8;

        fn entry_hashes(self: &CompactArrayOfDoublesSketchShim) -> Vec<u64>;
        fn entry_values(self: &CompactArrayOfDoublesSketchShim) -> Vec<f64>;

        fn serialize(self: &CompactArrayOfDoublesSketchShim) -> Vec<u8>;
    }
}
```

In `apache-datasketches-sys/src/array_of_doubles_sketch.rs`, add the `include!` and type alias, and the `compact` method. The `unsafe extern "C++"` block's opening lines become:

```rust
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");

        type ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim = crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;
```

and add as the final line inside that block, after `fn entry_values(...)`:

```rust
        fn compact(self: &ArrayOfDoublesSketchShim, ordered: bool) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
```

Add to `apache-datasketches-sys/src/lib.rs`, after the `array_of_doubles_sketch` declaration:

```rust
#[cfg(feature = "tuple")]
pub mod array_of_doubles_compact;
```

- [ ] **Step 7: Run the link test to verify it passes**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_compact_link_test`
Expected: PASS (3 tests).

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_sketch_link_test`
Expected: still PASS (4 tests).

- [ ] **Step 8: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_compact_smoke_test.rs`:

```rust
use apache_datasketches::tuple::{ArrayOfDoublesSketchBuilder, CompactArrayOfDoublesSketch};

fn build_sketch(num_values: u8, keys: std::ops::Range<u64>) -> apache_datasketches::tuple::ArrayOfDoublesSketch {
    let mut sketch = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    let values: Vec<f64> = (0..num_values).map(|i| (i + 1) as f64).collect();
    for key in keys {
        sketch.update_u64(key, &values).unwrap();
    }
    sketch
}

#[test]
fn compact_preserves_estimate_and_num_values() {
    let sketch = build_sketch(2, 0..1000);
    let compact = sketch.compact(true);
    assert!((compact.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(compact.get_num_values(), 2);
    assert_eq!(compact.get_num_retained(), 1000);
    assert!(compact.is_ordered());
    assert!(!compact.is_empty());
    assert!(!compact.is_estimation_mode());
    assert_eq!(compact.get_theta(), 1.0);
    assert!(compact.get_lower_bound(1).unwrap() <= compact.get_estimate());
    assert!(compact.get_upper_bound(1).unwrap() >= compact.get_estimate());
}

#[test]
fn serialize_deserialize_round_trip() {
    let sketch = build_sketch(3, 0..500);
    let compact = sketch.compact(true);
    let bytes = compact.serialize();
    let restored = CompactArrayOfDoublesSketch::deserialize(&bytes).unwrap();
    assert_eq!(restored.get_num_values(), 3);
    assert_eq!(restored.get_num_retained(), compact.get_num_retained());
    let before: Vec<(u64, Vec<f64>)> = compact.entries().collect();
    let after: Vec<(u64, Vec<f64>)> = restored.entries().collect();
    assert_eq!(before, after);
}

#[test]
fn deserialize_garbage_is_err() {
    assert!(CompactArrayOfDoublesSketch::deserialize(&[0u8; 8]).is_err());
}

#[test]
fn ordered_entries_are_sorted_by_hash() {
    let sketch = build_sketch(1, 0..200);
    let compact = sketch.compact(true);
    let hashes: Vec<u64> = compact.entries().map(|(h, _)| h).collect();
    assert_eq!(hashes.len(), 200);
    let mut sorted = hashes.clone();
    sorted.sort_unstable();
    assert_eq!(hashes, sorted);
}

#[test]
fn compact_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<CompactArrayOfDoublesSketch>();
}
```

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "tuple_compact_smoke_test"
required-features = ["tuple"]
```

- [ ] **Step 9: Run the smoke test to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_compact_smoke_test`
Expected: FAIL — compile error, `no CompactArrayOfDoublesSketch in apache_datasketches::tuple`.

- [ ] **Step 10: Write the safe-crate compact wrapper**

Create `apache-datasketches/src/tuple/compact.rs`:

```rust
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_compact::ffi as sys;
use cxx::UniquePtr;

/// An immutable, serializable snapshot of an ArrayOfDoubles Tuple sketch.
/// Produced by [`super::ArrayOfDoublesSketch::compact`], by any set
/// operation's result, or by [`Self::deserialize`].
pub struct CompactArrayOfDoublesSketch {
    pub(crate) inner: UniquePtr<sys::CompactArrayOfDoublesSketchShim>,
}

unsafe impl Send for CompactArrayOfDoublesSketch {}

impl CompactArrayOfDoublesSketch {
    pub(crate) fn from_shim(inner: UniquePtr<sys::CompactArrayOfDoublesSketchShim>) -> Self {
        Self { inner }
    }

    /// Deserializes bytes produced by [`Self::serialize`]. Returns
    /// [`SketchError::Deserialization`] if the bytes are truncated, corrupt,
    /// or not an ArrayOfDoubles sketch.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::compact_array_of_doubles_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Serializes this sketch. Unlike Theta, this family has exactly one
    /// serialization format upstream — there is no compressed variant — and
    /// no `ordered` parameter: orderedness is fixed when the snapshot was
    /// created (e.g. via
    /// [`ArrayOfDoublesSketch::compact`](super::ArrayOfDoublesSketch::compact)).
    pub fn serialize(&self) -> Vec<u8> {
        self.inner.serialize()
    }

    /// Returns the current estimate of the number of distinct keys in this
    /// sketch.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`]. See
    /// [`ArrayOfDoublesSketch::get_lower_bound`](super::ArrayOfDoublesSketch::get_lower_bound)
    /// for the meaning of `num_std_dev`.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See
    /// [`ArrayOfDoublesSketch::get_lower_bound`](super::ArrayOfDoublesSketch::get_lower_bound)
    /// for the meaning of `num_std_dev`.
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if this sketch represents an empty set.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if this sketch's theta threshold is below `1.0`
    /// (i.e. [`Self::get_estimate`] is a statistical estimate rather than an
    /// exact count).
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if this sketch's retained entries are sorted by hash
    /// value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold (`1.0` if not in estimation mode).
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of entries retained by this sketch.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }

    /// Returns the fixed number of `f64` values each retained entry carries.
    pub fn get_num_values(&self) -> u8 {
        self.inner.get_num_values()
    }

    /// Iterates the retained entries as `(hash, values)` pairs, where
    /// `values.len() == self.get_num_values()`. Ordered by hash if
    /// [`Self::is_ordered`] is `true`.
    ///
    /// The entries are copied out of C++ in two FFI calls up front (cxx
    /// cannot hand back a live C++ iterator), so each item owns its `Vec`
    /// rather than borrowing from the sketch.
    pub fn entries(&self) -> impl Iterator<Item = (u64, Vec<f64>)> {
        let num_values = self.inner.get_num_values() as usize;
        let hashes: Vec<u64> = self.inner.entry_hashes().into_iter().collect();
        let values: Vec<f64> = self.inner.entry_values().into_iter().collect();
        let grouped: Vec<Vec<f64>> = if num_values == 0 {
            Vec::new()
        } else {
            values.chunks(num_values).map(|c| c.to_vec()).collect()
        };
        hashes.into_iter().zip(grouped)
    }
}
```

- [ ] **Step 11: Add `compact` to the safe-crate sketch and export the new type**

In `apache-datasketches/src/tuple/sketch.rs`, extend the struct's doc comment so it reads:

```rust
/// A mutable, update-only ArrayOfDoubles Tuple sketch: estimates the number
/// of distinct keys added via `update_*`, and carries a fixed-width array of
/// `f64` values per retained key, summed on collision. Build one with
/// [`ArrayOfDoublesSketchBuilder`](super::ArrayOfDoublesSketchBuilder).
///
/// Call [`Self::compact`] to produce an immutable, serializable
/// [`super::CompactArrayOfDoublesSketch`] snapshot for storage, transmission,
/// or use as input to a set operation.
pub struct ArrayOfDoublesSketch {
```

and add this method at the end of the `impl ArrayOfDoublesSketch` block:

```rust
    /// Produces an immutable, serializable
    /// [`super::CompactArrayOfDoublesSketch`] snapshot of this sketch's
    /// current state. If `ordered` is `true`, the snapshot's entries are
    /// sorted by hash value.
    pub fn compact(&self, ordered: bool) -> super::CompactArrayOfDoublesSketch {
        super::CompactArrayOfDoublesSketch::from_shim(self.inner.compact(ordered))
    }
```

In `apache-datasketches/src/tuple/mod.rs`, add `compact` to the module list and re-exports, and extend the doc bullet list:

```rust
//! - [`ArrayOfDoublesSketch`] / [`ArrayOfDoublesSketchBuilder`] — the
//!   updatable sketch.
//! - [`CompactArrayOfDoublesSketch`] — an immutable, serializable snapshot
//!   produced by `ArrayOfDoublesSketch::compact` or by a set operation's
//!   result.

mod builder;
mod compact;
mod sketch;

pub use builder::{ArrayOfDoublesSketchBuilder, ResizeFactor};
pub use compact::CompactArrayOfDoublesSketch;
pub use sketch::ArrayOfDoublesSketch;
```

- [ ] **Step 12: Run the smoke tests to verify they pass**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS — `tuple_sketch_smoke_test` (6 tests) and `tuple_compact_smoke_test` (5 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

- [ ] **Step 13: Commit**

```bash
git add apache-datasketches-sys/cpp/tuple apache-datasketches-sys/src apache-datasketches-sys/tests apache-datasketches-sys/Cargo.toml apache-datasketches/src/tuple apache-datasketches/tests/tuple_compact_smoke_test.rs apache-datasketches/Cargo.toml
git commit -m "feat(tuple): add CompactArrayOfDoublesSketch with serialization and entry iteration"
```

---

### Task 4: Input dispatch and `ArrayOfDoublesUnion`

Introduces the two-variant dispatch machinery (so every set operation can accept either sketch type through one method) plus the union itself. Union sums values on collision, via upstream's `default_array_of_doubles_union_policy`.

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_union_shim.h`
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_union_shim.cc`
- Create: `apache-datasketches-sys/src/array_of_doubles_union.rs`
- Create: `apache-datasketches-sys/src/array_of_doubles_input.rs`
- Create (test): `apache-datasketches-sys/tests/array_of_doubles_union_link_test.rs`
- Create: `apache-datasketches/src/tuple/input.rs`
- Create: `apache-datasketches/src/tuple/union.rs`
- Create (test): `apache-datasketches/tests/tuple_union_smoke_test.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Modify: `apache-datasketches/src/tuple/mod.rs`
- Modify: `apache-datasketches-sys/Cargo.toml`, `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: `ArrayOfDoublesSketchShim`, `CompactArrayOfDoublesSketchShim`, `to_cpp_tuple_resize_factor`, `TupleResizeFactor`, `crate::tuple::{ArrayOfDoublesSketch, CompactArrayOfDoublesSketch, ResizeFactor}`.
- Produces:
  - Rust (sys): `apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef<'a>` with variants `Sketch(&'a ArrayOfDoublesSketchShim)` and `Compact(&'a CompactArrayOfDoublesSketchShim)`; `apache_datasketches_sys::array_of_doubles_union::ffi::ArrayOfDoublesUnionShim`.
  - Rust (safe): sealed trait `apache_datasketches::tuple::ArrayOfDoublesInput` with `fn as_input(&self) -> ArrayOfDoublesInputRef<'_>` (hidden) and `fn get_num_values(&self) -> u8`; `ArrayOfDoublesUnionBuilder` (`lg_k`/`resize_factor`/`p`/`num_values`, `build() -> Result<ArrayOfDoublesUnion, SketchError>`); `ArrayOfDoublesUnion` with `update(&mut self, &impl ArrayOfDoublesInput) -> Result<(), SketchError>`, `get_result(&self, ordered: bool) -> CompactArrayOfDoublesSketch`, `reset(&mut self)`, `get_num_values(&self) -> u8`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/array_of_doubles_union_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;
use apache_datasketches_sys::array_of_doubles_union::ffi as union_ffi;

fn sketch(keys: std::ops::Range<u64>) -> cxx::UniquePtr<sketch_ffi::ArrayOfDoublesSketchShim> {
    let mut s =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1)
            .unwrap();
    for key in keys {
        s.pin_mut().update_u64(key, &[1.0]);
    }
    s
}

#[test]
fn union_half_overlap() {
    let a = sketch(0..1000);
    let b = sketch(500..1500);
    let mut u =
        union_ffi::new_array_of_doubles_union(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_sketch(&b);
    let result = u.get_result(true);
    assert_eq!(result.get_estimate(), 1500.0);
    assert_eq!(result.get_num_values(), 1);
}

#[test]
fn union_accepts_compact_and_resets() {
    let a = sketch(0..100);
    let compact = a.compact(true);
    let mut u =
        union_ffi::new_array_of_doubles_union(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    u.pin_mut().update_with_compact(&compact);
    assert_eq!(u.get_result(true).get_estimate(), 100.0);
    u.pin_mut().reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn union_sums_values_on_collision() {
    let mut a =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    a.pin_mut().update_u64(1, &[1.0, 10.0]);
    let mut b =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    b.pin_mut().update_u64(1, &[2.0, 20.0]);

    let mut u =
        union_ffi::new_array_of_doubles_union(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_sketch(&b);
    let result = u.get_result(true);
    assert_eq!(result.get_num_retained(), 1);
    let values: Vec<f64> = result.entry_values().into_iter().collect();
    assert_eq!(values, vec![3.0, 30.0]);
}

#[test]
fn invalid_lg_k_returns_err() {
    assert!(
        union_ffi::new_array_of_doubles_union(4, sketch_ffi::TupleResizeFactor::X8, 1.0, 1).is_err()
    );
}
```

Register it in `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "array_of_doubles_union_link_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_union_link_test`
Expected: FAIL — compile error, `unresolved import apache_datasketches_sys::array_of_doubles_union`.

- [ ] **Step 3: Write the union shim header**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_union_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"

namespace apache_datasketches_rs {

// Forward declarations only — TupleResizeFactor's definition comes from the
// cxx-generated header, and to_cpp_tuple_resize_factor is defined once in
// array_of_doubles_sketch_shim.cc. Both translation units end up in the same
// static library, so one definition satisfies the ODR. Same pattern as
// theta_union_shim.h.
enum class TupleResizeFactor : std::uint8_t;
datasketches::resize_factor to_cpp_tuple_resize_factor(TupleResizeFactor rf);

class ArrayOfDoublesUnionShim {
public:
  ArrayOfDoublesUnionShim(uint8_t lg_k, datasketches::resize_factor rf, float p, uint8_t num_values);

  void update_with_sketch(const ArrayOfDoublesSketchShim& sketch);
  void update_with_compact(const CompactArrayOfDoublesSketchShim& sketch);

  std::unique_ptr<CompactArrayOfDoublesSketchShim> get_result(bool ordered) const;
  void reset();

private:
  datasketches::array_of_doubles_union union_;
};

std::unique_ptr<ArrayOfDoublesUnionShim> new_array_of_doubles_union(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values);

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the union shim implementation**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_union_shim.cc`:

```cpp
#include "array_of_doubles_union_shim.h"

namespace apache_datasketches_rs {

ArrayOfDoublesUnionShim::ArrayOfDoublesUnionShim(uint8_t lg_k, datasketches::resize_factor rf, float p, uint8_t num_values)
  : union_(datasketches::array_of_doubles_union::builder(
               datasketches::default_array_of_doubles_union_policy(num_values))
               .set_lg_k(lg_k)
               .set_resize_factor(rf)
               .set_p(p)
               .build()) {}

void ArrayOfDoublesUnionShim::update_with_sketch(const ArrayOfDoublesSketchShim& sketch) {
  union_.update(sketch.inner());
}

void ArrayOfDoublesUnionShim::update_with_compact(const CompactArrayOfDoublesSketchShim& sketch) {
  union_.update(sketch.inner());
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesUnionShim::get_result(bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(union_.get_result(ordered));
}

void ArrayOfDoublesUnionShim::reset() { union_.reset(); }

std::unique_ptr<ArrayOfDoublesUnionShim> new_array_of_doubles_union(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values) {
  return std::make_unique<ArrayOfDoublesUnionShim>(lg_k, to_cpp_tuple_resize_factor(rf), p, num_values);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 5: Write the union bridge and the sys-side input enum**

Create `apache-datasketches-sys/src/array_of_doubles_union.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");
        include!("array_of_doubles_union_shim.h");

        type ArrayOfDoublesSketchShim = crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim = crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;
        type TupleResizeFactor = crate::array_of_doubles_sketch::ffi::TupleResizeFactor;

        type ArrayOfDoublesUnionShim;

        fn new_array_of_doubles_union(lg_k: u8, rf: TupleResizeFactor, p: f32, num_values: u8) -> Result<UniquePtr<ArrayOfDoublesUnionShim>>;

        fn update_with_sketch(self: Pin<&mut ArrayOfDoublesUnionShim>, sketch: &ArrayOfDoublesSketchShim);
        fn update_with_compact(self: Pin<&mut ArrayOfDoublesUnionShim>, sketch: &CompactArrayOfDoublesSketchShim);

        fn get_result(self: &ArrayOfDoublesUnionShim, ordered: bool) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn reset(self: Pin<&mut ArrayOfDoublesUnionShim>);
    }
}
```

Create `apache-datasketches-sys/src/array_of_doubles_input.rs`:

```rust
use crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;
use crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;

/// A borrowed reference to one of the two ArrayOfDoubles sketch shim types,
/// used to dispatch set-operation and Jaccard-similarity calls to the correct
/// concrete C++ overload. This is a plain Rust enum rather than a
/// `#[cxx::bridge]` type: `cxx` only supports C-like (payload-free) shared
/// enums, and this enum's variants carry borrowed references to two unrelated
/// opaque C++ types, which cxx cannot express directly.
pub enum ArrayOfDoublesInputRef<'a> {
    /// A mutable, update-only sketch.
    Sketch(&'a ArrayOfDoublesSketchShim),
    /// An immutable, compact sketch.
    Compact(&'a CompactArrayOfDoublesSketchShim),
}
```

Add to `apache-datasketches-sys/src/lib.rs`, after the `array_of_doubles_compact` declaration:

```rust
#[cfg(feature = "tuple")]
pub mod array_of_doubles_input;
#[cfg(feature = "tuple")]
pub mod array_of_doubles_union;
```

- [ ] **Step 6: Run the link test to verify it passes**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_union_link_test`
Expected: PASS (4 tests).

- [ ] **Step 7: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_union_smoke_test.rs`:

```rust
use apache_datasketches::tuple::{
    ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnion,
    ArrayOfDoublesUnionBuilder,
};

fn sketch(num_values: u8, keys: std::ops::Range<u64>) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    let values: Vec<f64> = (0..num_values).map(|i| (i + 1) as f64).collect();
    for key in keys {
        s.update_u64(key, &values).unwrap();
    }
    s
}

#[test]
fn union_half_overlap() {
    let a = sketch(1, 0..1000);
    let b = sketch(1, 500..1500);
    let mut u = ArrayOfDoublesUnionBuilder::new().lg_k(12).build().unwrap();
    u.update(&a).unwrap();
    u.update(&b).unwrap();
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);
}

#[test]
fn union_accepts_both_input_types() {
    let a = sketch(2, 0..100);
    let b = sketch(2, 50..150).compact(true);
    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    u.update(&a).unwrap();
    u.update(&b).unwrap();
    let result = u.get_result(true);
    assert_eq!(result.get_estimate(), 150.0);
    assert_eq!(result.get_num_values(), 2);
}

#[test]
fn union_sums_values_on_collision() {
    let mut a = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    a.update_u64(1, &[1.0, 10.0]).unwrap();
    let mut b = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    b.update_u64(1, &[2.0, 20.0]).unwrap();

    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    u.update(&a).unwrap();
    u.update(&b).unwrap();
    let entries: Vec<(u64, Vec<f64>)> = u.get_result(true).entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, vec![3.0, 30.0]);
}

#[test]
fn union_reset_empties_result() {
    let a = sketch(1, 0..100);
    let mut u = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    u.update(&a).unwrap();
    assert!(!u.get_result(true).is_empty());
    u.reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn mismatched_num_values_is_err() {
    let a = sketch(3, 0..10);
    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    assert_eq!(u.get_num_values(), 2);
    assert!(u.update(&a).is_err());
    assert!(u.update(&a.compact(true)).is_err());
}

#[test]
fn invalid_config_is_err() {
    assert!(ArrayOfDoublesUnionBuilder::new().lg_k(4).build().is_err());
    assert!(ArrayOfDoublesUnionBuilder::new().num_values(0).build().is_err());
}

#[test]
fn union_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<ArrayOfDoublesUnion>();
}
```

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "tuple_union_smoke_test"
required-features = ["tuple"]
```

- [ ] **Step 8: Run the smoke test to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_union_smoke_test`
Expected: FAIL — compile error, `no ArrayOfDoublesUnion in apache_datasketches::tuple`.

- [ ] **Step 9: Write the safe-crate input trait**

Create `apache-datasketches/src/tuple/input.rs`:

```rust
use super::{ArrayOfDoublesSketch, CompactArrayOfDoublesSketch};
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ArrayOfDoublesSketch {}
    impl Sealed for super::CompactArrayOfDoublesSketch {}
}

/// Either ArrayOfDoubles sketch type can be fed into any of this module's set
/// operations — union, intersection, a-not-b, and Jaccard similarity. This
/// trait is sealed — it cannot be implemented outside this crate — since the
/// set-op shims only have concrete overloads for these two exact types.
pub trait ArrayOfDoublesInput: sealed::Sealed {
    #[doc(hidden)]
    fn as_input(&self) -> ArrayOfDoublesInputRef<'_>;

    /// The fixed number of `f64` values each of this sketch's retained
    /// entries carries. Set operations require all operands to agree.
    fn get_num_values(&self) -> u8;
}

impl ArrayOfDoublesInput for ArrayOfDoublesSketch {
    fn as_input(&self) -> ArrayOfDoublesInputRef<'_> {
        ArrayOfDoublesInputRef::Sketch(&self.inner)
    }

    fn get_num_values(&self) -> u8 {
        ArrayOfDoublesSketch::get_num_values(self)
    }
}

impl ArrayOfDoublesInput for CompactArrayOfDoublesSketch {
    fn as_input(&self) -> ArrayOfDoublesInputRef<'_> {
        ArrayOfDoublesInputRef::Compact(&self.inner)
    }

    fn get_num_values(&self) -> u8 {
        CompactArrayOfDoublesSketch::get_num_values(self)
    }
}
```

Note the fully-qualified inner calls: both types have an inherent `get_num_values` with the same name as the trait method, and the inherent method must be the one that is called (a bare `self.get_num_values()` inside the trait impl would resolve to the inherent method too, but qualifying it makes that explicit and immune to future refactors).

- [ ] **Step 10: Write the safe-crate union wrapper**

Create `apache-datasketches/src/tuple/union.rs`:

```rust
use super::input::ArrayOfDoublesInput;
use super::{CompactArrayOfDoublesSketch, ResizeFactor};
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;
use apache_datasketches_sys::array_of_doubles_union::ffi as sys;
use cxx::UniquePtr;

/// Builder for [`ArrayOfDoublesUnion`], mirroring upstream's
/// `array_of_doubles_union::builder`. `lg_k` defaults to `12`,
/// `resize_factor` to [`ResizeFactor::X8`], `p` to `1.0` (no sampling), and
/// `num_values` to `1`. As with
/// [`ArrayOfDoublesSketchBuilder`](super::ArrayOfDoublesSketchBuilder), the
/// seed is never exposed.
#[derive(Debug, Clone, Copy)]
pub struct ArrayOfDoublesUnionBuilder {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
    num_values: u8,
}

impl Default for ArrayOfDoublesUnionBuilder {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::default(),
            p: 1.0,
            num_values: 1,
        }
    }
}

impl ArrayOfDoublesUnionBuilder {
    /// Creates a new builder with default settings (`lg_k = 12`,
    /// `resize_factor = X8`, `p = 1.0`, `num_values = 1`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the target number of retained entries in the
    /// union's result.
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Sets the hash table's growth [`ResizeFactor`].
    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    /// Sets the sampling probability. `1.0` (the default) disables sampling.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Sets the fixed number of `f64` values per entry. Must be at least `1`,
    /// and must match every sketch later passed to
    /// [`ArrayOfDoublesUnion::update`].
    pub fn num_values(mut self, num_values: u8) -> Self {
        self.num_values = num_values;
        self
    }

    /// Builds the union. Returns [`SketchError::InvalidConfig`] if `lg_k` is
    /// out of range, `p` is outside `(0, 1]`, or `num_values` is `0`.
    pub fn build(self) -> Result<ArrayOfDoublesUnion, SketchError> {
        if self.num_values == 0 {
            return Err(SketchError::InvalidConfig(
                "num_values must be at least 1".to_string(),
            ));
        }
        let inner = sys::new_array_of_doubles_union(
            self.lg_k,
            self.resize_factor.into(),
            self.p,
            self.num_values,
        )
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(ArrayOfDoublesUnion {
            inner,
            num_values: self.num_values,
        })
    }
}

/// A streaming union accumulator over ArrayOfDoubles sketches. Values are
/// summed per index when the same key appears in more than one input, using
/// upstream's `default_array_of_doubles_union_policy`.
///
/// Accepts either [`super::ArrayOfDoublesSketch`] or
/// [`CompactArrayOfDoublesSketch`] via the sealed [`ArrayOfDoublesInput`]
/// trait.
pub struct ArrayOfDoublesUnion {
    inner: UniquePtr<sys::ArrayOfDoublesUnionShim>,
    num_values: u8,
}

unsafe impl Send for ArrayOfDoublesUnion {}

impl ArrayOfDoublesUnion {
    /// Merges the given sketch into this union's running result.
    ///
    /// Returns [`SketchError::InvalidConfig`] if the sketch's `num_values`
    /// differs from this union's. Upstream does not validate this itself —
    /// merging mismatched array widths would read and write past the shorter
    /// array's bounds rather than error — so the check happens here, before
    /// the sketch crosses the FFI boundary.
    pub fn update(&mut self, input: &impl ArrayOfDoublesInput) -> Result<(), SketchError> {
        let actual = input.get_num_values();
        if actual != self.num_values {
            return Err(SketchError::InvalidConfig(format!(
                "num_values mismatch: union has {}, input has {actual}",
                self.num_values
            )));
        }
        match input.as_input() {
            ArrayOfDoublesInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ArrayOfDoublesInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
        }
        Ok(())
    }

    /// Returns the union's current result as a
    /// [`CompactArrayOfDoublesSketch`]. If `ordered` is `true`, the result's
    /// entries are sorted by hash value.
    pub fn get_result(&self, ordered: bool) -> CompactArrayOfDoublesSketch {
        CompactArrayOfDoublesSketch::from_shim(self.inner.get_result(ordered))
    }

    /// Resets this union to its initial, empty state. `num_values` is
    /// preserved.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns the fixed number of `f64` values per entry this union was
    /// built with. Every input passed to [`Self::update`] must match it.
    pub fn get_num_values(&self) -> u8 {
        self.num_values
    }
}
```

- [ ] **Step 11: Export the new items**

In `apache-datasketches/src/tuple/mod.rs`, extend the doc bullet list with:

```rust
//! - [`ArrayOfDoublesUnion`] / [`ArrayOfDoublesUnionBuilder`] — merges
//!   multiple sketches, summing values per index on collision.
//!
//! [`ArrayOfDoublesSketch`] and [`CompactArrayOfDoublesSketch`] can both be
//! passed interchangeably (via the sealed [`ArrayOfDoublesInput`] trait) to
//! every set operation in this module.
```

and update the module/export lists to:

```rust
mod builder;
mod compact;
mod input;
mod sketch;
mod union;

pub use builder::{ArrayOfDoublesSketchBuilder, ResizeFactor};
pub use compact::CompactArrayOfDoublesSketch;
pub use input::ArrayOfDoublesInput;
pub use sketch::ArrayOfDoublesSketch;
pub use union::{ArrayOfDoublesUnion, ArrayOfDoublesUnionBuilder};
```

- [ ] **Step 12: Run the smoke tests to verify they pass**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS — including `tuple_union_smoke_test` (7 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

- [ ] **Step 13: Commit**

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add ArrayOfDoublesUnion and two-variant input dispatch"
```

---

### Task 5: `ArrayOfDoublesIntersection`

Plain constructor taking `num_values` (no builder), matching `ThetaIntersection`'s precedent. Upstream ships **no** default policy for array-tuple intersection — the `Policy` template parameter has no default at all — so the shim explicitly instantiates `array_of_doubles_intersection<default_array_of_doubles_union_policy>`, i.e. sum-on-collision. This is exactly what upstream's own test file does, with the comment "there is no default policy for intersection ... let's combine values the same way as in union for testing".

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_intersection_shim.h`
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_intersection_shim.cc`
- Create: `apache-datasketches-sys/src/array_of_doubles_intersection.rs`
- Create (test): `apache-datasketches-sys/tests/array_of_doubles_intersection_link_test.rs`
- Create: `apache-datasketches/src/tuple/intersection.rs`
- Create (test): `apache-datasketches/tests/tuple_intersection_smoke_test.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`, `apache-datasketches/src/tuple/mod.rs`
- Modify: `apache-datasketches-sys/Cargo.toml`, `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: `ArrayOfDoublesSketchShim`, `CompactArrayOfDoublesSketchShim`, `ArrayOfDoublesInput`, `ArrayOfDoublesInputRef`.
- Produces:
  - C++: `apache_datasketches_rs::ArrayOfDoublesIntersectionShim`; free function `std::unique_ptr<ArrayOfDoublesIntersectionShim> new_array_of_doubles_intersection(uint8_t num_values)`. Also the type alias `apache_datasketches_rs::aod_intersection` (used again by the jaccard shim in Task 7).
  - Rust (sys): `apache_datasketches_sys::array_of_doubles_intersection::ffi::ArrayOfDoublesIntersectionShim`.
  - Rust (safe): `apache_datasketches::tuple::ArrayOfDoublesIntersection` with `new(num_values: u8) -> Result<Self, SketchError>`, `update(&mut self, &impl ArrayOfDoublesInput) -> Result<(), SketchError>`, `get_result(&self, ordered: bool) -> Result<CompactArrayOfDoublesSketch, SketchError>`, `has_result(&self) -> bool`, `get_num_values(&self) -> u8`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/array_of_doubles_intersection_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_intersection::ffi as intersection_ffi;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;

fn sketch(
    num_values: u8,
    keys: std::ops::Range<u64>,
    values: &[f64],
) -> cxx::UniquePtr<sketch_ffi::ArrayOfDoublesSketchShim> {
    let mut s = sketch_ffi::new_array_of_doubles_sketch(
        12,
        sketch_ffi::TupleResizeFactor::X8,
        1.0,
        num_values,
    )
    .unwrap();
    for key in keys {
        s.pin_mut().update_u64(key, values);
    }
    s
}

#[test]
fn intersection_half_overlap() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 500..1500, &[1.0]);
    let mut i = intersection_ffi::new_array_of_doubles_intersection(1);
    assert!(!i.has_result());
    i.pin_mut().update_with_sketch(&a);
    i.pin_mut().update_with_sketch(&b);
    assert!(i.has_result());
    let result = i.get_result(true).unwrap();
    assert_eq!(result.get_estimate(), 500.0);
    assert_eq!(result.get_num_values(), 1);
}

#[test]
fn intersection_sums_values_on_collision() {
    let a = sketch(2, 0..1, &[1.0, 10.0]);
    let b = sketch(2, 0..1, &[2.0, 20.0]);
    let mut i = intersection_ffi::new_array_of_doubles_intersection(2);
    i.pin_mut().update_with_sketch(&a);
    i.pin_mut().update_with_compact(&b.compact(true));
    let result = i.get_result(true).unwrap();
    assert_eq!(result.get_num_retained(), 1);
    let values: Vec<f64> = result.entry_values().into_iter().collect();
    assert_eq!(values, vec![3.0, 30.0]);
}

#[test]
fn get_result_without_update_returns_err() {
    let i = intersection_ffi::new_array_of_doubles_intersection(1);
    assert!(!i.has_result());
    assert!(i.get_result(true).is_err());
}
```

Register it in `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "array_of_doubles_intersection_link_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_intersection_link_test`
Expected: FAIL — compile error, `unresolved import apache_datasketches_sys::array_of_doubles_intersection`.

- [ ] **Step 3: Write the intersection shim header**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_intersection_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"

namespace apache_datasketches_rs {

// Upstream's array_of_doubles_intersection has NO default Policy template
// argument ("no default policy since it is not clear in general", per its own
// header). v1 picks sum-on-collision, reusing the union's policy — the same
// choice upstream's own array_of_doubles_sketch_test.cpp makes. This alias is
// also reused by the jaccard shim.
using aod_intersection =
    datasketches::array_of_doubles_intersection<datasketches::default_array_of_doubles_union_policy>;

class ArrayOfDoublesIntersectionShim {
public:
  explicit ArrayOfDoublesIntersectionShim(uint8_t num_values);

  void update_with_sketch(const ArrayOfDoublesSketchShim& sketch);
  void update_with_compact(const CompactArrayOfDoublesSketchShim& sketch);

  std::unique_ptr<CompactArrayOfDoublesSketchShim> get_result(bool ordered) const;
  bool has_result() const;

private:
  aod_intersection intersection_;
};

std::unique_ptr<ArrayOfDoublesIntersectionShim> new_array_of_doubles_intersection(uint8_t num_values);

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the intersection shim implementation**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_intersection_shim.cc`:

```cpp
#include "array_of_doubles_intersection_shim.h"

namespace apache_datasketches_rs {

// The policy — and therefore num_values — must be supplied at construction
// time; unlike the sketch and union there is no builder for this type.
ArrayOfDoublesIntersectionShim::ArrayOfDoublesIntersectionShim(uint8_t num_values)
  : intersection_(datasketches::DEFAULT_SEED,
                  datasketches::default_array_of_doubles_union_policy(num_values)) {}

void ArrayOfDoublesIntersectionShim::update_with_sketch(const ArrayOfDoublesSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

void ArrayOfDoublesIntersectionShim::update_with_compact(const CompactArrayOfDoublesSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesIntersectionShim::get_result(bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(intersection_.get_result(ordered));
}

bool ArrayOfDoublesIntersectionShim::has_result() const { return intersection_.has_result(); }

std::unique_ptr<ArrayOfDoublesIntersectionShim> new_array_of_doubles_intersection(uint8_t num_values) {
  return std::make_unique<ArrayOfDoublesIntersectionShim>(num_values);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 5: Write the intersection bridge**

Create `apache-datasketches-sys/src/array_of_doubles_intersection.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");
        include!("array_of_doubles_intersection_shim.h");

        type ArrayOfDoublesSketchShim = crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim = crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;

        type ArrayOfDoublesIntersectionShim;

        fn new_array_of_doubles_intersection(num_values: u8) -> UniquePtr<ArrayOfDoublesIntersectionShim>;

        fn update_with_sketch(self: Pin<&mut ArrayOfDoublesIntersectionShim>, sketch: &ArrayOfDoublesSketchShim);
        fn update_with_compact(self: Pin<&mut ArrayOfDoublesIntersectionShim>, sketch: &CompactArrayOfDoublesSketchShim);

        fn get_result(self: &ArrayOfDoublesIntersectionShim, ordered: bool) -> Result<UniquePtr<CompactArrayOfDoublesSketchShim>>;
        fn has_result(self: &ArrayOfDoublesIntersectionShim) -> bool;
    }
}
```

Add to `apache-datasketches-sys/src/lib.rs`, after the `array_of_doubles_union` declaration:

```rust
#[cfg(feature = "tuple")]
pub mod array_of_doubles_intersection;
```

- [ ] **Step 6: Run the link test to verify it passes**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_intersection_link_test`
Expected: PASS (3 tests).

- [ ] **Step 7: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_intersection_smoke_test.rs`:

```rust
use apache_datasketches::tuple::{
    ArrayOfDoublesIntersection, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder,
};
use apache_datasketches::SketchError;

fn sketch(num_values: u8, keys: std::ops::Range<u64>, values: &[f64]) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    for key in keys {
        s.update_u64(key, values).unwrap();
    }
    s
}

#[test]
fn intersection_half_overlap() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 500..1500, &[1.0]);
    let mut i = ArrayOfDoublesIntersection::new(1).unwrap();
    i.update(&a).unwrap();
    i.update(&b).unwrap();
    assert_eq!(i.get_result(true).unwrap().get_estimate(), 500.0);
}

#[test]
fn intersection_accepts_both_input_types_and_sums_values() {
    let a = sketch(2, 0..1, &[1.0, 10.0]);
    let b = sketch(2, 0..1, &[2.0, 20.0]).compact(true);
    let mut i = ArrayOfDoublesIntersection::new(2).unwrap();
    i.update(&a).unwrap();
    i.update(&b).unwrap();
    let entries: Vec<(u64, Vec<f64>)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, vec![3.0, 30.0]);
}

#[test]
fn get_result_before_update_is_empty_intersection_err() {
    let i = ArrayOfDoublesIntersection::new(1).unwrap();
    assert!(!i.has_result());
    assert!(matches!(
        i.get_result(true),
        Err(SketchError::EmptyIntersection)
    ));
}

#[test]
fn mismatched_num_values_is_err() {
    let a = sketch(3, 0..10, &[1.0, 2.0, 3.0]);
    let mut i = ArrayOfDoublesIntersection::new(2).unwrap();
    assert_eq!(i.get_num_values(), 2);
    assert!(i.update(&a).is_err());
    assert!(i.update(&a.compact(true)).is_err());
}

#[test]
fn num_values_zero_is_err() {
    assert!(ArrayOfDoublesIntersection::new(0).is_err());
}

#[test]
fn intersection_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<ArrayOfDoublesIntersection>();
}
```

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "tuple_intersection_smoke_test"
required-features = ["tuple"]
```

- [ ] **Step 8: Run the smoke test to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_intersection_smoke_test`
Expected: FAIL — compile error, `no ArrayOfDoublesIntersection in apache_datasketches::tuple`.

- [ ] **Step 9: Write the safe-crate intersection wrapper**

Create `apache-datasketches/src/tuple/intersection.rs`:

```rust
use super::input::ArrayOfDoublesInput;
use super::CompactArrayOfDoublesSketch;
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;
use apache_datasketches_sys::array_of_doubles_intersection::ffi as sys;
use cxx::UniquePtr;

/// Computes the intersection of ArrayOfDoubles sketches fed via
/// [`Self::update`]. Values are summed per index for keys present in every
/// input.
///
/// Unlike [`super::ArrayOfDoublesUnion`] there is no builder — upstream's
/// `array_of_doubles_intersection` has a plain constructor, and the
/// intersecting universe is defined entirely by the sketches passed to
/// `update`. Only `num_values` (which the combine policy needs at
/// construction time) must be supplied up front.
///
/// Upstream ships no default combine policy for this type; v1 uses
/// sum-on-collision, mirroring the union's policy. Additional policies
/// (min/max, etc.) can be added later without changing this type's shape.
pub struct ArrayOfDoublesIntersection {
    inner: UniquePtr<sys::ArrayOfDoublesIntersectionShim>,
    num_values: u8,
}

unsafe impl Send for ArrayOfDoublesIntersection {}

impl ArrayOfDoublesIntersection {
    /// Creates a new intersection accumulator for sketches carrying
    /// `num_values` values per entry, with no result yet — call
    /// [`Self::update`] at least once before [`Self::get_result`].
    ///
    /// Returns [`SketchError::InvalidConfig`] if `num_values` is `0`.
    pub fn new(num_values: u8) -> Result<Self, SketchError> {
        if num_values == 0 {
            return Err(SketchError::InvalidConfig(
                "num_values must be at least 1".to_string(),
            ));
        }
        Ok(Self {
            inner: sys::new_array_of_doubles_intersection(num_values),
            num_values,
        })
    }

    /// Narrows this intersection's running result to also require membership
    /// in the given sketch. The first call establishes the initial universe;
    /// each subsequent call intersects further.
    ///
    /// Returns [`SketchError::InvalidConfig`] if the sketch's `num_values`
    /// differs from this intersection's — upstream does not validate this
    /// itself, and mismatched widths would read and write out of bounds.
    pub fn update(&mut self, input: &impl ArrayOfDoublesInput) -> Result<(), SketchError> {
        let actual = input.get_num_values();
        if actual != self.num_values {
            return Err(SketchError::InvalidConfig(format!(
                "num_values mismatch: intersection has {}, input has {actual}",
                self.num_values
            )));
        }
        match input.as_input() {
            ArrayOfDoublesInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ArrayOfDoublesInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
        }
        Ok(())
    }

    /// Returns the current intersection result as a
    /// [`CompactArrayOfDoublesSketch`], or
    /// [`SketchError::EmptyIntersection`] if [`Self::update`] has never been
    /// called. If `ordered` is `true`, the result's entries are sorted by
    /// hash value.
    pub fn get_result(&self, ordered: bool) -> Result<CompactArrayOfDoublesSketch, SketchError> {
        if !self.inner.has_result() {
            return Err(SketchError::EmptyIntersection);
        }
        let inner = self
            .inner
            .get_result(ordered)
            .map_err(|e| SketchError::Cpp(e.what().to_string()))?;
        Ok(CompactArrayOfDoublesSketch::from_shim(inner))
    }

    /// Returns `true` if [`Self::update`] has been called at least once.
    pub fn has_result(&self) -> bool {
        self.inner.has_result()
    }

    /// Returns the fixed number of `f64` values per entry this intersection
    /// was created with. Every input passed to [`Self::update`] must match it.
    pub fn get_num_values(&self) -> u8 {
        self.num_values
    }
}
```

- [ ] **Step 10: Export the new type**

In `apache-datasketches/src/tuple/mod.rs`, add the doc bullet:

```rust
//! - [`ArrayOfDoublesIntersection`] — computes the intersection of sketches
//!   fed via `update`, summing values per index.
```

(insert it directly after the `ArrayOfDoublesUnion` bullet, before the trailing "can both be passed interchangeably" paragraph), and update the module/export lists:

```rust
mod builder;
mod compact;
mod input;
mod intersection;
mod sketch;
mod union;

pub use builder::{ArrayOfDoublesSketchBuilder, ResizeFactor};
pub use compact::CompactArrayOfDoublesSketch;
pub use input::ArrayOfDoublesInput;
pub use intersection::ArrayOfDoublesIntersection;
pub use sketch::ArrayOfDoublesSketch;
pub use union::{ArrayOfDoublesUnion, ArrayOfDoublesUnionBuilder};
```

- [ ] **Step 11: Run the smoke tests to verify they pass**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS — including `tuple_intersection_smoke_test` (6 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

- [ ] **Step 12: Commit**

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add ArrayOfDoublesIntersection with sum combine policy"
```

---

### Task 6: `ArrayOfDoublesAnotB`

Stateless set difference. Upstream's `array_of_doubles_a_not_b::compute` is a template over both operand types, so the shim provides the four concrete `compute_<a>_<b>` overloads and the safe wrapper dispatches over them with a single `compute(a, b, ordered)`.

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_a_not_b_shim.h`
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_a_not_b_shim.cc`
- Create: `apache-datasketches-sys/src/array_of_doubles_a_not_b.rs`
- Create (test): `apache-datasketches-sys/tests/array_of_doubles_a_not_b_link_test.rs`
- Create: `apache-datasketches/src/tuple/a_not_b.rs`
- Create (test): `apache-datasketches/tests/tuple_a_not_b_smoke_test.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`, `apache-datasketches/src/tuple/mod.rs`
- Modify: `apache-datasketches-sys/Cargo.toml`, `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: `ArrayOfDoublesSketchShim`, `CompactArrayOfDoublesSketchShim`, `ArrayOfDoublesInput`, `ArrayOfDoublesInputRef`.
- Produces:
  - C++: `apache_datasketches_rs::ArrayOfDoublesAnotBShim` with `compute_sketch_sketch`, `compute_sketch_compact`, `compute_compact_sketch`, `compute_compact_compact`, each `(a, b, bool ordered) const`; free function `new_array_of_doubles_a_not_b()`.
  - Rust (safe): `apache_datasketches::tuple::ArrayOfDoublesAnotB` with `new() -> Self`, `Default`, and `compute(&self, a: &impl ArrayOfDoublesInput, b: &impl ArrayOfDoublesInput, ordered: bool) -> Result<CompactArrayOfDoublesSketch, SketchError>`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/array_of_doubles_a_not_b_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_a_not_b::ffi as a_not_b_ffi;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;

fn sketch(keys: std::ops::Range<u64>) -> cxx::UniquePtr<sketch_ffi::ArrayOfDoublesSketchShim> {
    let mut s =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1)
            .unwrap();
    for key in keys {
        s.pin_mut().update_u64(key, &[1.0]);
    }
    s
}

#[test]
fn a_not_b_half_overlap_all_four_combinations() {
    let a = sketch(0..1000);
    let b = sketch(500..1500);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc = a_not_b_ffi::new_array_of_doubles_a_not_b();

    assert_eq!(calc.compute_sketch_sketch(&a, &b, true).get_estimate(), 500.0);
    assert_eq!(calc.compute_sketch_compact(&a, &cb, true).get_estimate(), 500.0);
    assert_eq!(calc.compute_compact_sketch(&ca, &b, true).get_estimate(), 500.0);
    assert_eq!(calc.compute_compact_compact(&ca, &cb, true).get_estimate(), 500.0);
}

#[test]
fn a_not_b_preserves_num_values_and_values() {
    let mut a =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    a.pin_mut().update_u64(1, &[5.0, 6.0]);
    let b = sketch_ffi::new_array_of_doubles_sketch(
        12,
        sketch_ffi::TupleResizeFactor::X8,
        1.0,
        2,
    )
    .unwrap();

    let calc = a_not_b_ffi::new_array_of_doubles_a_not_b();
    let result = calc.compute_sketch_sketch(&a, &b, true);
    assert_eq!(result.get_num_values(), 2);
    assert_eq!(result.get_num_retained(), 1);
    let values: Vec<f64> = result.entry_values().into_iter().collect();
    assert_eq!(values, vec![5.0, 6.0]);
}
```

Register it in `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "array_of_doubles_a_not_b_link_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_a_not_b_link_test`
Expected: FAIL — compile error, `unresolved import apache_datasketches_sys::array_of_doubles_a_not_b`.

- [ ] **Step 3: Write the a-not-b shim header**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_a_not_b_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"

namespace apache_datasketches_rs {

class ArrayOfDoublesAnotBShim {
public:
  ArrayOfDoublesAnotBShim();

  std::unique_ptr<CompactArrayOfDoublesSketchShim> compute_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactArrayOfDoublesSketchShim> compute_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactArrayOfDoublesSketchShim> compute_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactArrayOfDoublesSketchShim> compute_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b, bool ordered) const;

private:
  datasketches::array_of_doubles_a_not_b a_not_b_;
};

std::unique_ptr<ArrayOfDoublesAnotBShim> new_array_of_doubles_a_not_b();

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the a-not-b shim implementation**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_a_not_b_shim.cc`:

```cpp
#include "array_of_doubles_a_not_b_shim.h"

namespace apache_datasketches_rs {

ArrayOfDoublesAnotBShim::ArrayOfDoublesAnotBShim() : a_not_b_() {}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesAnotBShim::compute_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b, bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesAnotBShim::compute_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b, bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesAnotBShim::compute_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b, bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesAnotBShim::compute_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b, bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<ArrayOfDoublesAnotBShim> new_array_of_doubles_a_not_b() {
  return std::make_unique<ArrayOfDoublesAnotBShim>();
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 5: Write the a-not-b bridge**

Create `apache-datasketches-sys/src/array_of_doubles_a_not_b.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");
        include!("array_of_doubles_a_not_b_shim.h");

        type ArrayOfDoublesSketchShim = crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim = crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;

        type ArrayOfDoublesAnotBShim;

        fn new_array_of_doubles_a_not_b() -> UniquePtr<ArrayOfDoublesAnotBShim>;

        fn compute_sketch_sketch(self: &ArrayOfDoublesAnotBShim, a: &ArrayOfDoublesSketchShim, b: &ArrayOfDoublesSketchShim, ordered: bool) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn compute_sketch_compact(self: &ArrayOfDoublesAnotBShim, a: &ArrayOfDoublesSketchShim, b: &CompactArrayOfDoublesSketchShim, ordered: bool) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn compute_compact_sketch(self: &ArrayOfDoublesAnotBShim, a: &CompactArrayOfDoublesSketchShim, b: &ArrayOfDoublesSketchShim, ordered: bool) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn compute_compact_compact(self: &ArrayOfDoublesAnotBShim, a: &CompactArrayOfDoublesSketchShim, b: &CompactArrayOfDoublesSketchShim, ordered: bool) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
    }
}
```

Add to `apache-datasketches-sys/src/lib.rs`, after the `array_of_doubles_intersection` declaration:

```rust
#[cfg(feature = "tuple")]
pub mod array_of_doubles_a_not_b;
```

- [ ] **Step 6: Run the link test to verify it passes**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_a_not_b_link_test`
Expected: PASS (2 tests).

- [ ] **Step 7: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_a_not_b_smoke_test.rs`:

```rust
use apache_datasketches::tuple::{
    ArrayOfDoublesAnotB, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder,
};

fn sketch(num_values: u8, keys: std::ops::Range<u64>, values: &[f64]) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    for key in keys {
        s.update_u64(key, values).unwrap();
    }
    s
}

#[test]
fn a_not_b_half_overlap_all_four_combinations() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 500..1500, &[1.0]);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc = ArrayOfDoublesAnotB::new();

    assert_eq!(calc.compute(&a, &b, true).unwrap().get_estimate(), 500.0);
    assert_eq!(calc.compute(&a, &cb, true).unwrap().get_estimate(), 500.0);
    assert_eq!(calc.compute(&ca, &b, true).unwrap().get_estimate(), 500.0);
    assert_eq!(calc.compute(&ca, &cb, true).unwrap().get_estimate(), 500.0);
}

#[test]
fn a_not_b_preserves_values() {
    let a = sketch(2, 0..1, &[5.0, 6.0]);
    let b = sketch(2, 100..101, &[1.0, 1.0]);
    let calc = ArrayOfDoublesAnotB::new();
    let result = calc.compute(&a, &b, true).unwrap();
    let entries: Vec<(u64, Vec<f64>)> = result.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, vec![5.0, 6.0]);
    assert_eq!(result.get_num_values(), 2);
}

#[test]
fn mismatched_num_values_is_err() {
    let a = sketch(2, 0..10, &[1.0, 2.0]);
    let b = sketch(3, 0..10, &[1.0, 2.0, 3.0]);
    let calc = ArrayOfDoublesAnotB::new();
    assert!(calc.compute(&a, &b, true).is_err());
    assert!(calc.compute(&b, &a, true).is_err());
}

#[test]
fn a_not_b_is_send_and_default() {
    fn assert_send<T: Send>() {}
    assert_send::<ArrayOfDoublesAnotB>();
    let _ = ArrayOfDoublesAnotB::default();
}
```

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "tuple_a_not_b_smoke_test"
required-features = ["tuple"]
```

- [ ] **Step 8: Run the smoke test to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_a_not_b_smoke_test`
Expected: FAIL — compile error, `no ArrayOfDoublesAnotB in apache_datasketches::tuple`.

- [ ] **Step 9: Write the safe-crate a-not-b wrapper**

Create `apache-datasketches/src/tuple/a_not_b.rs`:

```rust
use super::input::ArrayOfDoublesInput;
use super::CompactArrayOfDoublesSketch;
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_a_not_b::ffi as sys;
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;
use cxx::UniquePtr;

/// Computes the set difference ("A not B": keys in `a` but not `b`) of two
/// ArrayOfDoubles sketches via [`Self::compute`]. Retained entries keep `a`'s
/// values unchanged. Stateless between calls — unlike
/// [`super::ArrayOfDoublesUnion`]/[`super::ArrayOfDoublesIntersection`], there
/// is no accumulation across repeated calls.
pub struct ArrayOfDoublesAnotB {
    inner: UniquePtr<sys::ArrayOfDoublesAnotBShim>,
}

unsafe impl Send for ArrayOfDoublesAnotB {}

impl Default for ArrayOfDoublesAnotB {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrayOfDoublesAnotB {
    /// Creates a new, reusable a-not-b calculator.
    pub fn new() -> Self {
        Self {
            inner: sys::new_array_of_doubles_a_not_b(),
        }
    }

    /// Computes the set difference `a - b` (keys in `a` that are not in `b`)
    /// as a [`CompactArrayOfDoublesSketch`]. `a` and `b` may independently be
    /// a [`super::ArrayOfDoublesSketch`] or a
    /// [`CompactArrayOfDoublesSketch`]. If `ordered` is `true`, the result's
    /// entries are sorted by hash value.
    ///
    /// Returns [`SketchError::InvalidConfig`] if `a` and `b` disagree on
    /// `num_values` — upstream does not validate this itself, and mismatched
    /// widths would read out of bounds.
    pub fn compute(
        &self,
        a: &impl ArrayOfDoublesInput,
        b: &impl ArrayOfDoublesInput,
        ordered: bool,
    ) -> Result<CompactArrayOfDoublesSketch, SketchError> {
        let (a_num, b_num) = (a.get_num_values(), b.get_num_values());
        if a_num != b_num {
            return Err(SketchError::InvalidConfig(format!(
                "num_values mismatch: a has {a_num}, b has {b_num}"
            )));
        }
        let inner = match (a.as_input(), b.as_input()) {
            (ArrayOfDoublesInputRef::Sketch(a), ArrayOfDoublesInputRef::Sketch(b)) => {
                self.inner.compute_sketch_sketch(a, b, ordered)
            }
            (ArrayOfDoublesInputRef::Sketch(a), ArrayOfDoublesInputRef::Compact(b)) => {
                self.inner.compute_sketch_compact(a, b, ordered)
            }
            (ArrayOfDoublesInputRef::Compact(a), ArrayOfDoublesInputRef::Sketch(b)) => {
                self.inner.compute_compact_sketch(a, b, ordered)
            }
            (ArrayOfDoublesInputRef::Compact(a), ArrayOfDoublesInputRef::Compact(b)) => {
                self.inner.compute_compact_compact(a, b, ordered)
            }
        };
        Ok(CompactArrayOfDoublesSketch::from_shim(inner))
    }
}
```

- [ ] **Step 10: Export the new type**

In `apache-datasketches/src/tuple/mod.rs`, add the doc bullet after the intersection bullet:

```rust
//! - [`ArrayOfDoublesAnotB`] — computes the set difference (keys in `a` but
//!   not `b`), preserving `a`'s values.
```

and update the module/export lists:

```rust
mod a_not_b;
mod builder;
mod compact;
mod input;
mod intersection;
mod sketch;
mod union;

pub use a_not_b::ArrayOfDoublesAnotB;
pub use builder::{ArrayOfDoublesSketchBuilder, ResizeFactor};
pub use compact::CompactArrayOfDoublesSketch;
pub use input::ArrayOfDoublesInput;
pub use intersection::ArrayOfDoublesIntersection;
pub use sketch::ArrayOfDoublesSketch;
pub use union::{ArrayOfDoublesUnion, ArrayOfDoublesUnionBuilder};
```

- [ ] **Step 11: Run the smoke tests to verify they pass**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS — including `tuple_a_not_b_smoke_test` (4 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

- [ ] **Step 12: Commit**

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add ArrayOfDoublesAnotB set difference"
```

---

### Task 7: `array_of_doubles_jaccard_similarity`

Upstream provides no ready-made `array_of_doubles_jaccard_similarity` alias (unlike Theta's built-in `theta_jaccard_similarity`), so the shim instantiates the existing generic `jaccard_similarity_base<Union, Intersection, ExtractKey>` template with the union and intersection types already in scope. No new upstream algorithm is written — only a template instantiation.

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_jaccard_shim.h`
- Create: `apache-datasketches-sys/cpp/tuple/array_of_doubles_jaccard_shim.cc`
- Create: `apache-datasketches-sys/src/array_of_doubles_jaccard.rs`
- Create (test): `apache-datasketches-sys/tests/array_of_doubles_jaccard_link_test.rs`
- Create: `apache-datasketches/src/tuple/jaccard.rs`
- Create (test): `apache-datasketches/tests/tuple_jaccard_smoke_test.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`, `apache-datasketches/src/tuple/mod.rs`
- Modify: `apache-datasketches-sys/Cargo.toml`, `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: `ArrayOfDoublesSketchShim`, `CompactArrayOfDoublesSketchShim`, the `aod_intersection` alias from `array_of_doubles_intersection_shim.h` (Task 5), `ArrayOfDoublesInput`, `ArrayOfDoublesInputRef`.
- Produces:
  - C++: shared struct `apache_datasketches_rs::TupleJaccardBoundsFfi`; free functions `jaccard_sketch_sketch`, `jaccard_sketch_compact`, `jaccard_compact_sketch`, `jaccard_compact_compact`, each returning `TupleJaccardBoundsFfi`.
  - Rust (safe): `apache_datasketches::tuple::JaccardBounds { lower_bound: f64, estimate: f64, upper_bound: f64 }` and `pub fn array_of_doubles_jaccard_similarity(a: &impl ArrayOfDoublesInput, b: &impl ArrayOfDoublesInput) -> Result<JaccardBounds, SketchError>`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/array_of_doubles_jaccard_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_jaccard::ffi as jaccard_ffi;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;

fn sketch(keys: std::ops::Range<u64>) -> cxx::UniquePtr<sketch_ffi::ArrayOfDoublesSketchShim> {
    let mut s =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1)
            .unwrap();
    for key in keys {
        s.pin_mut().update_u64(key, &[1.0]);
    }
    s
}

#[test]
fn identical_sketches_are_fully_similar() {
    let a = sketch(0..1000);
    let b = sketch(0..1000);
    let bounds = jaccard_ffi::jaccard_sketch_sketch(&a, &b);
    assert_eq!(bounds.estimate, 1.0);
    assert_eq!(bounds.lower_bound, 1.0);
    assert_eq!(bounds.upper_bound, 1.0);
}

#[test]
fn disjoint_sketches_are_dissimilar() {
    let a = sketch(0..1000);
    let b = sketch(2000..3000);
    let bounds = jaccard_ffi::jaccard_sketch_sketch(&a, &b);
    assert_eq!(bounds.estimate, 0.0);
}

#[test]
fn half_overlap_is_about_one_third_all_four_combinations() {
    let a = sketch(0..1000);
    let b = sketch(500..1500);
    let ca = a.compact(true);
    let cb = b.compact(true);

    // |A ∩ B| / |A ∪ B| = 500 / 1500 = 1/3
    for bounds in [
        jaccard_ffi::jaccard_sketch_sketch(&a, &b),
        jaccard_ffi::jaccard_sketch_compact(&a, &cb),
        jaccard_ffi::jaccard_compact_sketch(&ca, &b),
        jaccard_ffi::jaccard_compact_compact(&ca, &cb),
    ] {
        assert!(
            (bounds.estimate - 1.0 / 3.0).abs() < 0.01,
            "estimate was {}",
            bounds.estimate
        );
        assert!(bounds.lower_bound <= bounds.estimate);
        assert!(bounds.upper_bound >= bounds.estimate);
    }
}
```

Register it in `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "array_of_doubles_jaccard_link_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_jaccard_link_test`
Expected: FAIL — compile error, `unresolved import apache_datasketches_sys::array_of_doubles_jaccard`.

- [ ] **Step 3: Write the jaccard shim header**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_jaccard_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "tuple_jaccard_similarity.hpp"
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"
#include "array_of_doubles_intersection_shim.h"

namespace apache_datasketches_rs {

// Named TupleJaccardBoundsFfi rather than JaccardBoundsFfi because cxx emits
// one C++ definition per shared type into this namespace, and the theta bridge
// already emits apache_datasketches_rs::JaccardBoundsFfi there.
struct TupleJaccardBoundsFfi;

TupleJaccardBoundsFfi jaccard_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi jaccard_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi jaccard_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi jaccard_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b);

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the jaccard shim implementation**

Create `apache-datasketches-sys/cpp/tuple/array_of_doubles_jaccard_shim.cc`:

```cpp
#include "array_of_doubles_jaccard_shim.h"
#include "array_of_doubles_jaccard.rs.h"

namespace apache_datasketches_rs {

namespace {

// Upstream ships tuple_jaccard_similarity<Summary, IntersectionPolicy,
// UnionPolicy, Allocator> but no array-of-doubles alias for it, so we
// instantiate the same underlying generic template directly with this
// family's concrete union/intersection types.
//
// Note on num_values: jaccard_similarity_base::jaccard() internally builds a
// scratch union via `typename Union::builder()` and a scratch intersection via
// `Intersection(seed)`, both of which get a default-constructed policy whose
// num_values is 1 regardless of the operand sketches' actual width. That is
// harmless here: jaccard() derives its result solely from
// get_num_retained()/get_theta64()/is_empty() on those scratch results and
// never reads a summary array, so the scratch objects' incomplete per-index
// summing cannot affect the returned bounds.
using aod_jaccard = datasketches::jaccard_similarity_base<
    datasketches::array_of_doubles_union,
    aod_intersection,
    datasketches::pair_extract_key<uint64_t, datasketches::array<double>>>;

TupleJaccardBoundsFfi to_ffi(const std::array<double, 3>& result) {
  return TupleJaccardBoundsFfi{result[0], result[1], result[2]};
}

} // namespace

TupleJaccardBoundsFfi jaccard_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b) {
  return to_ffi(aod_jaccard::jaccard(a.inner(), b.inner()));
}

TupleJaccardBoundsFfi jaccard_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b) {
  return to_ffi(aod_jaccard::jaccard(a.inner(), b.inner()));
}

TupleJaccardBoundsFfi jaccard_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b) {
  return to_ffi(aod_jaccard::jaccard(a.inner(), b.inner()));
}

TupleJaccardBoundsFfi jaccard_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b) {
  return to_ffi(aod_jaccard::jaccard(a.inner(), b.inner()));
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 5: Write the jaccard bridge**

Create `apache-datasketches-sys/src/array_of_doubles_jaccard.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    /// Named `TupleJaccardBoundsFfi` rather than `JaccardBoundsFfi` because
    /// cxx emits one C++ definition per shared type into the bridge namespace,
    /// and the theta bridge already emits
    /// `apache_datasketches_rs::JaccardBoundsFfi`.
    struct TupleJaccardBoundsFfi {
        lower_bound: f64,
        estimate: f64,
        upper_bound: f64,
    }

    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");
        include!("array_of_doubles_jaccard_shim.h");

        type ArrayOfDoublesSketchShim = crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim = crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;

        fn jaccard_sketch_sketch(a: &ArrayOfDoublesSketchShim, b: &ArrayOfDoublesSketchShim) -> TupleJaccardBoundsFfi;
        fn jaccard_sketch_compact(a: &ArrayOfDoublesSketchShim, b: &CompactArrayOfDoublesSketchShim) -> TupleJaccardBoundsFfi;
        fn jaccard_compact_sketch(a: &CompactArrayOfDoublesSketchShim, b: &ArrayOfDoublesSketchShim) -> TupleJaccardBoundsFfi;
        fn jaccard_compact_compact(a: &CompactArrayOfDoublesSketchShim, b: &CompactArrayOfDoublesSketchShim) -> TupleJaccardBoundsFfi;
    }
}
```

Add to `apache-datasketches-sys/src/lib.rs`, after the `array_of_doubles_a_not_b` declaration:

```rust
#[cfg(feature = "tuple")]
pub mod array_of_doubles_jaccard;
```

- [ ] **Step 6: Run the link test to verify it passes**

Run: `cargo test -p apache-datasketches-sys --features tuple --test array_of_doubles_jaccard_link_test`
Expected: PASS (3 tests).

- [ ] **Step 7: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_jaccard_smoke_test.rs`:

```rust
use apache_datasketches::tuple::{
    array_of_doubles_jaccard_similarity, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder,
};

fn sketch(num_values: u8, keys: std::ops::Range<u64>, values: &[f64]) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    for key in keys {
        s.update_u64(key, values).unwrap();
    }
    s
}

#[test]
fn identical_sketches_are_fully_similar() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 0..1000, &[1.0]);
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert_eq!(bounds.estimate, 1.0);
    assert_eq!(bounds.lower_bound, 1.0);
    assert_eq!(bounds.upper_bound, 1.0);
}

#[test]
fn disjoint_sketches_are_dissimilar() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 2000..3000, &[1.0]);
    assert_eq!(array_of_doubles_jaccard_similarity(&a, &b).unwrap().estimate, 0.0);
}

#[test]
fn half_overlap_accepts_all_four_combinations() {
    let a = sketch(2, 0..1000, &[1.0, 2.0]);
    let b = sketch(2, 500..1500, &[1.0, 2.0]);
    let ca = a.compact(true);
    let cb = b.compact(true);

    for bounds in [
        array_of_doubles_jaccard_similarity(&a, &b).unwrap(),
        array_of_doubles_jaccard_similarity(&a, &cb).unwrap(),
        array_of_doubles_jaccard_similarity(&ca, &b).unwrap(),
        array_of_doubles_jaccard_similarity(&ca, &cb).unwrap(),
    ] {
        assert!(
            (bounds.estimate - 1.0 / 3.0).abs() < 0.01,
            "estimate was {}",
            bounds.estimate
        );
        assert!(bounds.lower_bound <= bounds.estimate);
        assert!(bounds.upper_bound >= bounds.estimate);
    }
}

#[test]
fn mismatched_num_values_is_err() {
    let a = sketch(1, 0..10, &[1.0]);
    let b = sketch(2, 0..10, &[1.0, 2.0]);
    assert!(array_of_doubles_jaccard_similarity(&a, &b).is_err());
}
```

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "tuple_jaccard_smoke_test"
required-features = ["tuple"]
```

- [ ] **Step 8: Run the smoke test to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_jaccard_smoke_test`
Expected: FAIL — compile error, `no array_of_doubles_jaccard_similarity in apache_datasketches::tuple`.

- [ ] **Step 9: Write the safe-crate jaccard wrapper**

Create `apache-datasketches/src/tuple/jaccard.rs`:

```rust
use super::input::ArrayOfDoublesInput;
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;
use apache_datasketches_sys::array_of_doubles_jaccard::ffi as sys;

/// The result of [`array_of_doubles_jaccard_similarity`]: a confidence
/// interval around the estimated Jaccard index of two ArrayOfDoubles
/// sketches, in `[0.0, 1.0]`.
///
/// This is a distinct type from the theta module's `JaccardBounds` with the
/// same shape — the two sketch families are independently feature-gated and
/// do not share types. (Deliberately not an intra-doc link: `theta` may not
/// be compiled in.)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JaccardBounds {
    /// Lower bound of the confidence interval around [`Self::estimate`].
    pub lower_bound: f64,
    /// The estimated Jaccard index.
    pub estimate: f64,
    /// Upper bound of the confidence interval around [`Self::estimate`].
    pub upper_bound: f64,
}

impl From<sys::TupleJaccardBoundsFfi> for JaccardBounds {
    fn from(ffi: sys::TupleJaccardBoundsFfi) -> Self {
        Self {
            lower_bound: ffi.lower_bound,
            estimate: ffi.estimate,
            upper_bound: ffi.upper_bound,
        }
    }
}

/// Estimates the Jaccard index (intersection-over-union) of two
/// ArrayOfDoubles sketches, each of which may independently be a
/// [`super::ArrayOfDoublesSketch`] or a
/// [`super::CompactArrayOfDoublesSketch`].
///
/// Only the keys matter — the per-entry values do not affect the result.
///
/// Returns [`SketchError::InvalidConfig`] if the two sketches disagree on
/// `num_values`, for consistency with the other set operations (the
/// underlying computation would tolerate a mismatch, but accepting one here
/// would let a genuine modelling error pass silently).
pub fn array_of_doubles_jaccard_similarity(
    a: &impl ArrayOfDoublesInput,
    b: &impl ArrayOfDoublesInput,
) -> Result<JaccardBounds, SketchError> {
    let (a_num, b_num) = (a.get_num_values(), b.get_num_values());
    if a_num != b_num {
        return Err(SketchError::InvalidConfig(format!(
            "num_values mismatch: a has {a_num}, b has {b_num}"
        )));
    }
    let ffi = match (a.as_input(), b.as_input()) {
        (ArrayOfDoublesInputRef::Sketch(a), ArrayOfDoublesInputRef::Sketch(b)) => {
            sys::jaccard_sketch_sketch(a, b)
        }
        (ArrayOfDoublesInputRef::Sketch(a), ArrayOfDoublesInputRef::Compact(b)) => {
            sys::jaccard_sketch_compact(a, b)
        }
        (ArrayOfDoublesInputRef::Compact(a), ArrayOfDoublesInputRef::Sketch(b)) => {
            sys::jaccard_compact_sketch(a, b)
        }
        (ArrayOfDoublesInputRef::Compact(a), ArrayOfDoublesInputRef::Compact(b)) => {
            sys::jaccard_compact_compact(a, b)
        }
    };
    Ok(ffi.into())
}
```

- [ ] **Step 10: Export the new items**

In `apache-datasketches/src/tuple/mod.rs`, add the doc bullet after the a-not-b bullet:

```rust
//! - [`array_of_doubles_jaccard_similarity`] / [`JaccardBounds`] — estimates
//!   the Jaccard index (intersection-over-union) of two sketches.
```

and update the module/export lists:

```rust
mod a_not_b;
mod builder;
mod compact;
mod input;
mod intersection;
mod jaccard;
mod sketch;
mod union;

pub use a_not_b::ArrayOfDoublesAnotB;
pub use builder::{ArrayOfDoublesSketchBuilder, ResizeFactor};
pub use compact::CompactArrayOfDoublesSketch;
pub use input::ArrayOfDoublesInput;
pub use intersection::ArrayOfDoublesIntersection;
pub use jaccard::{array_of_doubles_jaccard_similarity, JaccardBounds};
pub use sketch::ArrayOfDoublesSketch;
pub use union::{ArrayOfDoublesUnion, ArrayOfDoublesUnionBuilder};
```

- [ ] **Step 11: Run the smoke tests to verify they pass**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS — including `tuple_jaccard_smoke_test` (4 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

- [ ] **Step 12: Commit**

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add array_of_doubles_jaccard_similarity"
```

---

### Task 8: 1:1-ported upstream test suite, validation tests, and concurrency test

Ports upstream's single `array_of_doubles_sketch_test.cpp` (7 `TEST_CASE`s covering sketch, union, intersection, and a-not-b) and adds the Rust-specific tests for the two validations this binding introduces, plus the standard `Send` check.

**Files:**
- Create (test): `apache-datasketches/tests/array_of_doubles_sketch_test.rs`
- Create (test): `apache-datasketches/tests/tuple_validation_test.rs`
- Create (test): `apache-datasketches/tests/tuple_concurrency_test.rs`
- Modify: `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: the complete public API from Tasks 2–7.
- Produces: no new library API.

- [ ] **Step 1: Write the ported upstream test file**

Create `apache-datasketches/tests/array_of_doubles_sketch_test.rs`:

```rust
//! Ported 1:1 from upstream datasketches-cpp's
//! `tuple/test/array_of_doubles_sketch_test.cpp`. Upstream covers the sketch,
//! union, intersection, and a-not-b for this family from that single file
//! (unlike Theta, which has one test file per class), so this file does too.
//!
//! Deviations from upstream, and why:
//!
//! - Upstream has three separate serialization test cases ("stream serialize
//!   deserialize", "bytes to stream serialize deserialize", and "bytes
//!   serialize deserialize") because C++ exposes both `std::ostream` and
//!   byte-vector overloads and both must round-trip through each other. These
//!   bindings expose exactly one byte-oriented `serialize()`/`deserialize()`
//!   pair (there is no stream API to bind — `&[u8]`/`Vec<u8>` is the idiomatic
//!   Rust equivalent and the wire format is identical either way), so the
//!   three cases collapse into the single `serialize_deserialize_estimation_mode`
//!   test below.
//! - Upstream's `builder(2)` relies on implicit conversion from `int` to the
//!   update policy; here that is the explicit `.num_values(2)` builder setter.
//! - Upstream iterates entries with `begin()`/`end()` and reads
//!   `entry.second[i]`; here `entries()` yields owned `(u64, Vec<f64>)` pairs
//!   (cxx cannot hand back a live C++ iterator).
//! - `update()` returns `Result` here because the values-length check has no
//!   C++ exception to delegate to; upstream's equivalent call is infallible.

use apache_datasketches::tuple::{
    ArrayOfDoublesAnotB, ArrayOfDoublesIntersection, ArrayOfDoublesSketch,
    ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder, CompactArrayOfDoublesSketch,
};

/// Upstream: `TEST_CASE("aod sketch: reset")`.
#[test]
fn sketch_reset() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    sketch.update_i32(1, &[1.0]).unwrap();
    assert!(!sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 1);

    sketch.reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 0);
}

/// Upstream: `TEST_CASE("aod sketch: stream serialize deserialize - estimation
/// mode")`, merged with the two byte-oriented serialization cases (see the
/// module comment).
#[test]
fn serialize_deserialize_estimation_mode() {
    let mut update_sketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    for i in 0..8192i32 {
        update_sketch.update_i32(i, &[1.0, 2.0]).unwrap();
    }
    assert!(!update_sketch.is_empty());
    assert!(update_sketch.is_estimation_mode());
    assert_eq!(update_sketch.get_num_values(), 2);

    let compact_sketch = update_sketch.compact(true);
    let bytes = compact_sketch.serialize();
    let deserialized = CompactArrayOfDoublesSketch::deserialize(&bytes).unwrap();

    assert_eq!(deserialized.get_num_values(), compact_sketch.get_num_values());
    assert_eq!(deserialized.is_empty(), compact_sketch.is_empty());
    assert_eq!(deserialized.is_ordered(), compact_sketch.is_ordered());
    assert_eq!(
        deserialized.is_estimation_mode(),
        compact_sketch.is_estimation_mode()
    );
    assert_eq!(deserialized.get_num_retained(), compact_sketch.get_num_retained());
    assert_eq!(deserialized.get_theta(), compact_sketch.get_theta());
    assert_eq!(deserialized.get_estimate(), compact_sketch.get_estimate());
    assert_eq!(
        deserialized.get_lower_bound(1).unwrap(),
        compact_sketch.get_lower_bound(1).unwrap()
    );
    assert_eq!(
        deserialized.get_upper_bound(1).unwrap(),
        compact_sketch.get_upper_bound(1).unwrap()
    );

    // Upstream compares the two sketches entry by entry via parallel
    // iteration, checking hash, value[0], and value[1].
    let expected: Vec<(u64, Vec<f64>)> = compact_sketch.entries().collect();
    let actual: Vec<(u64, Vec<f64>)> = deserialized.entries().collect();
    assert_eq!(expected.len(), compact_sketch.get_num_retained() as usize);
    assert_eq!(expected, actual);
    for (_, values) in &expected {
        assert_eq!(values.as_slice(), &[1.0, 2.0]);
    }

    // Upstream also iterates the update sketch and the compact sketch
    // together; the compact one is ordered, so compare against the update
    // sketch's entries sorted by hash.
    let mut from_update: Vec<(u64, Vec<f64>)> = update_sketch.entries().collect();
    from_update.sort_by_key(|(hash, _)| *hash);
    assert_eq!(from_update, expected);
}

/// Upstream: `TEST_CASE("aod union: half overlap")`.
#[test]
fn union_half_overlap() {
    let mut sketch1 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 0..1000i32 {
        sketch1.update_i32(i, &[1.0]).unwrap();
    }
    let mut sketch2 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 500..1500i32 {
        sketch2.update_i32(i, &[1.0]).unwrap();
    }

    let mut u = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    u.update(&sketch1).unwrap();
    u.update(&sketch2).unwrap();
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);

    u.reset();
    assert!(u.get_result(true).is_empty());
}

/// Upstream: `TEST_CASE("aod intersection: half overlap")`. Upstream notes
/// there is no default intersection policy and picks the union's sum policy
/// for testing; these bindings make that same choice permanently for v1.
#[test]
fn intersection_half_overlap() {
    let mut sketch1 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 0..1000i32 {
        sketch1.update_i32(i, &[1.0]).unwrap();
    }
    let mut sketch2 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 500..1500i32 {
        sketch2.update_i32(i, &[1.0]).unwrap();
    }

    let mut intersection = ArrayOfDoublesIntersection::new(1).unwrap();
    intersection.update(&sketch1).unwrap();
    intersection.update(&sketch2).unwrap();
    assert_eq!(intersection.get_result(true).unwrap().get_estimate(), 500.0);
}

/// Upstream: `TEST_CASE("aod a-not-b: half overlap")`.
#[test]
fn a_not_b_half_overlap() {
    let mut sketch1 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 0..1000i32 {
        sketch1.update_i32(i, &[1.0]).unwrap();
    }
    let mut sketch2 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 500..1500i32 {
        sketch2.update_i32(i, &[1.0]).unwrap();
    }

    let a_not_b = ArrayOfDoublesAnotB::new();
    let result = a_not_b.compute(&sketch1, &sketch2, true).unwrap();
    assert_eq!(result.get_estimate(), 500.0);
}

/// Not from upstream: confirms an empty sketch round-trips, which the
/// serialized format handles via its IS_EMPTY flag byte.
#[test]
fn empty_sketch_round_trips() {
    let sketch: ArrayOfDoublesSketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    let compact = sketch.compact(true);
    assert!(compact.is_empty());
    assert_eq!(compact.get_estimate(), 0.0);

    let restored = CompactArrayOfDoublesSketch::deserialize(&compact.serialize()).unwrap();
    assert!(restored.is_empty());
    assert_eq!(restored.get_num_values(), 2);
    assert_eq!(restored.get_num_retained(), 0);
    assert_eq!(restored.entries().count(), 0);
}
```

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "array_of_doubles_sketch_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the ported test suite**

Run: `cargo test -p apache-datasketches --features tuple --test array_of_doubles_sketch_test`
Expected: PASS (6 tests). The whole public API already exists after Task 7, so this test is expected to pass on the first run — it is a port, not a driver for new code.

- [ ] **Step 3: Write the validation test file**

Create `apache-datasketches/tests/tuple_validation_test.rs`:

```rust
//! The two validations this family adds in Rust because upstream C++ has no
//! equivalent check to delegate to. Both are safety-critical, not merely
//! ergonomic: upstream's update and combine policies index the supplied array
//! blindly for `i in 0..num_values`, so a mismatch would be an out-of-bounds
//! read or write rather than a graceful failure.

use apache_datasketches::tuple::{
    array_of_doubles_jaccard_similarity, ArrayOfDoublesAnotB, ArrayOfDoublesIntersection,
    ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder,
};
use apache_datasketches::SketchError;

fn sketch(num_values: u8) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    let values: Vec<f64> = vec![1.0; num_values as usize];
    for key in 0..10u64 {
        s.update_u64(key, &values).unwrap();
    }
    s
}

#[test]
fn update_with_too_few_values_is_invalid_config() {
    let mut s = ArrayOfDoublesSketchBuilder::new().num_values(3).build().unwrap();
    let err = s.update_u64(1, &[1.0, 2.0]).unwrap_err();
    assert!(matches!(err, SketchError::InvalidConfig(_)));
    // Nothing was added.
    assert!(s.is_empty());
}

#[test]
fn update_with_too_many_values_is_invalid_config() {
    let mut s = ArrayOfDoublesSketchBuilder::new().num_values(3).build().unwrap();
    assert!(matches!(
        s.update_u64(1, &[1.0, 2.0, 3.0, 4.0]).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(s.is_empty());
}

#[test]
fn update_with_empty_slice_is_invalid_config() {
    let mut s = ArrayOfDoublesSketchBuilder::new().num_values(1).build().unwrap();
    assert!(matches!(
        s.update_u64(1, &[]).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
}

#[test]
fn every_update_key_type_validates_length() {
    let mut s = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    let short: &[f64] = &[1.0];
    assert!(s.update_u64(1, short).is_err());
    assert!(s.update_i64(1, short).is_err());
    assert!(s.update_u32(1, short).is_err());
    assert!(s.update_i32(1, short).is_err());
    assert!(s.update_u16(1, short).is_err());
    assert!(s.update_i16(1, short).is_err());
    assert!(s.update_u8(1, short).is_err());
    assert!(s.update_i8(1, short).is_err());
    assert!(s.update_f64(1.0, short).is_err());
    assert!(s.update_str("k", short).is_err());
    assert!(s.update_bytes(b"k", short).is_err());
    assert!(s.is_empty());

    // And the correct length succeeds for each.
    let ok: &[f64] = &[1.0, 2.0];
    assert!(s.update_u64(1, ok).is_ok());
    assert!(s.update_i64(2, ok).is_ok());
    assert!(s.update_u32(3, ok).is_ok());
    assert!(s.update_i32(4, ok).is_ok());
    assert!(s.update_u16(5, ok).is_ok());
    assert!(s.update_i16(6, ok).is_ok());
    assert!(s.update_u8(7, ok).is_ok());
    assert!(s.update_i8(8, ok).is_ok());
    assert!(s.update_f64(9.0, ok).is_ok());
    assert!(s.update_str("k", ok).is_ok());
    assert!(s.update_bytes(b"k2", ok).is_ok());
    assert!(!s.is_empty());
}

#[test]
fn union_rejects_mismatched_num_values() {
    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    let wrong = sketch(3);
    assert!(matches!(
        u.update(&wrong).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(matches!(
        u.update(&wrong.compact(true)).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    // The matching width still works.
    assert!(u.update(&sketch(2)).is_ok());
}

#[test]
fn intersection_rejects_mismatched_num_values() {
    let mut i = ArrayOfDoublesIntersection::new(2).unwrap();
    let wrong = sketch(1);
    assert!(matches!(
        i.update(&wrong).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(matches!(
        i.update(&wrong.compact(true)).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(i.update(&sketch(2)).is_ok());
}

#[test]
fn a_not_b_rejects_mismatched_num_values() {
    let a = sketch(2);
    let b = sketch(4);
    let calc = ArrayOfDoublesAnotB::new();
    assert!(matches!(
        calc.compute(&a, &b, true).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(matches!(
        calc.compute(&a.compact(true), &b.compact(true), true).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(calc.compute(&a, &sketch(2), true).is_ok());
}

#[test]
fn jaccard_rejects_mismatched_num_values() {
    let a = sketch(2);
    let b = sketch(3);
    assert!(matches!(
        array_of_doubles_jaccard_similarity(&a, &b).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(array_of_doubles_jaccard_similarity(&a, &sketch(2)).is_ok());
}

#[test]
fn zero_num_values_is_rejected_everywhere() {
    assert!(ArrayOfDoublesSketchBuilder::new().num_values(0).build().is_err());
    assert!(ArrayOfDoublesUnionBuilder::new().num_values(0).build().is_err());
    assert!(ArrayOfDoublesIntersection::new(0).is_err());
}
```

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "tuple_validation_test"
required-features = ["tuple"]
```

- [ ] **Step 4: Run the validation tests**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_validation_test`
Expected: PASS (9 tests).

- [ ] **Step 5: Write the concurrency test file**

Create `apache-datasketches/tests/tuple_concurrency_test.rs`:

```rust
//! Every ArrayOfDoubles type is `Send` but not `Sync`, matching every other
//! sketch family in this crate: the underlying C++ objects can be moved
//! between threads but have no internal synchronisation for shared access.

use apache_datasketches::tuple::{
    ArrayOfDoublesAnotB, ArrayOfDoublesIntersection, ArrayOfDoublesSketch,
    ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnion, ArrayOfDoublesUnionBuilder,
    CompactArrayOfDoublesSketch,
};

fn assert_send<T: Send>() {}

#[test]
fn all_types_are_send() {
    assert_send::<ArrayOfDoublesSketch>();
    assert_send::<CompactArrayOfDoublesSketch>();
    assert_send::<ArrayOfDoublesUnion>();
    assert_send::<ArrayOfDoublesIntersection>();
    assert_send::<ArrayOfDoublesAnotB>();
}

#[test]
fn sketch_can_be_built_on_one_thread_and_used_on_another() {
    let handle = std::thread::spawn(|| {
        let mut sketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
        for i in 0..1000u64 {
            sketch.update_u64(i, &[1.0, 2.0]).unwrap();
        }
        sketch.compact(true)
    });
    let compact = handle.join().unwrap();

    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    u.update(&compact).unwrap();
    assert!((u.get_result(true).get_estimate() - 1000.0).abs() < 1.0);
}

#[test]
fn per_thread_sketches_merge_correctly() {
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            std::thread::spawn(move || {
                let mut sketch = ArrayOfDoublesSketchBuilder::new().build().unwrap();
                for i in (t * 250)..((t + 1) * 250) {
                    sketch.update_u64(i, &[1.0]).unwrap();
                }
                sketch.compact(true)
            })
        })
        .collect();

    let mut u = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    for handle in handles {
        u.update(&handle.join().unwrap()).unwrap();
    }
    assert_eq!(u.get_result(true).get_estimate(), 1000.0);
}
```

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "tuple_concurrency_test"
required-features = ["tuple"]
```

- [ ] **Step 6: Run the concurrency tests**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_concurrency_test`
Expected: PASS (3 tests).

- [ ] **Step 7: Run every tuple test together**

Run: `cargo test --workspace --features apache-datasketches/tuple`
Expected: all pass — 6 sys-crate link test binaries and 9 safe-crate test binaries.

- [ ] **Step 8: Commit**

```bash
git add apache-datasketches/tests apache-datasketches/Cargo.toml
git commit -m "test(tuple): port upstream array_of_doubles_sketch_test and add validation/concurrency tests"
```

---

### Task 9: Runnable example, crate-level docs, and full-matrix verification

**Files:**
- Create: `apache-datasketches/examples/tuple.rs`
- Modify: `apache-datasketches/Cargo.toml` (add `[[example]]`)
- Modify: `apache-datasketches/src/lib.rs` (crate-level docs)
- Modify: `AGENTS.md`

**Interfaces:**
- Consumes: the complete public API from Tasks 2–7.
- Produces: no new library API.

- [ ] **Step 1: Write the example**

Create `apache-datasketches/examples/tuple.rs`:

```rust
//! Demonstrates the ArrayOfDoubles Tuple sketch family: cardinality
//! estimation where each distinct key also carries a fixed-width array of
//! `f64` values, summed on collision — plus set operations (union,
//! intersection, a-not-b) and Jaccard similarity.
//!
//! Run with:
//!   cargo run --example tuple --features tuple

use apache_datasketches::tuple::{
    array_of_doubles_jaccard_similarity, ArrayOfDoublesAnotB, ArrayOfDoublesIntersection,
    ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder, CompactArrayOfDoublesSketch,
};

fn main() {
    // Two sketches of user IDs, each carrying [sessions, revenue] per user.
    let mut day1 = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();
    for id in 0..10_000u64 {
        day1.update_u64(id, &[1.0, 2.50]).unwrap();
    }

    let mut day2 = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();
    for id in 5_000..15_000u64 {
        day2.update_u64(id, &[1.0, 4.00]).unwrap();
    }

    println!("Day 1 unique users (estimate): {:.0}", day1.get_estimate());
    println!("Day 2 unique users (estimate): {:.0}", day2.get_estimate());
    println!("Values per entry: {}", day1.get_num_values());

    // Union: unique users across both days, with per-user values summed for
    // anyone who appeared on both.
    let mut union = ArrayOfDoublesUnionBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();
    union.update(&day1).unwrap();
    union.update(&day2).unwrap();
    let combined = union.get_result(true);
    println!(
        "Total unique users (union estimate): {:.0}",
        combined.get_estimate()
    );

    // Per-entry access is what distinguishes Tuple sketches from HLL/Theta/CPC:
    // scale the retained sample's revenue back up by 1/theta to estimate the
    // full population total.
    let retained_revenue: f64 = combined.entries().map(|(_, values)| values[1]).sum();
    println!(
        "Estimated total revenue: {:.2} (from {} retained entries, theta = {:.4})",
        retained_revenue / combined.get_theta(),
        combined.get_num_retained(),
        combined.get_theta()
    );

    // Intersection: users who came back on day 2.
    let mut intersection = ArrayOfDoublesIntersection::new(2).unwrap();
    intersection.update(&day1).unwrap();
    intersection.update(&day2).unwrap();
    match intersection.get_result(true) {
        Ok(returning) => println!(
            "Returning users (intersection estimate): {:.0}",
            returning.get_estimate()
        ),
        Err(e) => println!("No intersection result: {e}"),
    }

    // A-not-b: users who only came on day 1.
    let a_not_b = ArrayOfDoublesAnotB::new();
    let day1_only = a_not_b.compute(&day1, &day2, true).unwrap();
    println!(
        "Day-1-only users (a-not-b estimate): {:.0}",
        day1_only.get_estimate()
    );

    // Jaccard similarity of the two days' audiences.
    let similarity = array_of_doubles_jaccard_similarity(&day1, &day2).unwrap();
    println!(
        "Jaccard similarity: {:.3} (range [{:.3}, {:.3}])",
        similarity.estimate, similarity.lower_bound, similarity.upper_bound
    );

    // Serialize a compact sketch for storage/transmission, then restore it.
    let compact = day1.compact(true);
    let bytes = compact.serialize();
    println!("Serialized day-1 sketch: {} bytes", bytes.len());
    let restored = CompactArrayOfDoublesSketch::deserialize(&bytes).unwrap();
    println!(
        "Restored estimate: {:.0} ({} values per entry)",
        restored.get_estimate(),
        restored.get_num_values()
    );
}
```

Register it in `apache-datasketches/Cargo.toml`, alongside the other `[[example]]` blocks:

```toml
[[example]]
name = "tuple"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the example**

Run: `cargo run -p apache-datasketches --example tuple --features tuple`
Expected: prints the estimates without panicking. Day 1 and Day 2 print roughly `10000`; union roughly `15000`; intersection roughly `5000`; a-not-b roughly `5000`; Jaccard estimate roughly `0.333`.

- [ ] **Step 3: Update the crate-level documentation**

In `apache-datasketches/src/lib.rs`, add the `tuple` bullet to the feature list and update the parenthetical, so the doc comment reads:

```rust
//! - `hll` (feature `hll`) — HyperLogLog cardinality estimation (sketch +
//!   union).
//! - `theta` (feature `theta`) — cardinality estimation plus set
//!   operations: union, intersection, a-not-b, and Jaccard similarity.
//! - `cpc` (feature `cpc`) — Compressed Probabilistic Counting
//!   cardinality estimation with a more compact serialized form (sketch +
//!   union only; no set operations beyond union).
//! - `tuple` (feature `tuple`) — ArrayOfDoubles Tuple sketches: cardinality
//!   estimation where each distinct key also carries a fixed-width array of
//!   `f64` values (summed on collision), plus the same set operations and
//!   Jaccard similarity as `theta`.
//!
//! (Module-level docs for each feature are only linked above when built
//! with that feature enabled — see `hll`/`theta`/`cpc`/`tuple` in the
//! sidebar, or build with `--all-features` to see all four at once.)
```

- [ ] **Step 4: Update `AGENTS.md`**

Four edits, each replacing the existing text exactly.

Line 13 — replace:

```
Both crates have `default = []`; every sketch family is opt-in via Cargo features: `hll`, `theta`, `cpc`.
```

with:

```
Both crates have `default = []`; every sketch family is opt-in via Cargo features: `hll`, `theta`, `cpc`, `tuple`.
```

In the test-commands block (around lines 34-37) — replace:

```
cargo test -p apache-datasketches --features cpc
cargo test --workspace --features hll,theta,cpc
```

with:

```
cargo test -p apache-datasketches --features cpc
cargo test -p apache-datasketches --features tuple
cargo test --workspace --features hll,theta,cpc,tuple
```

In the example-commands block (around line 48) — replace:

```
cargo run -p apache-datasketches --example cpc --features cpc
```

with:

```
cargo run -p apache-datasketches --example cpc --features cpc
cargo run -p apache-datasketches --example tuple --features tuple
```

In the vendored-headers paragraph (around line 82) — replace:

```
compiled (`common/`, `hll/`, `theta/`, `cpc/` `include/` dirs, plus `LICENSE`/`NOTICE`). This copy is
```

with:

```
compiled (`common/`, `hll/`, `theta/`, `cpc/`, `tuple/` `include/` dirs, plus `LICENSE`/`NOTICE`). This copy is
```

- [ ] **Step 5: Verify the whole feature matrix**

Run each of these and expect success with no warnings:

```bash
cargo build --workspace
cargo build --workspace --features apache-datasketches/tuple
cargo build --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo doc --workspace --all-features --no-deps
cargo fmt --all -- --check
```

The `--all-features` build is the important one for this plan: it is the only configuration where both the theta and tuple bridges are compiled together, which is what the `TupleResizeFactor`/`TupleJaccardBoundsFfi` naming exists to keep working.

If `cargo fmt --all -- --check` reports diffs, run `cargo fmt --all` and re-run the check.

- [ ] **Step 6: Verify the crate still packages cleanly**

Run: `cargo package -p apache-datasketches-sys --allow-dirty --no-verify`
Expected: succeeds, and the printed file list includes `vendor/datasketches-cpp/tuple/include/array_of_doubles_sketch.hpp` and `cpp/tuple/array_of_doubles_sketch_shim.cc`. This confirms the vendored-header copy (Task 1) is inside the crate directory and will reach crates.io.

- [ ] **Step 7: Commit**

```bash
git add apache-datasketches/examples/tuple.rs apache-datasketches/Cargo.toml apache-datasketches/src/lib.rs AGENTS.md
git commit -m "docs(tuple): add runnable tuple example and document the tuple feature"
```

---

## Task Summary

| Task | Deliverable |
|------|-------------|
| 1 | `tuple` Cargo feature, vendored tuple headers, `build.rs` plumbing |
| 2 | `ArrayOfDoublesSketch` + `ArrayOfDoublesSketchBuilder` + `ResizeFactor` |
| 3 | `CompactArrayOfDoublesSketch`, `serialize`/`deserialize`, `ArrayOfDoublesSketch::compact` |
| 4 | `ArrayOfDoublesInput` dispatch + `ArrayOfDoublesUnion` + `ArrayOfDoublesUnionBuilder` |
| 5 | `ArrayOfDoublesIntersection` (sum policy) |
| 6 | `ArrayOfDoublesAnotB` |
| 7 | `array_of_doubles_jaccard_similarity` + `JaccardBounds` |
| 8 | Ported upstream test suite + validation + concurrency tests |
| 9 | `examples/tuple.rs`, crate docs, full-matrix verification |

