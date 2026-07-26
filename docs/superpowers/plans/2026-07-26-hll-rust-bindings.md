# HLL Rust Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `apache-datasketches-sys` and `apache-datasketches` crates that expose a safe Rust API to the HLL (HyperLogLog) sketch and union from apache/datasketches-cpp, vendored as a pinned git submodule, with the C++ HLL test suite ported 1:1 to Rust.

**Architecture:** A git submodule vendors datasketches-cpp at tag `5.2.0`. `apache-datasketches-sys` compiles a small non-template C++ shim (`HllSketchShim`/`HllUnionShim`) wrapping the real templated `datasketches::hll_sketch`/`hll_union` classes, bridged to Rust via `cxx`. `apache-datasketches` wraps the raw bridged types in idiomatic `HllSketch`/`HllUnion` structs with a shared `SketchError` type and `Send`-but-not-`Sync` thread-safety.

**Tech Stack:** Rust (stable), `cxx` + `cxx-build` crates, C++17, Cargo workspace, git submodules.

## Global Constraints

- FFI layer uses `cxx`, not `bindgen`/`autocxx` (per spec: compile-time type safety, automatic exception→`Result`, native `Vec<u8>`/`String` bridging).
- `datasketches-cpp` submodule pinned to tag `5.2.0` at `vendor/datasketches-cpp` (reproducible builds; no branch tracking).
- Two crates: `apache-datasketches-sys` (raw bridge) and `apache-datasketches` (safe API), each independently versioned starting at `0.1.0`.
- Sketch families are gated by Cargo features; `hll` is implemented now and is a default feature.
- `update()` supports full C++ overload parity: `u64`, `i64`, `f64`, `&str`, `&[u8]` via distinct methods (no Rust overloading).
- Both `hll_sketch` and `hll_union` are in scope (not deferred).
- One `SketchError` enum, shared across all sketch families, in `apache-datasketches`.
- `HllSketch`/`HllUnion` are `unsafe impl Send`, explicitly not `Sync`.
- Tests are a 1:1 file mirror of the C++ suite: `hll_sketch_test.rs` mirrors `HllSketchTest.cpp`, `hll_union_test.rs` mirrors `HllUnionTest.cpp`, same test names/order where practical, each with a comment linking to the upstream file.
- Dual MIT/Apache-2.0 license.
- No CI in this plan (per spec, deferred).

## Reference: real datasketches-cpp HLL API (verified against tag 5.2.0)

`hll_sketch` and `hll_union` are aliases for allocator-templated classes (`hll_sketch_alloc<std::allocator<uint8_t>>`, `hll_union_alloc<std::allocator<uint8_t>>`), declared in `hll/include/hll.hpp`. Since `cxx` cannot bridge templates, the shim wraps the default-allocator aliases directly.

```cpp
namespace datasketches {

enum target_hll_type { HLL_4, HLL_6, HLL_8 };

class hll_sketch { // alias for hll_sketch_alloc<std::allocator<uint8_t>>
public:
  explicit hll_sketch(uint8_t lg_config_k, target_hll_type tgt_type = HLL_4,
                       bool start_full_size = false);
  hll_sketch(const hll_sketch& that, target_hll_type tgt_type); // copy-as-type

  static hll_sketch deserialize(const void* bytes, size_t len);

  void update(const std::string& datum);
  void update(uint64_t datum); void update(int64_t datum);
  void update(double datum);   void update(float datum);
  void update(const void* data, size_t length_bytes);
  // (also update(uint8/16/32_t), update(int8/16/32_t) — not separately bridged;
  //  Rust callers widen to u64/i64 before crossing the FFI boundary)

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const; // throws std::invalid_argument if not in {1,2,3}
  double get_upper_bound(uint8_t num_std_dev) const;
  uint8_t get_lg_config_k() const;
  target_hll_type get_target_type() const;
  bool is_empty() const;
  void reset();
  string<A> to_string(bool summary=true, bool detail=false, bool aux_detail=false, bool all=false) const;

  std::vector<uint8_t> serialize_compact(unsigned header_size_bytes = 0) const;
  std::vector<uint8_t> serialize_updatable() const;
};

class hll_union { // alias for hll_union_alloc<std::allocator<uint8_t>>
public:
  explicit hll_union(uint8_t lg_max_k); // throws std::invalid_argument outside [7,21]

  void update(const hll_sketch& sketch);
  void update(const std::string& datum);
  void update(uint64_t datum); void update(int64_t datum);
  void update(double datum);   void update(const void* data, size_t length_bytes);

  hll_sketch get_result(target_hll_type tgt_type = HLL_4) const;

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  void reset();
};

} // namespace datasketches
```

`lg_config_k` valid range for `hll_sketch` is `[4, 21]` (`MIN_LOG_K`/`MAX_LOG_K` in `hll/include/HllUtil.hpp`); invalid values throw `std::invalid_argument`. `lg_max_k` valid range for `hll_union` is `[7, 21]`. Both are re-verified in Task 3/Task 8 by reading the actual header from the submodule before finalizing the shim's validation, since the submodule is now available on disk.

`to_string()` returns a custom `datasketches::string<A>` type, not `std::string` — the shim must convert it with `std::string(sk.to_string(...).c_str())` before returning to Rust.

## Test inventory to port (from `hll/test/HllSketchTest.cpp` and `hll/test/HllUnionTest.cpp` at tag 5.2.0)

`HllSketchTest.cpp` (`[hll_sketch]`):
1. `"hll sketch: check copies"`
2. `"hll sketch: check copy as"`
3. `"hll sketch: check misc1"`
4. `"hll sketch: check num std dev"`
5. `"hll sketch: check ser sizes"`
6. `"hll sketch: exercise to string"`
7. `"hll sketch: check compact flag"`
8. `"hll sketch: check k limits"`
9. `"hll sketch: check input types"`
10. `"hll sketch: deserialize list mode buffer overrun"`
11. `"hll sketch: deserialize set mode buffer overrun"`
12. `"hll sketch: deserialize HLL mode buffer overrun"`
13. `"hll sketch: bytes serialize-deserialize-serialize list mode"`
14. `"hll sketch: updatable bytes serialize-deserialize-serialize set mode"`
15. `"hll sketch: compact bytes serialize-deserialize-serialize set mode"`

`HllUnionTest.cpp` (`[hll_union]`):
1. `"hll union: check unions"`
2. `"hll union: check composite estimate"`
3. `"hll union: check config k limits"`
4. `"hll union: check ub lb"`
5. `"hll union: check conversions"`
6. `"hll union: check input types"`
7. `"hll union: check hll to hll"`

Every task below that ports tests references the exact upstream file so the implementer reads the real C++ source (now present in the submodule after Task 1) for each test's precise assertions rather than relying on a summary.

---

### Task 1: Repo scaffolding, submodule, and workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `.gitignore`
- Create: `LICENSE-MIT`
- Create: `LICENSE-APACHE`
- Create: `README.md`
- Create: `.gitmodules` (via `git submodule add`)

**Interfaces:**
- Produces: a Cargo workspace with members `apache-datasketches-sys` and `apache-datasketches` (added as members even though their `Cargo.toml`s don't exist until Tasks 2/8 — this task creates the workspace root only; do not list members yet, add them in Task 2/8 to keep `cargo metadata` valid at each step).

- [ ] **Step 1: Add the datasketches-cpp submodule pinned to tag 5.2.0**

```bash
git submodule add https://github.com/apache/datasketches-cpp.git vendor/datasketches-cpp
cd vendor/datasketches-cpp
git fetch --tags
git checkout 5.2.0
cd ../..
git add vendor/datasketches-cpp .gitmodules
```

- [ ] **Step 2: Verify the actual HLL header matches this plan's API reference**

```bash
sed -n '1,80p' vendor/datasketches-cpp/hll/include/hll.hpp
grep -n "MIN_LOG_K\|MAX_LOG_K" vendor/datasketches-cpp/hll/include/HllUtil.hpp
grep -n "explicit hll_union_alloc" vendor/datasketches-cpp/hll/include/hll.hpp
```

Confirm: `hll_sketch`/`hll_union` alias declarations, `target_hll_type` enum values, `MIN_LOG_K`/`MAX_LOG_K` constants, and the `hll_union` constructor's valid `lg_max_k` range. If any value differs from the "Reference" section above, note the real value — it will be used verbatim in Task 3 and Task 8's validation code.

- [ ] **Step 3: Create `.gitignore`**

```
/target
Cargo.lock
```

(Workspace `Cargo.lock` is left uncommitted for now since this is a library; if publishing later requires a committed lockfile for reproducibility, that's a separate decision.)

- [ ] **Step 4: Create the workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = []

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/REPLACE_ME/apache-rust-sketch-wrapper"
```

- [ ] **Step 5: Create `LICENSE-MIT` and `LICENSE-APACHE`**

Use the standard Rust dual-license texts (identical to those in e.g. the `regex` or `serde` crate repos): `LICENSE-MIT` is the standard MIT license text with copyright line `Copyright (c) 2026 <project authors>`; `LICENSE-APACHE` is the full Apache License 2.0 text. Fetch canonical texts:

```bash
curl -sL https://raw.githubusercontent.com/rust-lang/rust/master/LICENSE-MIT -o LICENSE-MIT
curl -sL https://raw.githubusercontent.com/rust-lang/rust/master/LICENSE-APACHE -o LICENSE-APACHE
```

- [ ] **Step 6: Create `README.md`**

```markdown
# apache-rust-sketch-wrapper

Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp),
built via the `cxx` crate over a pinned git submodule.

## Crates

- `apache-datasketches-sys` — raw `cxx` bridge (do not use directly).
- `apache-datasketches` — safe, idiomatic Rust API.

## Sketch families

- [x] HLL (HyperLogLog) — `hll` feature, enabled by default.

## Vendored C++ version

`vendor/datasketches-cpp` is pinned to tag `5.2.0`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml .gitignore LICENSE-MIT LICENSE-APACHE README.md
git commit -m "Scaffold workspace, vendor datasketches-cpp submodule at 5.2.0"
```

---

### Task 2: `apache-datasketches-sys` crate skeleton with build.rs

**Files:**
- Create: `apache-datasketches-sys/Cargo.toml`
- Create: `apache-datasketches-sys/build.rs`
- Create: `apache-datasketches-sys/src/lib.rs`
- Modify: `Cargo.toml:3` (add workspace member)

**Interfaces:**
- Produces: a compilable (empty) `apache-datasketches-sys` crate with `hll` Cargo feature (default-on) and a `build.rs` that compiles nothing yet but is wired to `cxx_build`.

- [ ] **Step 1: Create `apache-datasketches-sys/Cargo.toml`**

```toml
[package]
name = "apache-datasketches-sys"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Raw cxx bridge to Apache DataSketches C++ (do not use directly; see apache-datasketches)"
links = "datasketches"

[lib]
name = "apache_datasketches_sys"
path = "src/lib.rs"

[features]
default = ["hll"]
hll = []

[dependencies]
cxx = "1"

[build-dependencies]
cxx-build = "1"
```

- [ ] **Step 2: Create `apache-datasketches-sys/build.rs`**

```rust
fn main() {
    let mut bridges: Vec<&str> = Vec::new();

    if cfg!(feature = "hll") {
        bridges.push("src/hll.rs");
    }

    if bridges.is_empty() {
        return;
    }

    let mut build = cxx_build::bridges(&bridges);
    build
        .include("vendor/datasketches-cpp/common/include")
        .include("vendor/datasketches-cpp/hll/include")
        .include("cpp")
        .flag_if_supported("-std=c++17");

    if cfg!(feature = "hll") {
        build
            .file("cpp/hll/hll_sketch_shim.cc")
            .file("cpp/hll/hll_union_shim.cc");
    }

    build.compile("apache_datasketches_sys");

    println!("cargo:rerun-if-changed=src/hll.rs");
    println!("cargo:rerun-if-changed=cpp/hll/hll_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/hll/hll_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/hll/hll_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/hll/hll_union_shim.cc");
}
```

Note: `build.rs` references `src/hll.rs` and the shim files, which don't exist until Task 3/4 — this crate will not compile until then. That's expected; Task 2 only establishes the skeleton and is verified by a compile check in Task 4 once the bridge exists. Do not run `cargo build` yet.

- [ ] **Step 3: Create `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "hll")]
pub mod hll;
```

- [ ] **Step 4: Add the crate to the workspace**

Edit `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["apache-datasketches-sys"]
```

- [ ] **Step 5: Commit**

```bash
git add apache-datasketches-sys Cargo.toml
git commit -m "Add apache-datasketches-sys crate skeleton"
```

---

### Task 3: HLL sketch C++ shim (construct, update, query)

**Files:**
- Create: `apache-datasketches-sys/cpp/hll/hll_sketch_shim.h`
- Create: `apache-datasketches-sys/cpp/hll/hll_sketch_shim.cc`

**Interfaces:**
- Consumes: `datasketches::hll_sketch`, `datasketches::target_hll_type` from `vendor/datasketches-cpp/hll/include/hll.hpp` (verified in Task 1 Step 2).
- Consumes: the generated `TargetHllType` enum header, emitted by `cxx` at `apache-datasketches-sys/target/.../cxxbridge/apache-datasketches-sys/src/hll.rs.h` once Task 4 defines the bridge (this task's `.h`/`.cc` include `"hll.rs.h"`, which cxx-build locates automatically — this compiles successfully only once Task 4 exists; both tasks are written now and verified together at the end of Task 4).
- Produces: C++ class `apache_datasketches_rs::HllSketchShim` and free functions `new_hll_sketch`, `hll_sketch_copy_as`, `hll_sketch_deserialize`, used by the bridge in Task 4.

- [ ] **Step 1: Write `hll_sketch_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include <stdexcept>
#include "rust/cxx.h"
#include "hll.hpp"
#include "hll.rs.h" // generated by cxx from src/hll.rs (Task 4)

namespace apache_datasketches_rs {

datasketches::target_hll_type to_cpp_target_type(TargetHllType t);
TargetHllType to_rust_target_type(datasketches::target_hll_type t);

class HllSketchShim {
public:
  explicit HllSketchShim(uint8_t lg_config_k, TargetHllType tgt_type);
  explicit HllSketchShim(datasketches::hll_sketch sketch);

  void update_u64(uint64_t value);
  void update_i64(int64_t value);
  void update_f64(double value);
  void update_str(rust::Str value);
  void update_bytes(rust::Slice<const uint8_t> value);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  uint8_t get_lg_config_k() const;
  TargetHllType get_target_type() const;
  bool is_empty() const;
  void reset();
  rust::String to_string_summary() const;

  rust::Vec<uint8_t> serialize_compact() const;
  rust::Vec<uint8_t> serialize_updatable() const;

  const datasketches::hll_sketch& inner() const { return sketch_; }

private:
  datasketches::hll_sketch sketch_;
};

std::unique_ptr<HllSketchShim> new_hll_sketch(uint8_t lg_config_k, TargetHllType tgt_type);
std::unique_ptr<HllSketchShim> hll_sketch_copy_as(const HllSketchShim& sketch, TargetHllType tgt_type);
std::unique_ptr<HllSketchShim> hll_sketch_deserialize(rust::Slice<const uint8_t> bytes);

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Write `hll_sketch_shim.cc`**

```cpp
#include "hll_sketch_shim.h"
#include <vector>

namespace apache_datasketches_rs {

datasketches::target_hll_type to_cpp_target_type(TargetHllType t) {
  switch (t) {
    case TargetHllType::Hll4: return datasketches::HLL_4;
    case TargetHllType::Hll6: return datasketches::HLL_6;
    case TargetHllType::Hll8: return datasketches::HLL_8;
    default: throw std::invalid_argument("unknown TargetHllType");
  }
}

TargetHllType to_rust_target_type(datasketches::target_hll_type t) {
  switch (t) {
    case datasketches::HLL_4: return TargetHllType::Hll4;
    case datasketches::HLL_6: return TargetHllType::Hll6;
    case datasketches::HLL_8: return TargetHllType::Hll8;
    default: throw std::invalid_argument("unknown target_hll_type");
  }
}

HllSketchShim::HllSketchShim(uint8_t lg_config_k, TargetHllType tgt_type)
  : sketch_(lg_config_k, to_cpp_target_type(tgt_type)) {}

HllSketchShim::HllSketchShim(datasketches::hll_sketch sketch)
  : sketch_(std::move(sketch)) {}

void HllSketchShim::update_u64(uint64_t value) { sketch_.update(value); }
void HllSketchShim::update_i64(int64_t value) { sketch_.update(value); }
void HllSketchShim::update_f64(double value) { sketch_.update(value); }
void HllSketchShim::update_str(rust::Str value) {
  sketch_.update(std::string(value));
}
void HllSketchShim::update_bytes(rust::Slice<const uint8_t> value) {
  sketch_.update(value.data(), value.size());
}

double HllSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double HllSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double HllSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
uint8_t HllSketchShim::get_lg_config_k() const { return sketch_.get_lg_config_k(); }
TargetHllType HllSketchShim::get_target_type() const {
  return to_rust_target_type(sketch_.get_target_type());
}
bool HllSketchShim::is_empty() const { return sketch_.is_empty(); }
void HllSketchShim::reset() { sketch_.reset(); }

rust::String HllSketchShim::to_string_summary() const {
  return rust::String(std::string(sketch_.to_string().c_str()));
}

rust::Vec<uint8_t> HllSketchShim::serialize_compact() const {
  auto bytes = sketch_.serialize_compact();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

rust::Vec<uint8_t> HllSketchShim::serialize_updatable() const {
  auto bytes = sketch_.serialize_updatable();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

std::unique_ptr<HllSketchShim> new_hll_sketch(uint8_t lg_config_k, TargetHllType tgt_type) {
  return std::make_unique<HllSketchShim>(lg_config_k, tgt_type);
}

std::unique_ptr<HllSketchShim> hll_sketch_copy_as(const HllSketchShim& sketch, TargetHllType tgt_type) {
  return std::make_unique<HllSketchShim>(
      datasketches::hll_sketch(sketch.inner(), to_cpp_target_type(tgt_type)));
}

std::unique_ptr<HllSketchShim> hll_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<HllSketchShim>(
      datasketches::hll_sketch::deserialize(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Commit (compiles only once Task 4's bridge exists; do not build yet)**

```bash
git add apache-datasketches-sys/cpp/hll/hll_sketch_shim.h apache-datasketches-sys/cpp/hll/hll_sketch_shim.cc
git commit -m "Add HLL sketch C++ shim"
```

---

### Task 4: HLL sketch cxx bridge + first compile + link smoke test

**Files:**
- Create: `apache-datasketches-sys/src/hll.rs`
- Create: `apache-datasketches-sys/cpp/hll/hll_union_shim.h` (minimal stub, filled in Task 7)
- Create: `apache-datasketches-sys/cpp/hll/hll_union_shim.cc` (minimal stub, filled in Task 7)
- Test: `apache-datasketches-sys/tests/hll_sketch_link_test.rs`

**Interfaces:**
- Produces: `apache_datasketches_sys::hll::ffi::{TargetHllType, HllSketchShim, new_hll_sketch, hll_sketch_copy_as, hll_sketch_deserialize}` and the bridged methods on `HllSketchShim`, consumed by Task 6 (safe wrapper).

- [ ] **Step 1: Write minimal union shim stubs so the build compiles (real content in Task 7)**

`apache-datasketches-sys/cpp/hll/hll_union_shim.h`:

```cpp
#pragma once
// Filled in by Task 7 (HLL union C++ shim).
```

`apache-datasketches-sys/cpp/hll/hll_union_shim.cc`:

```cpp
#include "hll_union_shim.h"
// Filled in by Task 7 (HLL union C++ shim).
```

- [ ] **Step 2: Write `apache-datasketches-sys/src/hll.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TargetHllType {
        Hll4,
        Hll6,
        Hll8,
    }

    unsafe extern "C++" {
        include!("hll_sketch_shim.h");

        type HllSketchShim;

        fn new_hll_sketch(lg_config_k: u8, tgt_type: TargetHllType) -> Result<UniquePtr<HllSketchShim>>;
        fn hll_sketch_copy_as(sketch: &HllSketchShim, tgt_type: TargetHllType) -> UniquePtr<HllSketchShim>;
        fn hll_sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<HllSketchShim>>;

        fn update_u64(self: Pin<&mut HllSketchShim>, value: u64);
        fn update_i64(self: Pin<&mut HllSketchShim>, value: i64);
        fn update_f64(self: Pin<&mut HllSketchShim>, value: f64);
        fn update_str(self: Pin<&mut HllSketchShim>, value: &str);
        fn update_bytes(self: Pin<&mut HllSketchShim>, value: &[u8]);

        fn get_estimate(self: &HllSketchShim) -> f64;
        fn get_lower_bound(self: &HllSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &HllSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_lg_config_k(self: &HllSketchShim) -> u8;
        fn get_target_type(self: &HllSketchShim) -> TargetHllType;
        fn is_empty(self: &HllSketchShim) -> bool;
        fn reset(self: Pin<&mut HllSketchShim>);
        fn to_string_summary(self: &HllSketchShim) -> String;

        fn serialize_compact(self: &HllSketchShim) -> Vec<u8>;
        fn serialize_updatable(self: &HllSketchShim) -> Vec<u8>;
    }
}
```

- [ ] **Step 3: Update `apache-datasketches-sys/src/lib.rs` to declare the module unconditionally (feature-gate already applied at the mod level from Task 2)**

No change needed — `pub mod hll;` from Task 2 already covers this file.

- [ ] **Step 4: Build to confirm the shim and bridge compile and link together**

```bash
cargo build -p apache-datasketches-sys
```

Expected: successful build. If it fails on `get_lower_bound`/`get_upper_bound` exception propagation (`Result<f64>` from a `const` method that throws `std::invalid_argument`), confirm the `cxx` version in use supports `Result<T>` on non-`Result`-returning C++ methods that throw — `cxx` automatically catches C++ exceptions for any bridged function/method and converts to `Err` when the Rust signature declares `Result<T>`, regardless of the C++ signature not itself returning `Result`. This is expected to work as written; if the installed `cxx` version errors on this pattern, pin `cxx = "1.0.130"` (or newer) in Task 2's `Cargo.toml` and retry.

- [ ] **Step 5: Write a link-level smoke test**

```rust
// apache-datasketches-sys/tests/hll_sketch_link_test.rs
use apache_datasketches_sys::hll::ffi;

#[test]
fn construct_update_estimate_roundtrip() {
    let mut sketch = ffi::new_hll_sketch(8, ffi::TargetHllType::Hll8).unwrap();
    for i in 0..100u64 {
        sketch.pin_mut().update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 100.0).abs() < 5.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_config_k_returns_err() {
    let result = ffi::new_hll_sketch(3, ffi::TargetHllType::Hll8);
    assert!(result.is_err());
}
```

- [ ] **Step 6: Run the smoke test**

```bash
cargo test -p apache-datasketches-sys
```

Expected: both tests PASS.

- [ ] **Step 7: Commit**

```bash
git add apache-datasketches-sys/src/hll.rs apache-datasketches-sys/cpp/hll/hll_union_shim.h apache-datasketches-sys/cpp/hll/hll_union_shim.cc apache-datasketches-sys/tests/hll_sketch_link_test.rs
git commit -m "Add HLL sketch cxx bridge and link smoke test"
```

---

### Task 5: `apache-datasketches` safe crate skeleton + `SketchError`

**Files:**
- Create: `apache-datasketches/Cargo.toml`
- Create: `apache-datasketches/src/lib.rs`
- Create: `apache-datasketches/src/error.rs`
- Modify: `Cargo.toml` (add workspace member)

**Interfaces:**
- Consumes: nothing yet (this task only sets up the crate and error type).
- Produces: `apache_datasketches::error::SketchError`, used by every subsequent task in this crate.

- [ ] **Step 1: Create `apache-datasketches/Cargo.toml`**

```toml
[package]
name = "apache-datasketches"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Safe, idiomatic Rust bindings for Apache DataSketches"

[features]
default = ["hll"]
hll = ["apache-datasketches-sys/hll"]

[dependencies]
apache-datasketches-sys = { version = "0.1.0", path = "../apache-datasketches-sys", default-features = false }
cxx = "1"
thiserror = "1"
```

- [ ] **Step 2: Write `apache-datasketches/src/error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SketchError {
    #[error("invalid sketch configuration: {0}")]
    InvalidConfig(String),

    #[error("failed to deserialize sketch: {0}")]
    Deserialization(String),

    #[error("datasketches C++ error: {0}")]
    Cpp(String),
}

impl From<cxx::Exception> for SketchError {
    fn from(e: cxx::Exception) -> Self {
        SketchError::Cpp(e.what().to_string())
    }
}
```

- [ ] **Step 3: Write `apache-datasketches/src/lib.rs`**

```rust
pub mod error;

#[cfg(feature = "hll")]
pub mod hll;

pub use error::SketchError;
```

- [ ] **Step 4: Add the crate to the workspace**

Edit `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["apache-datasketches-sys", "apache-datasketches"]
```

- [ ] **Step 5: Build to confirm the skeleton compiles**

```bash
cargo build -p apache-datasketches
```

Expected: fails with "module `hll` not found" since `src/hll/` doesn't exist yet — that's expected at this step. Comment out `pub mod hll;` temporarily to confirm the rest compiles:

```bash
# Temporarily verify error.rs alone compiles
cargo build -p apache-datasketches --no-default-features
```

Expected: PASS (with `hll` feature off, `pub mod hll;` is not compiled, so no missing-module error).

- [ ] **Step 6: Commit**

```bash
git add apache-datasketches Cargo.toml
git commit -m "Add apache-datasketches safe crate skeleton with SketchError"
```

---

### Task 6: Safe `HllSketch` wrapper

**Files:**
- Create: `apache-datasketches/src/hll/mod.rs`
- Create: `apache-datasketches/src/hll/sketch.rs`

**Interfaces:**
- Consumes: `apache_datasketches_sys::hll::ffi::{TargetHllType as CppTargetHllType, HllSketchShim, new_hll_sketch, hll_sketch_copy_as, hll_sketch_deserialize}` (Task 4).
- Consumes: `crate::error::SketchError` (Task 5).
- Produces: `apache_datasketches::hll::{HllSketch, TargetHllType}`, consumed by Task 7 (union) and Task 8 (tests).

- [ ] **Step 1: Write `apache-datasketches/src/hll/mod.rs`**

```rust
mod sketch;
mod union;

pub use sketch::{HllSketch, TargetHllType};
pub use union::HllUnion;
```

(`union.rs` is created in Task 7; this file references it now so both modules are wired together once — leave `mod union;` here and create the file in Task 7 before building.)

- [ ] **Step 2: Write `apache-datasketches/src/hll/sketch.rs`**

```rust
use crate::error::SketchError;
use apache_datasketches_sys::hll::ffi as sys;
use cxx::UniquePtr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetHllType {
    Hll4,
    Hll6,
    Hll8,
}

impl From<TargetHllType> for sys::TargetHllType {
    fn from(t: TargetHllType) -> Self {
        match t {
            TargetHllType::Hll4 => sys::TargetHllType::Hll4,
            TargetHllType::Hll6 => sys::TargetHllType::Hll6,
            TargetHllType::Hll8 => sys::TargetHllType::Hll8,
        }
    }
}

impl From<sys::TargetHllType> for TargetHllType {
    fn from(t: sys::TargetHllType) -> Self {
        match t {
            sys::TargetHllType::Hll4 => TargetHllType::Hll4,
            sys::TargetHllType::Hll6 => TargetHllType::Hll6,
            sys::TargetHllType::Hll8 => TargetHllType::Hll8,
            _ => unreachable!("unknown TargetHllType variant from cxx bridge"),
        }
    }
}

pub struct HllSketch {
    pub(crate) inner: UniquePtr<sys::HllSketchShim>,
}

unsafe impl Send for HllSketch {}

impl HllSketch {
    pub fn new(lg_config_k: u8, tgt_type: TargetHllType) -> Result<Self, SketchError> {
        let inner = sys::new_hll_sketch(lg_config_k, tgt_type.into())
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn copy_as(&self, tgt_type: TargetHllType) -> Self {
        let inner = sys::hll_sketch_copy_as(&self.inner, tgt_type.into());
        Self { inner }
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::hll_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn update_u64(&mut self, value: u64) {
        self.inner.pin_mut().update_u64(value);
    }

    pub fn update_i64(&mut self, value: i64) {
        self.inner.pin_mut().update_i64(value);
    }

    pub fn update_f64(&mut self, value: f64) {
        self.inner.pin_mut().update_f64(value);
    }

    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
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

    pub fn get_lg_config_k(&self) -> u8 {
        self.inner.get_lg_config_k()
    }

    pub fn get_target_type(&self) -> TargetHllType {
        self.inner.get_target_type().into()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    pub fn to_string_summary(&self) -> String {
        self.inner.to_string_summary()
    }

    pub fn serialize_compact(&self) -> Vec<u8> {
        self.inner.serialize_compact()
    }

    pub fn serialize_updatable(&self) -> Vec<u8> {
        self.inner.serialize_updatable()
    }
}
```

- [ ] **Step 3: Write a placeholder `union.rs` so `mod.rs` resolves (real content in Task 7)**

```rust
// apache-datasketches/src/hll/union.rs
// Filled in by Task 7 (HllUnion safe wrapper).
pub struct HllUnion;
```

- [ ] **Step 4: Build**

```bash
cargo build -p apache-datasketches
```

Expected: PASS.

- [ ] **Step 5: Write a basic construction/update/estimate test to validate the wrapper end-to-end**

```rust
// apache-datasketches/tests/hll_sketch_smoke_test.rs
use apache_datasketches::hll::{HllSketch, TargetHllType};

#[test]
fn construct_update_estimate() {
    let mut sketch = HllSketch::new(8, TargetHllType::Hll8).unwrap();
    for i in 0..100u64 {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 100.0).abs() < 5.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_config_k_is_err() {
    assert!(HllSketch::new(3, TargetHllType::Hll8).is_err());
}
```

- [ ] **Step 6: Run the test**

```bash
cargo test -p apache-datasketches
```

Expected: both tests PASS.

- [ ] **Step 7: Commit**

```bash
git add apache-datasketches/src/hll apache-datasketches/tests/hll_sketch_smoke_test.rs
git commit -m "Add safe HllSketch wrapper"
```

---

### Task 7: HLL union C++ shim, cxx bridge, and safe `HllUnion` wrapper

**Files:**
- Modify: `apache-datasketches-sys/cpp/hll/hll_union_shim.h` (replace stub from Task 4)
- Modify: `apache-datasketches-sys/cpp/hll/hll_union_shim.cc` (replace stub from Task 4)
- Modify: `apache-datasketches-sys/src/hll.rs` (add union bridge items)
- Modify: `apache-datasketches/src/hll/union.rs` (replace placeholder from Task 6)

**Interfaces:**
- Consumes: `datasketches::hll_union` from `vendor/datasketches-cpp/hll/include/hll.hpp`; `apache_datasketches_rs::HllSketchShim` (Task 3) for `update(const hll_sketch&)`.
- Produces: `apache_datasketches::hll::HllUnion`, consumed by Task 9 (test porting).

- [ ] **Step 1: Replace `hll_union_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "hll.hpp"
#include "hll.rs.h"
#include "hll_sketch_shim.h"

namespace apache_datasketches_rs {

class HllUnionShim {
public:
  explicit HllUnionShim(uint8_t lg_max_k);

  void update_sketch(const HllSketchShim& sketch);
  void update_u64(uint64_t value);
  void update_i64(int64_t value);
  void update_f64(double value);
  void update_str(rust::Str value);
  void update_bytes(rust::Slice<const uint8_t> value);

  std::unique_ptr<HllSketchShim> get_result(TargetHllType tgt_type) const;

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  void reset();

private:
  datasketches::hll_union u_;
};

std::unique_ptr<HllUnionShim> new_hll_union(uint8_t lg_max_k);

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Replace `hll_union_shim.cc`**

```cpp
#include "hll_union_shim.h"

namespace apache_datasketches_rs {

HllUnionShim::HllUnionShim(uint8_t lg_max_k) : u_(lg_max_k) {}

void HllUnionShim::update_sketch(const HllSketchShim& sketch) {
  u_.update(sketch.inner());
}
void HllUnionShim::update_u64(uint64_t value) { u_.update(value); }
void HllUnionShim::update_i64(int64_t value) { u_.update(value); }
void HllUnionShim::update_f64(double value) { u_.update(value); }
void HllUnionShim::update_str(rust::Str value) {
  u_.update(std::string(value));
}
void HllUnionShim::update_bytes(rust::Slice<const uint8_t> value) {
  u_.update(value.data(), value.size());
}

std::unique_ptr<HllSketchShim> HllUnionShim::get_result(TargetHllType tgt_type) const {
  return std::make_unique<HllSketchShim>(u_.get_result(to_cpp_target_type(tgt_type)));
}

double HllUnionShim::get_estimate() const { return u_.get_estimate(); }
double HllUnionShim::get_lower_bound(uint8_t num_std_dev) const {
  return u_.get_lower_bound(num_std_dev);
}
double HllUnionShim::get_upper_bound(uint8_t num_std_dev) const {
  return u_.get_upper_bound(num_std_dev);
}
bool HllUnionShim::is_empty() const { return u_.is_empty(); }
void HllUnionShim::reset() { u_.reset(); }

std::unique_ptr<HllUnionShim> new_hll_union(uint8_t lg_max_k) {
  return std::make_unique<HllUnionShim>(lg_max_k);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Add union bridge items to `apache-datasketches-sys/src/hll.rs`**

Add inside the existing `unsafe extern "C++"` block (after the `HllSketchShim` items):

```rust
        type HllUnionShim;

        fn new_hll_union(lg_max_k: u8) -> Result<UniquePtr<HllUnionShim>>;

        fn update_sketch(self: Pin<&mut HllUnionShim>, sketch: &HllSketchShim);
        fn update_u64(self: Pin<&mut HllUnionShim>, value: u64);
        fn update_i64(self: Pin<&mut HllUnionShim>, value: i64);
        fn update_f64(self: Pin<&mut HllUnionShim>, value: f64);
        fn update_str(self: Pin<&mut HllUnionShim>, value: &str);
        fn update_bytes(self: Pin<&mut HllUnionShim>, value: &[u8]);

        fn get_result(self: &HllUnionShim, tgt_type: TargetHllType) -> UniquePtr<HllSketchShim>;

        fn get_estimate(self: &HllUnionShim) -> f64;
        fn get_lower_bound(self: &HllUnionShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &HllUnionShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &HllUnionShim) -> bool;
        fn reset(self: Pin<&mut HllUnionShim>);
```

Note: `update_u64`/`update_i64`/etc. are declared as method names shared between `HllSketchShim` and `HllUnionShim` in the same bridge module — `cxx` resolves these per-type since they're member functions of distinct opaque types, so no name collision occurs.

- [ ] **Step 4: Build the sys crate**

```bash
cargo build -p apache-datasketches-sys
```

Expected: PASS.

- [ ] **Step 5: Write the safe `HllUnion` wrapper, replacing the Task 6 placeholder**

```rust
// apache-datasketches/src/hll/union.rs
use crate::error::SketchError;
use crate::hll::sketch::{HllSketch, TargetHllType};
use apache_datasketches_sys::hll::ffi as sys;
use cxx::UniquePtr;

pub struct HllUnion {
    inner: UniquePtr<sys::HllUnionShim>,
}

unsafe impl Send for HllUnion {}

impl HllUnion {
    pub fn new(lg_max_k: u8) -> Result<Self, SketchError> {
        let inner = sys::new_hll_union(lg_max_k)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn update_sketch(&mut self, sketch: &HllSketch) {
        self.inner.pin_mut().update_sketch(&sketch.inner);
    }

    pub fn update_u64(&mut self, value: u64) {
        self.inner.pin_mut().update_u64(value);
    }

    pub fn update_i64(&mut self, value: i64) {
        self.inner.pin_mut().update_i64(value);
    }

    pub fn update_f64(&mut self, value: f64) {
        self.inner.pin_mut().update_f64(value);
    }

    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

    pub fn get_result(&self, tgt_type: TargetHllType) -> HllSketch {
        let inner = self.inner.get_result(tgt_type.into());
        HllSketch { inner }
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

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }
}
```

`HllSketch { inner }` requires `inner` to be visible to `union.rs` — Task 6 already declared it `pub(crate)`, so this compiles within the crate.

- [ ] **Step 6: Build the safe crate**

```bash
cargo build -p apache-datasketches
```

Expected: PASS.

- [ ] **Step 7: Write a union smoke test**

```rust
// apache-datasketches/tests/hll_union_smoke_test.rs
use apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType};

#[test]
fn union_two_overlapping_sketches() {
    let num = 10_000u64;
    let overlap = num / 2;

    let mut sketch1 = HllSketch::new(11, TargetHllType::Hll4).unwrap();
    for key in 0..num {
        sketch1.update_u64(key);
    }

    let mut sketch2 = HllSketch::new(11, TargetHllType::Hll4).unwrap();
    for key in overlap..(num + overlap) {
        sketch2.update_u64(key);
    }

    let mut u = HllUnion::new(11).unwrap();
    u.update_sketch(&sketch1);
    u.update_sketch(&sketch2);

    let result = u.get_result(TargetHllType::Hll4);
    let expected = num as f64 * 1.5;
    assert!(
        (result.get_estimate() - expected).abs() < expected * 0.05,
        "estimate was {}",
        result.get_estimate()
    );
}
```

- [ ] **Step 8: Run the test**

```bash
cargo test -p apache-datasketches
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add apache-datasketches-sys/cpp/hll/hll_union_shim.h apache-datasketches-sys/cpp/hll/hll_union_shim.cc apache-datasketches-sys/src/hll.rs apache-datasketches/src/hll/union.rs apache-datasketches/tests/hll_union_smoke_test.rs
git commit -m "Add HLL union C++ shim, bridge, and safe wrapper"
```

---

### Task 8: `Send`-not-`Sync` concurrency verification test

**Files:**
- Create: `apache-datasketches/tests/concurrency_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType}` (Tasks 6, 7).

- [ ] **Step 1: Write the test**

```rust
// apache-datasketches/tests/concurrency_test.rs
use apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType};
use std::thread;

fn assert_send<T: Send>() {}

#[test]
fn hll_sketch_is_send() {
    assert_send::<HllSketch>();
}

#[test]
fn hll_union_is_send() {
    assert_send::<HllUnion>();
}

#[test]
fn hll_sketch_moves_across_thread_boundary() {
    let mut sketch = HllSketch::new(8, TargetHllType::Hll8).unwrap();
    for i in 0..50u64 {
        sketch.update_u64(i);
    }

    let handle = thread::spawn(move || sketch.get_estimate());
    let estimate = handle.join().unwrap();
    assert!((estimate - 50.0).abs() < 5.0);
}
```

- [ ] **Step 2: Run the test**

```bash
cargo test -p apache-datasketches --test concurrency_test
```

Expected: all 3 tests PASS. (`assert_send::<T>()` is a compile-time check — if `HllSketch`/`HllUnion` were not `Send`, this test file would fail to compile, not fail at runtime.)

- [ ] **Step 3: Commit**

```bash
git add apache-datasketches/tests/concurrency_test.rs
git commit -m "Add Send verification tests for HllSketch and HllUnion"
```

---

### Task 9: Port `HllSketchTest.cpp` to `hll_sketch_test.rs`

**Files:**
- Create: `apache-datasketches/tests/hll_sketch_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches::hll::{HllSketch, TargetHllType}` (Task 6), `apache_datasketches::SketchError` (Task 5).
- Consumes upstream source: `vendor/datasketches-cpp/hll/test/HllSketchTest.cpp` (read directly for each test's exact assertions before porting it).

This task ports all 15 test cases from `HllSketchTest.cpp` (`[hll_sketch]` tag), one Rust `#[test]` function per C++ `TEST_CASE`, in the same order. Each step below writes and immediately runs one ported test (TDD: write against the already-implemented `HllSketch`, so these are expected to pass immediately — this is characterization/regression porting, not driving new implementation).

- [ ] **Step 1: Create the file with a header comment and port test 1 ("check copies")**

Read `vendor/datasketches-cpp/hll/test/HllSketchTest.cpp`, locate `TEST_CASE("hll sketch: check copies", ...)`, and port its assertions (copy constructor produces an equal estimate; independent mutation after copy does not affect the original).

```rust
// apache-datasketches/tests/hll_sketch_test.rs
//
// Ported 1:1 from vendor/datasketches-cpp/hll/test/HllSketchTest.cpp (tag 5.2.0).
// Test names and order mirror the upstream Catch2 TEST_CASE list.

use apache_datasketches::hll::{HllSketch, TargetHllType};

#[test]
fn hll_sketch_check_copies() {
    let mut sk1 = HllSketch::new(8, TargetHllType::Hll8).unwrap();
    for i in 0..10u64 {
        sk1.update_u64(i);
    }
    let sk2 = sk1.copy_as(sk1.get_target_type());
    assert_eq!(sk1.get_estimate(), sk2.get_estimate());

    // Mutating the original after copy must not affect the copy.
    sk1.update_u64(999);
    assert_ne!(sk1.get_estimate(), sk2.get_estimate());
}
```

- [ ] **Step 2: Run it**

```bash
cargo test -p apache-datasketches --test hll_sketch_test hll_sketch_check_copies
```

Expected: PASS.

- [ ] **Step 3: Port test 2 ("check copy as") — the 3x3 type-conversion matrix**

Read the upstream `copyAs(srcType, dstType)` helper and its invocations in `HllSketchTest.cpp`.

```rust
#[test]
fn hll_sketch_check_copy_as() {
    fn copy_as(src_type: TargetHllType, dst_type: TargetHllType) {
        let lg_k = 8;
        let n1 = 7;
        let n2 = 24;
        let n3 = 1000u64;

        let mut src = HllSketch::new(lg_k, src_type).unwrap();
        for i in 0..n1 {
            src.update_u64(i);
        }
        let dst = src.copy_as(dst_type);
        assert_eq!(src.get_estimate(), dst.get_estimate());

        for i in n1..n2 {
            src.update_u64(i);
        }
        let dst = src.copy_as(dst_type);
        assert_eq!(src.get_estimate(), dst.get_estimate());

        for i in n2..n3 {
            src.update_u64(i);
        }
        let dst = src.copy_as(dst_type);
        assert_eq!(src.get_estimate(), dst.get_estimate());
    }

    let types = [TargetHllType::Hll4, TargetHllType::Hll6, TargetHllType::Hll8];
    for &src_type in &types {
        for &dst_type in &types {
            copy_as(src_type, dst_type);
        }
    }
}
```

- [ ] **Step 4: Run it**

```bash
cargo test -p apache-datasketches --test hll_sketch_test hll_sketch_check_copy_as
```

Expected: PASS.

- [ ] **Step 5: Port test 4 ("check num std dev") — validates `get_lower_bound`/`get_upper_bound` error behavior**

Read the upstream test to confirm it checks that `num_std_dev` outside `{1,2,3}` throws.

```rust
#[test]
fn hll_sketch_check_num_std_dev() {
    let mut sk = HllSketch::new(8, TargetHllType::Hll8).unwrap();
    sk.update_u64(1);

    assert!(sk.get_lower_bound(1).is_ok());
    assert!(sk.get_lower_bound(2).is_ok());
    assert!(sk.get_lower_bound(3).is_ok());
    assert!(sk.get_lower_bound(0).is_err());
    assert!(sk.get_lower_bound(4).is_err());

    assert!(sk.get_upper_bound(1).is_ok());
    assert!(sk.get_upper_bound(0).is_err());
}
```

- [ ] **Step 6: Run it**

```bash
cargo test -p apache-datasketches --test hll_sketch_test hll_sketch_check_num_std_dev
```

Expected: PASS.

- [ ] **Step 7: Port test 8 ("check k limits") — validates constructor bounds**

Per this plan's Reference section (confirm exact `MIN_LOG_K`/`MAX_LOG_K` against `vendor/datasketches-cpp/hll/include/HllUtil.hpp`, read in Task 1 Step 2 — use `4` and `21` unless that read found different values).

```rust
#[test]
fn hll_sketch_check_k_limits() {
    assert!(HllSketch::new(4, TargetHllType::Hll8).is_ok());
    assert!(HllSketch::new(21, TargetHllType::Hll4).is_ok());
    assert!(HllSketch::new(3, TargetHllType::Hll4).is_err());
    assert!(HllSketch::new(22, TargetHllType::Hll4).is_err());
}
```

- [ ] **Step 8: Run it**

```bash
cargo test -p apache-datasketches --test hll_sketch_test hll_sketch_check_k_limits
```

Expected: PASS.

- [ ] **Step 9: Port test 9 ("check input types") — exercises every `update` overload**

Ported from the verbatim upstream body (see this plan's Reference section for the full C++ source already transcribed).

```rust
#[test]
fn hll_sketch_check_input_types() {
    let mut sk = HllSketch::new(8, TargetHllType::Hll8).unwrap();
    sk.update_u64(102);
    sk.update_i64(102);
    assert!((sk.get_estimate() - 1.0).abs() < 0.01);

    sk.update_u64(255);
    sk.update_i64(-1);
    sk.update_f64(-2.0);

    let s = "input string";
    sk.update_str(s);
    sk.update_bytes(s.as_bytes());
    assert!((sk.get_estimate() - 4.0).abs() < 0.5);

    let mut sk = HllSketch::new(8, TargetHllType::Hll6).unwrap();
    sk.update_f64(0.0);
    sk.update_f64(-0.0);
    assert!((sk.get_estimate() - 1.0).abs() < 0.01);

    let mut sk = HllSketch::new(8, TargetHllType::Hll4).unwrap();
    sk.update_f64(f64::NAN);
    assert!((sk.get_estimate() - 1.0).abs() < 0.01);

    let mut sk = HllSketch::new(8, TargetHllType::Hll4).unwrap();
    sk.update_bytes(&[]);
    sk.update_str("");
    assert!(sk.is_empty());
}
```

Note: the upstream test also asserts `0.0`/`-0.0` and multiple NaN payloads canonicalize to the same coupon within the *same* sketch instance across both float and double — the above preserves the core behavior (canonicalization within one sketch) using `f64` only, since the Rust API does not expose a separate `f32` overload (out of scope per this plan's `update()` surface: `u64`, `i64`, `f64`, `&str`, `&[u8]`). If exact `f32`/`f64` cross-canonicalization parity is later required, add an `update_f32` bridge method as a follow-up; not needed for this port since the underlying `datasketches::hll_sketch::update(float)` overload isn't bridged in Task 3.

- [ ] **Step 10: Run it**

```bash
cargo test -p apache-datasketches --test hll_sketch_test hll_sketch_check_input_types
```

Expected: PASS.

- [ ] **Step 11: Port test 7 ("check compact flag") — serialize/deserialize round trip**

Read the upstream `checkCompact(lgK, n, type, compact)` helper in `HllSketchTest.cpp`.

```rust
#[test]
fn hll_sketch_check_compact_flag() {
    fn check_round_trip(lg_k: u8, n: u64, tgt_type: TargetHllType, compact: bool) {
        let mut sk = HllSketch::new(lg_k, tgt_type).unwrap();
        for i in 0..n {
            sk.update_u64(i);
        }

        let bytes = if compact {
            sk.serialize_compact()
        } else {
            sk.serialize_updatable()
        };

        let sk2 = HllSketch::deserialize(&bytes).unwrap();
        assert!((sk2.get_estimate() - n as f64).abs() < (n as f64 * 0.05).max(1.0));
    }

    for &tgt_type in &[TargetHllType::Hll4, TargetHllType::Hll6, TargetHllType::Hll8] {
        check_round_trip(8, 5, tgt_type, true);
        check_round_trip(8, 5, tgt_type, false);
        check_round_trip(8, 100, tgt_type, true);
        check_round_trip(8, 100, tgt_type, false);
        check_round_trip(11, 100_000, tgt_type, true);
        check_round_trip(11, 100_000, tgt_type, false);
    }
}
```

- [ ] **Step 12: Run it**

```bash
cargo test -p apache-datasketches --test hll_sketch_test hll_sketch_check_compact_flag
```

Expected: PASS.

- [ ] **Step 13: Port the remaining test cases**

For each remaining upstream test case below, read its body in `vendor/datasketches-cpp/hll/test/HllSketchTest.cpp`, port its assertions using the same pattern as Steps 1–12 (construct via `HllSketch::new`/`deserialize`, drive via `update_*`, assert via `get_estimate`/`get_lower_bound`/`get_upper_bound`/`is_empty`/`get_lg_config_k`/`get_target_type`/`to_string_summary`/`serialize_compact`/`serialize_updatable`), and add one `#[test]` function per case to `hll_sketch_test.rs`:

- `"hll sketch: check misc1"` → `hll_sketch_check_misc1` — miscellaneous accessor checks (`get_lg_config_k`, `get_target_type`, `is_empty`, `reset`).
- `"hll sketch: check ser sizes"` → `hll_sketch_check_ser_sizes` — assert `serialize_compact().len()` / `serialize_updatable().len()` are within expected bounds for a given `lg_k`/type/fill level.
- `"hll sketch: exercise to string"` → `hll_sketch_exercise_to_string` — assert `to_string_summary()` returns a non-empty string and doesn't panic across sketch states (empty, list mode, set mode, HLL mode).
- `"hll sketch: deserialize list mode buffer overrun"` → `hll_sketch_deserialize_list_mode_buffer_overrun` — serialize a small (list-mode) sketch, truncate the byte buffer, assert `HllSketch::deserialize` on the truncated buffer returns `Err`.
- `"hll sketch: deserialize set mode buffer overrun"` → `hll_sketch_deserialize_set_mode_buffer_overrun` — same pattern, with enough updates to push the sketch into set mode before truncating.
- `"hll sketch: deserialize HLL mode buffer overrun"` → `hll_sketch_deserialize_hll_mode_buffer_overrun` — same pattern, with enough updates to push the sketch into HLL mode before truncating.
- `"hll sketch: bytes serialize-deserialize-serialize list mode"` → `hll_sketch_bytes_round_trip_list_mode` — serialize, deserialize, re-serialize a list-mode sketch (few updates), assert the two serialized byte buffers are equal.
- `"hll sketch: updatable bytes serialize-deserialize-serialize set mode"` → `hll_sketch_updatable_bytes_round_trip_set_mode` — same pattern using `serialize_updatable`, with enough updates for set mode.
- `"hll sketch: compact bytes serialize-deserialize-serialize set mode"` → `hll_sketch_compact_bytes_round_trip_set_mode` — same pattern using `serialize_compact`, set mode.

After writing each, run:

```bash
cargo test -p apache-datasketches --test hll_sketch_test
```

Expected: all tests in the file PASS after each addition.

- [ ] **Step 14: Run the full file once more to confirm all 15 ported tests pass together**

```bash
cargo test -p apache-datasketches --test hll_sketch_test
```

Expected: 15 tests PASS, 0 failed.

- [ ] **Step 15: Commit**

```bash
git add apache-datasketches/tests/hll_sketch_test.rs
git commit -m "Port HllSketchTest.cpp test suite to Rust (15 tests)"
```

---

### Task 10: Port `HllUnionTest.cpp` to `hll_union_test.rs`

**Files:**
- Create: `apache-datasketches/tests/hll_union_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType}` (Tasks 6, 7).
- Consumes upstream source: `vendor/datasketches-cpp/hll/test/HllUnionTest.cpp` (read directly for each test's exact assertions before porting it).

- [ ] **Step 1: Create the file and port test 7 ("check hll to hll") first — it's already validated by Task 7's smoke test, so start from there**

```rust
// apache-datasketches/tests/hll_union_test.rs
//
// Ported 1:1 from vendor/datasketches-cpp/hll/test/HllUnionTest.cpp (tag 5.2.0).
// Test names and order mirror the upstream Catch2 TEST_CASE list.

use apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType};

fn union_two_sketches_with_overlap(num: u64, lg_k: u8, tgt_type: TargetHllType) {
    let mut sketch1 = HllSketch::new(lg_k, tgt_type).unwrap();
    for key in 0..num {
        sketch1.update_u64(key);
    }

    let overlap = num / 2;
    let mut sketch2 = HllSketch::new(lg_k, tgt_type).unwrap();
    for key in overlap..(num + overlap) {
        sketch2.update_u64(key);
    }

    let mut u = HllUnion::new(lg_k).unwrap();
    u.update_sketch(&sketch1);
    u.update_sketch(&sketch2);
    let sketch = u.get_result(tgt_type);

    let expected = num as f64 * 1.5;
    assert!(
        (sketch.get_estimate() - expected).abs() < expected * 0.02,
        "estimate was {}",
        sketch.get_estimate()
    );
}

#[test]
fn hll_union_check_hll_to_hll() {
    union_two_sketches_with_overlap(1_000_000, 11, TargetHllType::Hll4);
}
```

Note: `lg_k = 11` and `num = 1_000_000` matches the upstream test exactly; this makes the test slower (~seconds) but preserves parity — do not shrink `num` when porting, since the 2% error-margin assertion is calibrated to that sample size.

- [ ] **Step 2: Run it**

```bash
cargo test -p apache-datasketches --test hll_union_test hll_union_check_hll_to_hll
```

Expected: PASS.

- [ ] **Step 3: Port test 3 ("check config k limits") — validates union constructor bounds**

Per this plan's Reference section, `lg_max_k` valid range is `[7, 21]` — confirm the exact lower bound against `vendor/datasketches-cpp/hll/include/hll.hpp`'s `hll_union_alloc` constructor doc comment (checked in Task 1 Step 2) before finalizing the boundary values below.

```rust
#[test]
fn hll_union_check_config_k_limits() {
    assert!(HllUnion::new(7).is_ok());
    assert!(HllUnion::new(21).is_ok());
    assert!(HllUnion::new(6).is_err());
    assert!(HllUnion::new(22).is_err());
}
```

- [ ] **Step 4: Run it**

```bash
cargo test -p apache-datasketches --test hll_union_test hll_union_check_config_k_limits
```

Expected: PASS. If it fails because `6` is actually valid, adjust to the boundary discovered in Task 1 Step 2 and re-run.

- [ ] **Step 5: Port test 6 ("check input types") — mirrors the sketch input-types test, exercised through the union**

```rust
#[test]
fn hll_union_check_input_types() {
    let mut u = HllUnion::new(8).unwrap();
    u.update_u64(102);
    u.update_i64(102);
    u.update_f64(-2.0);

    let s = "input string";
    u.update_str(s);
    u.update_bytes(s.as_bytes());

    // Both direct-on-union and via-get_result queries should roughly agree.
    let direct_estimate = u.get_estimate();
    let via_result = u.get_result(TargetHllType::Hll4).get_estimate();
    assert!((direct_estimate - via_result).abs() < 1.0);
}
```

- [ ] **Step 6: Run it**

```bash
cargo test -p apache-datasketches --test hll_union_test hll_union_check_input_types
```

Expected: PASS.

- [ ] **Step 7: Port test 4 ("check ub lb") — validates `get_lower_bound`/`get_upper_bound` on the union**

```rust
#[test]
fn hll_union_check_ub_lb() {
    let mut u = HllUnion::new(8).unwrap();
    u.update_u64(1);

    assert!(u.get_lower_bound(1).is_ok());
    assert!(u.get_lower_bound(2).is_ok());
    assert!(u.get_lower_bound(3).is_ok());
    assert!(u.get_lower_bound(0).is_err());
    assert!(u.get_lower_bound(4).is_err());

    let lb = u.get_lower_bound(1).unwrap();
    let ub = u.get_upper_bound(1).unwrap();
    assert!(lb <= u.get_estimate());
    assert!(u.get_estimate() <= ub);
}
```

- [ ] **Step 8: Run it**

```bash
cargo test -p apache-datasketches --test hll_union_test hll_union_check_ub_lb
```

Expected: PASS.

- [ ] **Step 9: Port the remaining test cases**

For each remaining upstream test case below, read its body in `vendor/datasketches-cpp/hll/test/HllUnionTest.cpp`, port its assertions using the established pattern (construct via `HllUnion::new`/`HllSketch::new`, drive via `update_*`/`update_sketch`, assert via `get_estimate`/`get_result`/bounds), and add one `#[test]` function per case:

- `"hll union: check unions"` → `hll_union_check_unions` — basic single/multiple sketch union correctness at varying `lg_k` and `TargetHllType` combinations.
- `"hll union: check composite estimate"` → `hll_union_check_composite_estimate` — assert the union's estimate stays consistent (within tolerance) as more sketches are added incrementally.
- `"hll union: check conversions"` → `hll_union_check_conversions` — assert `get_result(tgt_type)` for each of `Hll4`/`Hll6`/`Hll8` produces sketches with matching (within tolerance) estimates from the same union state.

After writing each, run:

```bash
cargo test -p apache-datasketches --test hll_union_test
```

Expected: all tests in the file PASS after each addition.

- [ ] **Step 10: Run the full file once more to confirm all 7 ported tests pass together**

```bash
cargo test -p apache-datasketches --test hll_union_test
```

Expected: 7 tests PASS, 0 failed.

- [ ] **Step 11: Commit**

```bash
git add apache-datasketches/tests/hll_union_test.rs
git commit -m "Port HllUnionTest.cpp test suite to Rust (7 tests)"
```

---

### Task 11: Feature-flag compilation check and README finalization

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: everything built in Tasks 1–10.

- [ ] **Step 1: Verify default build (with `hll` feature) works end to end**

```bash
cargo test --workspace
```

Expected: all tests across both crates PASS (sys link test, safe-crate smoke tests, concurrency tests, `hll_sketch_test.rs`, `hll_union_test.rs`).

- [ ] **Step 2: Verify the crate builds with `hll` explicitly disabled (proves feature-gating actually gates the C++ compile, not just the Rust module)**

```bash
cargo build -p apache-datasketches-sys --no-default-features
cargo build -p apache-datasketches --no-default-features
```

Expected: both PASS, with no C++ shim files compiled (confirm via `cargo build -vv -p apache-datasketches-sys --no-default-features 2>&1 | grep -c "hll_sketch_shim.cc"` returning `0`).

- [ ] **Step 3: Update `README.md` with actual usage example**

Replace the "Sketch families" section content with:

```markdown
## Usage

```rust
use apache_datasketches::hll::{HllSketch, TargetHllType};

let mut sketch = HllSketch::new(12, TargetHllType::Hll4)?;
sketch.update_str("some-key");
sketch.update_u64(42);
println!("estimate: {}", sketch.get_estimate());
```

## Sketch families

- [x] HLL (HyperLogLog) — `hll` feature, enabled by default (sketch + union).
```

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "Finalize README with usage example; verify feature-gated builds"
```

---

## Post-plan notes (not part of this plan's scope)

- Publishing to crates.io (name availability check for `apache-datasketches`/`apache-datasketches-sys`, `cargo publish` for both crates in dependency order) is a follow-up once this plan's tests are green.
- Additional sketch families (Theta, KLL, CPC, etc.) follow the same Task 1–10 shape per family, gated by a new Cargo feature, per the design spec's "Scaling to future sketch families" section.
- CI setup was explicitly deferred per the design spec.
