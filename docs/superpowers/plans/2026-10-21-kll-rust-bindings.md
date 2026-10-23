# KLL Rust Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the KLL quantiles sketch family (`f64` and `f32` items, sorted view, Kolmogorov–Smirnov test) to `apache-datasketches`/`apache-datasketches-sys`, behind a new `kll` feature.

**Architecture:** One C++ class template (`KllSketchShim<T>`, `KllSortedViewShim<T>`) wraps upstream `kll_sketch<T>` and `quantiles_sorted_view<T>`, explicitly instantiated for `double` and `float` and exposed to cxx under concrete aliases. Two cxx bridges (`kll_doubles.rs`, `kll_floats.rs`), identical modulo type, declare that surface. The safe crate has a generic `KllSketch<T: KllItem>`; the sealed `KllItem` trait (implemented for `f64`/`f32` by a macro) is the only place per-type dispatch happens.

**Tech Stack:** Rust 2021, `cxx`/`cxx-build` 1.x, vendored Apache DataSketches C++ 5.2.0, C++17.

**Spec:** `docs/superpowers/specs/2026-10-21-kll-rust-bindings-design.md` — read it before starting any task.

## Global Constraints

- Work on branch `feat/kll` (not `main`; `main` is protected and wants a PR).
- Commit messages: conventional prefix (`feat(kll):`, `test(kll):`, `docs(kll):`, `bench(kll):`, `chore(kll):`). **No `Co-Authored-By` trailer and no AI attribution of any kind** in commits, PR bodies or comments.
- Every public item in `apache-datasketches` needs a `///`/`//!` doc comment; `#![warn(missing_docs)]` must stay clean.
- cxx bridge free-function names and opaque type names must be unique across the whole sys crate; prefix free functions `kll_doubles_` / `kll_floats_`. Methods need no prefix.
- `kll_doubles.rs` and `kll_floats.rs` stay identical modulo `f64`↔`f32` and `Doubles`↔`Floats`.
- `k` range is `8..=65535`; default `k` is `200`.
- Errors: bad arguments → `SketchError::InvalidConfig`; query on empty → `SketchError::EmptySketch`; bad bytes → `SketchError::Deserialization`; anything else → `SketchError::Cpp`.
- `KllSketch` is `Send`, **not** `Sync`. `KllSortedView` is `Send + Sync`.
- KLL compaction is randomized (time-seeded, unseedable). Never assert an exact quantile/rank once `n > k`; assert within the rank-error bound. `n`, `k`, min, max, `num_retained`, serialized size, and every result while `n ≤ k` are deterministic and may be asserted exactly.
- Format Rust files with `rustfmt --edition 2021 <file>` (not `cargo fmt -- <file>`; that reformats the whole workspace).
- Versions at the end: `apache-datasketches` `0.3.0`, `apache-datasketches-sys` `0.2.3`.

## File map

| File | Status | Responsibility |
|---|---|---|
| `apache-datasketches-sys/vendor/datasketches-cpp/kll/include/*` | create (copy) | vendored upstream KLL headers |
| `apache-datasketches-sys/vendor/README.md` | modify | copy commands include `kll/` |
| `apache-datasketches-sys/tests/vendor_drift_test.rs` | modify | `MODULES` includes `kll` |
| `apache-datasketches-sys/Cargo.toml` | modify | `kll` feature, link-test targets |
| `apache-datasketches-sys/build.rs` | modify | register bridges, `.cc`, include dirs |
| `apache-datasketches-sys/cpp/kll/kll_sketch_shim.h` | create | template shim classes, aliases, free-fn decls |
| `apache-datasketches-sys/cpp/kll/kll_sketch_shim.cc` | create | template definitions, explicit instantiation, free fns |
| `apache-datasketches-sys/src/kll_doubles.rs` | create | cxx bridge for `double` |
| `apache-datasketches-sys/src/kll_floats.rs` | create | cxx bridge for `float` |
| `apache-datasketches-sys/src/lib.rs` | modify | `pub mod kll_doubles; pub mod kll_floats;` |
| `apache-datasketches-sys/tests/kll_{doubles,floats}_link_test.rs` | create | link tests |
| `apache-datasketches/src/error.rs` | modify | `EmptySketch`, `#[non_exhaustive]` |
| `apache-datasketches/Cargo.toml` | modify | `kll` feature, test/example targets |
| `apache-datasketches/src/lib.rs` | modify | `pub mod kll`, guard, crate docs |
| `apache-datasketches/src/kll/mod.rs` | create | module docs, re-exports |
| `apache-datasketches/src/kll/checks.rs` | create | shared argument validation |
| `apache-datasketches/src/kll/item.rs` | create | sealed `KllItem`, per-type dispatch macro |
| `apache-datasketches/src/kll/sketch.rs` | create | `KllSketch<T>` |
| `apache-datasketches/src/kll/sorted_view.rs` | create | `KllSortedView<T>`, `SortedViewEntry<T>` |
| `apache-datasketches/src/kll/kolmogorov_smirnov.rs` | create | KS free functions |
| `apache-datasketches/tests/kll_common/mod.rs` | create | test helpers shared by KLL tests |
| `apache-datasketches/tests/kll_*.rs`, `kolmogorov_smirnov_test.rs` | create | tests |
| `apache-datasketches/tests/data/kll_sketch_float_one_item_v1.sk` | create (copy) | upstream fixture |
| `apache-datasketches/examples/kll.rs` | create | runnable demo |
| `apache-datasketches/examples/bench_kll_update.rs` | create | Rust bench harness |
| `benches/cpp_reference/kll_update.cc` | create | C++ bench counterpart |
| `benches/cpp_reference/bench_common.h` | modify | `report_fields` |
| `benches/cpp_reference/{run.sh,compare.sh,README.md}` | modify | register the KLL pair |
| `.github/workflows/ci.yml` | modify | feature matrix leg, bench comment |
| `README.md`, `apache-datasketches/README.md`, `apache-datasketches-sys/README.md`, `AGENTS.md`, `CHANGELOG.md` | modify | docs, release notes |

---

### Task 0: Branch

- [ ] **Step 1: Create the branch**

```bash
git checkout -b feat/kll
```

(If executing in a worktree via superpowers:using-git-worktrees, create the worktree on branch `feat/kll` instead.)

---

### Task 1: Vendor the KLL headers and add the sys `kll` feature

**Files:**
- Create: `apache-datasketches-sys/vendor/datasketches-cpp/kll/include/` (copied)
- Modify: `apache-datasketches-sys/vendor/README.md`
- Modify: `apache-datasketches-sys/tests/vendor_drift_test.rs:27`
- Modify: `apache-datasketches-sys/Cargo.toml` (`[features]`)

**Interfaces:**
- Produces: sys feature `kll`; headers `kll_sketch.hpp`, `kll_sketch_impl.hpp`, `kll_helper.hpp`, `kll_helper_impl.hpp` available under `vendor/datasketches-cpp/kll/include`.

- [ ] **Step 1: Extend the drift test first (it must fail)**

In `apache-datasketches-sys/tests/vendor_drift_test.rs`, change:

```rust
/// The header trees `build.rs` puts on the include path.
const MODULES: [&str; 5] = ["common", "hll", "cpc", "theta", "tuple"];
```

to:

```rust
/// The header trees `build.rs` puts on the include path.
const MODULES: [&str; 6] = ["common", "hll", "cpc", "theta", "tuple", "kll"];
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p apache-datasketches-sys --test vendor_drift_test`
Expected: FAIL with `kll: missing from the vendored copy entirely`.

- [ ] **Step 3: Copy the headers**

```bash
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/kll
cp -R vendor/datasketches-cpp/kll/include apache-datasketches-sys/vendor/datasketches-cpp/kll/include
```

- [ ] **Step 4: Run the drift test again**

Run: `cargo test -p apache-datasketches-sys --test vendor_drift_test`
Expected: PASS.

- [ ] **Step 5: Update the vendor README**

In `apache-datasketches-sys/vendor/README.md`:
- In the sentence listing compiled headers, change `` `tuple/include`, `LICENSE`, `NOTICE`) `` to `` `tuple/include`, `kll/include`, `LICENSE`, `NOTICE`) ``.
- In the bash block, after the `mkdir -p .../tuple` line add `mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/kll`, and after the `cp -R .../tuple/include ...` line add `cp -R vendor/datasketches-cpp/kll/include apache-datasketches-sys/vendor/datasketches-cpp/kll/include`.
- In the closing paragraph change `` `common/`+`hll/`+`theta/`+`cpc/`+`tuple/` `` to `` `common/`+`hll/`+`theta/`+`cpc/`+`tuple/`+`kll/` ``.

- [ ] **Step 6: Add the sys feature**

In `apache-datasketches-sys/Cargo.toml` `[features]`, after `tuple = []` add:

```toml
kll = []
```

- [ ] **Step 7: Verify the feature builds (no bridges yet, so this only checks the manifest)**

Run: `cargo build -p apache-datasketches-sys --features kll`
Expected: success.

- [ ] **Step 8: Commit**

```bash
git add apache-datasketches-sys/vendor apache-datasketches-sys/tests/vendor_drift_test.rs apache-datasketches-sys/Cargo.toml
git commit -m "chore(kll): vendor the KLL headers and add the sys kll feature"
```

---

### Task 2: C++ shim, cxx bridges and link tests

**Files:**
- Create: `apache-datasketches-sys/cpp/kll/kll_sketch_shim.h`
- Create: `apache-datasketches-sys/cpp/kll/kll_sketch_shim.cc`
- Create: `apache-datasketches-sys/src/kll_doubles.rs`
- Create: `apache-datasketches-sys/src/kll_floats.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Modify: `apache-datasketches-sys/build.rs`
- Modify: `apache-datasketches-sys/Cargo.toml` (`[[test]]` entries)
- Test: `apache-datasketches-sys/tests/kll_doubles_link_test.rs`, `apache-datasketches-sys/tests/kll_floats_link_test.rs`

**Interfaces:**
- Consumes: sys feature `kll` and vendored headers (Task 1).
- Produces (module `apache_datasketches_sys::kll_doubles::ffi`; `kll_floats::ffi` is identical with `f32`/`Floats`):
  - opaque types `KllDoublesSketchShim`, `KllDoublesSortedViewShim`
  - `fn kll_doubles_new(k: u16) -> Result<UniquePtr<KllDoublesSketchShim>>`
  - `fn kll_doubles_deserialize(bytes: &[u8]) -> Result<UniquePtr<KllDoublesSketchShim>>`
  - `fn kll_doubles_normalized_rank_error(k: u16, pmf: bool) -> f64`
  - sketch methods: `clone_shim(&self) -> UniquePtr<Self>`, `update(Pin<&mut Self>, f64)`, `merge(Pin<&mut Self>, &Self)`, `is_empty`, `get_n -> u64`, `get_k -> u16`, `get_num_retained -> u32`, `is_estimation_mode`, `get_min_item -> Result<f64>`, `get_max_item -> Result<f64>`, `get_quantile(f64, bool) -> Result<f64>`, `get_rank(f64, bool) -> Result<f64>`, `get_pmf(&[f64], bool) -> Result<UniquePtr<CxxVector<f64>>>`, `get_cdf(..)` (same), `get_normalized_rank_error(bool) -> f64`, `serialize -> UniquePtr<CxxVector<u8>>`, `to_string_summary -> String`, `retained_items -> UniquePtr<CxxVector<f64>>`, `retained_weights -> UniquePtr<CxxVector<u64>>`, `sorted_view -> UniquePtr<KllDoublesSortedViewShim>`, `ks_delta(&Self) -> f64`, `ks_threshold(&Self, f64) -> f64`, `ks_test(&Self, f64) -> bool`
  - view methods: `size -> usize`, `get_quantile(f64, bool) -> Result<f64>`, `get_rank(f64, bool) -> Result<f64>`, `get_pmf`/`get_cdf` (as on the sketch), `items -> UniquePtr<CxxVector<f64>>`, `weights -> UniquePtr<CxxVector<u64>>`, `cumulative_weights -> UniquePtr<CxxVector<u64>>`

- [ ] **Step 1: Write the doubles link test**

Create `apache-datasketches-sys/tests/kll_doubles_link_test.rs`:

```rust
#![cfg(feature = "kll")]

use apache_datasketches_sys::kll_doubles::ffi;

#[test]
fn construct_update_query_roundtrip() {
    let mut sketch = ffi::kll_doubles_new(200).unwrap();
    for i in 1..=100 {
        sketch.pin_mut().update(f64::from(i));
    }
    assert_eq!(sketch.get_n(), 100);
    assert_eq!(sketch.get_k(), 200);
    assert_eq!(sketch.get_num_retained(), 100);
    assert_eq!(sketch.get_min_item().unwrap(), 1.0);
    assert_eq!(sketch.get_max_item().unwrap(), 100.0);
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), 50.0);
    assert_eq!(sketch.get_rank(50.0, true).unwrap(), 0.5);
    assert_eq!(sketch.get_cdf(&[50.0], true).unwrap().as_slice(), &[0.5, 1.0]);

    let restored = ffi::kll_doubles_deserialize(sketch.serialize().as_slice()).unwrap();
    assert_eq!(restored.get_n(), 100);

    let view = sketch.sorted_view();
    assert_eq!(view.size(), 100);
    assert_eq!(view.items().as_slice()[0], 1.0);
    assert_eq!(view.cumulative_weights().as_slice()[99], 100);

    let copy = sketch.clone_shim();
    assert_eq!(sketch.ks_delta(&copy), 0.0);
}

#[test]
fn invalid_k_returns_err() {
    assert!(ffi::kll_doubles_new(7).is_err());
}

#[test]
fn query_on_empty_returns_err() {
    let sketch = ffi::kll_doubles_new(200).unwrap();
    assert!(sketch.get_quantile(0.5, true).is_err());
}

#[test]
fn static_rank_error_matches_instance() {
    let sketch = ffi::kll_doubles_new(200).unwrap();
    assert_eq!(
        ffi::kll_doubles_normalized_rank_error(200, false),
        sketch.get_normalized_rank_error(false)
    );
}
```

Create `apache-datasketches-sys/tests/kll_floats_link_test.rs`:

```rust
#![cfg(feature = "kll")]

use apache_datasketches_sys::kll_floats::ffi;

#[test]
fn construct_update_query_roundtrip() {
    let mut sketch = ffi::kll_floats_new(200).unwrap();
    for i in 1..=100u8 {
        sketch.pin_mut().update(f32::from(i));
    }
    assert_eq!(sketch.get_n(), 100);
    assert_eq!(sketch.get_k(), 200);
    assert_eq!(sketch.get_num_retained(), 100);
    assert_eq!(sketch.get_min_item().unwrap(), 1.0f32);
    assert_eq!(sketch.get_max_item().unwrap(), 100.0f32);
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), 50.0f32);
    assert_eq!(sketch.get_rank(50.0f32, true).unwrap(), 0.5);
    assert_eq!(sketch.get_cdf(&[50.0f32], true).unwrap().as_slice(), &[0.5, 1.0]);

    let restored = ffi::kll_floats_deserialize(sketch.serialize().as_slice()).unwrap();
    assert_eq!(restored.get_n(), 100);

    let view = sketch.sorted_view();
    assert_eq!(view.size(), 100);
    assert_eq!(view.items().as_slice()[0], 1.0f32);
    assert_eq!(view.cumulative_weights().as_slice()[99], 100);

    let copy = sketch.clone_shim();
    assert_eq!(sketch.ks_delta(&copy), 0.0);
}

#[test]
fn invalid_k_returns_err() {
    assert!(ffi::kll_floats_new(7).is_err());
}

#[test]
fn query_on_empty_returns_err() {
    let sketch = ffi::kll_floats_new(200).unwrap();
    assert!(sketch.get_quantile(0.5, true).is_err());
}

#[test]
fn static_rank_error_matches_instance() {
    let sketch = ffi::kll_floats_new(200).unwrap();
    assert_eq!(
        ffi::kll_floats_normalized_rank_error(200, false),
        sketch.get_normalized_rank_error(false)
    );
}
```

- [ ] **Step 2: Register the test targets**

Append to `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "kll_doubles_link_test"
required-features = ["kll"]

[[test]]
name = "kll_floats_link_test"
required-features = ["kll"]
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features kll --test kll_doubles_link_test`
Expected: FAIL to compile — `could not find kll_doubles in apache_datasketches_sys`.

- [ ] **Step 4: Write the shim header**

Create `apache-datasketches-sys/cpp/kll/kll_sketch_shim.h`:

```cpp
#pragma once
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "kll_sketch.hpp"

namespace apache_datasketches_rs {

// One template serves both item types. cxx cannot name a template, so the
// concrete aliases at the bottom of this file are what the bridges declare,
// and kll_sketch_shim.cc explicitly instantiates both.

template<typename T>
using kll_sorted_view_t = datasketches::quantiles_sorted_view<T, std::less<T>, std::allocator<T>>;

template<typename T>
class KllSortedViewShim {
public:
  explicit KllSortedViewShim(kll_sorted_view_t<T> view);

  size_t size() const;
  T get_quantile(double rank, bool inclusive) const;
  double get_rank(T item, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_pmf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_cdf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<T>> items() const;
  std::unique_ptr<std::vector<uint64_t>> weights() const;
  std::unique_ptr<std::vector<uint64_t>> cumulative_weights() const;

private:
  kll_sorted_view_t<T> view_;
};

template<typename T>
class KllSketchShim {
public:
  explicit KllSketchShim(uint16_t k);
  explicit KllSketchShim(datasketches::kll_sketch<T> sketch);

  std::unique_ptr<KllSketchShim> clone_shim() const;
  void update(T item);
  void merge(const KllSketchShim& other);

  bool is_empty() const;
  uint64_t get_n() const;
  uint16_t get_k() const;
  uint32_t get_num_retained() const;
  bool is_estimation_mode() const;
  T get_min_item() const;
  T get_max_item() const;

  T get_quantile(double rank, bool inclusive) const;
  double get_rank(T item, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_pmf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_cdf(rust::Slice<const T> split_points, bool inclusive) const;
  double get_normalized_rank_error(bool pmf) const;

  std::unique_ptr<std::vector<uint8_t>> serialize() const;
  rust::String to_string_summary() const;

  std::unique_ptr<std::vector<T>> retained_items() const;
  std::unique_ptr<std::vector<uint64_t>> retained_weights() const;
  std::unique_ptr<KllSortedViewShim<T>> sorted_view() const;

  double ks_delta(const KllSketchShim& other) const;
  double ks_threshold(const KllSketchShim& other, double p) const;
  bool ks_test(const KllSketchShim& other, double p) const;

private:
  datasketches::kll_sketch<T> sketch_;
};

using KllDoublesSketchShim = KllSketchShim<double>;
using KllFloatsSketchShim = KllSketchShim<float>;
using KllDoublesSortedViewShim = KllSortedViewShim<double>;
using KllFloatsSortedViewShim = KllSortedViewShim<float>;

std::unique_ptr<KllDoublesSketchShim> kll_doubles_new(uint16_t k);
std::unique_ptr<KllDoublesSketchShim> kll_doubles_deserialize(rust::Slice<const uint8_t> bytes);
double kll_doubles_normalized_rank_error(uint16_t k, bool pmf);

std::unique_ptr<KllFloatsSketchShim> kll_floats_new(uint16_t k);
std::unique_ptr<KllFloatsSketchShim> kll_floats_deserialize(rust::Slice<const uint8_t> bytes);
double kll_floats_normalized_rank_error(uint16_t k, bool pmf);

} // namespace apache_datasketches_rs
```

- [ ] **Step 5: Write the shim implementation**

Create `apache-datasketches-sys/cpp/kll/kll_sketch_shim.cc`:

```cpp
#include "kll_sketch_shim.h"
#include "kolmogorov_smirnov.hpp"

namespace apache_datasketches_rs {

// --- KllSortedViewShim -------------------------------------------------------

template<typename T>
KllSortedViewShim<T>::KllSortedViewShim(kll_sorted_view_t<T> view) : view_(std::move(view)) {}

template<typename T>
size_t KllSortedViewShim<T>::size() const { return view_.size(); }

template<typename T>
T KllSortedViewShim<T>::get_quantile(double rank, bool inclusive) const {
  return view_.get_quantile(rank, inclusive);
}

template<typename T>
double KllSortedViewShim<T>::get_rank(T item, bool inclusive) const {
  return view_.get_rank(item, inclusive);
}

template<typename T>
std::unique_ptr<std::vector<double>> KllSortedViewShim<T>::get_pmf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(view_.get_PMF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
std::unique_ptr<std::vector<double>> KllSortedViewShim<T>::get_cdf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(view_.get_CDF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

// The three accessors below each walk the view once and hand back a whole
// std::vector, for the same reason serialize does: building a rust::Vec here
// would cost two calls back into Rust per element.
template<typename T>
std::unique_ptr<std::vector<T>> KllSortedViewShim<T>::items() const {
  auto out = std::make_unique<std::vector<T>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) out->push_back((*it).first);
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> KllSortedViewShim<T>::weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) out->push_back(it.get_weight());
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> KllSortedViewShim<T>::cumulative_weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) {
    out->push_back(it.get_cumulative_weight(true));
  }
  return out;
}

// --- KllSketchShim -----------------------------------------------------------

template<typename T>
KllSketchShim<T>::KllSketchShim(uint16_t k) : sketch_(k) {}

template<typename T>
KllSketchShim<T>::KllSketchShim(datasketches::kll_sketch<T> sketch) : sketch_(std::move(sketch)) {}

template<typename T>
std::unique_ptr<KllSketchShim<T>> KllSketchShim<T>::clone_shim() const {
  return std::make_unique<KllSketchShim<T>>(sketch_);
}

template<typename T>
void KllSketchShim<T>::update(T item) { sketch_.update(item); }

template<typename T>
void KllSketchShim<T>::merge(const KllSketchShim& other) { sketch_.merge(other.sketch_); }

template<typename T>
bool KllSketchShim<T>::is_empty() const { return sketch_.is_empty(); }

template<typename T>
uint64_t KllSketchShim<T>::get_n() const { return sketch_.get_n(); }

template<typename T>
uint16_t KllSketchShim<T>::get_k() const { return sketch_.get_k(); }

template<typename T>
uint32_t KllSketchShim<T>::get_num_retained() const { return sketch_.get_num_retained(); }

template<typename T>
bool KllSketchShim<T>::is_estimation_mode() const { return sketch_.is_estimation_mode(); }

template<typename T>
T KllSketchShim<T>::get_min_item() const { return sketch_.get_min_item(); }

template<typename T>
T KllSketchShim<T>::get_max_item() const { return sketch_.get_max_item(); }

template<typename T>
T KllSketchShim<T>::get_quantile(double rank, bool inclusive) const {
  return sketch_.get_quantile(rank, inclusive);
}

template<typename T>
double KllSketchShim<T>::get_rank(T item, bool inclusive) const {
  return sketch_.get_rank(item, inclusive);
}

template<typename T>
std::unique_ptr<std::vector<double>> KllSketchShim<T>::get_pmf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(sketch_.get_PMF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
std::unique_ptr<std::vector<double>> KllSketchShim<T>::get_cdf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(sketch_.get_CDF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
double KllSketchShim<T>::get_normalized_rank_error(bool pmf) const {
  return sketch_.get_normalized_rank_error(pmf);
}

// Returned whole as a heap std::vector; see cpc_sketch_shim.cc for why not a
// rust::Vec built element by element.
template<typename T>
std::unique_ptr<std::vector<uint8_t>> KllSketchShim<T>::serialize() const {
  return std::make_unique<std::vector<uint8_t>>(sketch_.serialize());
}

template<typename T>
rust::String KllSketchShim<T>::to_string_summary() const {
  return rust::String(std::string(sketch_.to_string().c_str()));
}

template<typename T>
std::unique_ptr<std::vector<T>> KllSketchShim<T>::retained_items() const {
  auto out = std::make_unique<std::vector<T>>();
  out->reserve(sketch_.get_num_retained());
  for (auto pair : sketch_) out->push_back(pair.first);
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> KllSketchShim<T>::retained_weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(sketch_.get_num_retained());
  for (auto pair : sketch_) out->push_back(pair.second);
  return out;
}

template<typename T>
std::unique_ptr<KllSortedViewShim<T>> KllSketchShim<T>::sorted_view() const {
  return std::make_unique<KllSortedViewShim<T>>(sketch_.get_sorted_view());
}

template<typename T>
double KllSketchShim<T>::ks_delta(const KllSketchShim& other) const {
  return datasketches::kolmogorov_smirnov::delta(sketch_, other.sketch_);
}

template<typename T>
double KllSketchShim<T>::ks_threshold(const KllSketchShim& other, double p) const {
  return datasketches::kolmogorov_smirnov::threshold(sketch_, other.sketch_, p);
}

template<typename T>
bool KllSketchShim<T>::ks_test(const KllSketchShim& other, double p) const {
  return datasketches::kolmogorov_smirnov::test(sketch_, other.sketch_, p);
}

template class KllSortedViewShim<double>;
template class KllSortedViewShim<float>;
template class KllSketchShim<double>;
template class KllSketchShim<float>;

// --- free functions ----------------------------------------------------------

std::unique_ptr<KllDoublesSketchShim> kll_doubles_new(uint16_t k) {
  return std::make_unique<KllDoublesSketchShim>(k);
}

std::unique_ptr<KllDoublesSketchShim> kll_doubles_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<KllDoublesSketchShim>(
      datasketches::kll_sketch<double>::deserialize(bytes.data(), bytes.size()));
}

double kll_doubles_normalized_rank_error(uint16_t k, bool pmf) {
  return datasketches::kll_sketch<double>::get_normalized_rank_error(k, pmf);
}

std::unique_ptr<KllFloatsSketchShim> kll_floats_new(uint16_t k) {
  return std::make_unique<KllFloatsSketchShim>(k);
}

std::unique_ptr<KllFloatsSketchShim> kll_floats_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<KllFloatsSketchShim>(
      datasketches::kll_sketch<float>::deserialize(bytes.data(), bytes.size()));
}

double kll_floats_normalized_rank_error(uint16_t k, bool pmf) {
  return datasketches::kll_sketch<float>::get_normalized_rank_error(k, pmf);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 6: Write the doubles bridge**

Create `apache-datasketches-sys/src/kll_doubles.rs`:

```rust
//! cxx bridge for `kll_sketch<double>` and its sorted view.
//!
//! Keep identical to `kll_floats.rs` modulo `f64`/`f32` and
//! `Doubles`/`Floats`: both declare the one C++ template in
//! `cpp/kll/kll_sketch_shim.h`. Free functions carry the `kll_doubles_`
//! prefix because cxx keys their symbols on name alone (see AGENTS.md);
//! methods need no prefix.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("kll_sketch_shim.h");

        type KllDoublesSketchShim;
        type KllDoublesSortedViewShim;

        fn kll_doubles_new(k: u16) -> Result<UniquePtr<KllDoublesSketchShim>>;
        fn kll_doubles_deserialize(bytes: &[u8]) -> Result<UniquePtr<KllDoublesSketchShim>>;
        fn kll_doubles_normalized_rank_error(k: u16, pmf: bool) -> f64;

        fn clone_shim(self: &KllDoublesSketchShim) -> UniquePtr<KllDoublesSketchShim>;
        fn update(self: Pin<&mut KllDoublesSketchShim>, item: f64);
        fn merge(self: Pin<&mut KllDoublesSketchShim>, other: &KllDoublesSketchShim);

        fn is_empty(self: &KllDoublesSketchShim) -> bool;
        fn get_n(self: &KllDoublesSketchShim) -> u64;
        fn get_k(self: &KllDoublesSketchShim) -> u16;
        fn get_num_retained(self: &KllDoublesSketchShim) -> u32;
        fn is_estimation_mode(self: &KllDoublesSketchShim) -> bool;
        fn get_min_item(self: &KllDoublesSketchShim) -> Result<f64>;
        fn get_max_item(self: &KllDoublesSketchShim) -> Result<f64>;

        fn get_quantile(self: &KllDoublesSketchShim, rank: f64, inclusive: bool) -> Result<f64>;
        fn get_rank(self: &KllDoublesSketchShim, item: f64, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &KllDoublesSketchShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &KllDoublesSketchShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_normalized_rank_error(self: &KllDoublesSketchShim, pmf: bool) -> f64;

        fn serialize(self: &KllDoublesSketchShim) -> UniquePtr<CxxVector<u8>>;
        fn to_string_summary(self: &KllDoublesSketchShim) -> String;

        fn retained_items(self: &KllDoublesSketchShim) -> UniquePtr<CxxVector<f64>>;
        fn retained_weights(self: &KllDoublesSketchShim) -> UniquePtr<CxxVector<u64>>;
        fn sorted_view(self: &KllDoublesSketchShim) -> UniquePtr<KllDoublesSortedViewShim>;

        fn ks_delta(self: &KllDoublesSketchShim, other: &KllDoublesSketchShim) -> f64;
        fn ks_threshold(self: &KllDoublesSketchShim, other: &KllDoublesSketchShim, p: f64) -> f64;
        fn ks_test(self: &KllDoublesSketchShim, other: &KllDoublesSketchShim, p: f64) -> bool;

        fn size(self: &KllDoublesSortedViewShim) -> usize;
        fn get_quantile(self: &KllDoublesSortedViewShim, rank: f64, inclusive: bool)
            -> Result<f64>;
        fn get_rank(self: &KllDoublesSortedViewShim, item: f64, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &KllDoublesSortedViewShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &KllDoublesSortedViewShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn items(self: &KllDoublesSortedViewShim) -> UniquePtr<CxxVector<f64>>;
        fn weights(self: &KllDoublesSortedViewShim) -> UniquePtr<CxxVector<u64>>;
        fn cumulative_weights(self: &KllDoublesSortedViewShim) -> UniquePtr<CxxVector<u64>>;
    }
}
```

- [ ] **Step 7: Write the floats bridge**

Create `apache-datasketches-sys/src/kll_floats.rs`. It differs from `kll_doubles.rs` only in names and in item-typed positions (`update`'s item, min/max/quantile returns, `get_rank`'s item, `split_points`, `retained_items`/`items`); ranks, `p`, rank errors and PMF/CDF masses stay `f64` because upstream returns `double` for both item types.

```rust
//! cxx bridge for `kll_sketch<float>` and its sorted view.
//!
//! Keep identical to `kll_doubles.rs` modulo `f64`/`f32` and
//! `Doubles`/`Floats`: both declare the one C++ template in
//! `cpp/kll/kll_sketch_shim.h`. Free functions carry the `kll_floats_`
//! prefix because cxx keys their symbols on name alone (see AGENTS.md);
//! methods need no prefix.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("kll_sketch_shim.h");

        type KllFloatsSketchShim;
        type KllFloatsSortedViewShim;

        fn kll_floats_new(k: u16) -> Result<UniquePtr<KllFloatsSketchShim>>;
        fn kll_floats_deserialize(bytes: &[u8]) -> Result<UniquePtr<KllFloatsSketchShim>>;
        fn kll_floats_normalized_rank_error(k: u16, pmf: bool) -> f64;

        fn clone_shim(self: &KllFloatsSketchShim) -> UniquePtr<KllFloatsSketchShim>;
        fn update(self: Pin<&mut KllFloatsSketchShim>, item: f32);
        fn merge(self: Pin<&mut KllFloatsSketchShim>, other: &KllFloatsSketchShim);

        fn is_empty(self: &KllFloatsSketchShim) -> bool;
        fn get_n(self: &KllFloatsSketchShim) -> u64;
        fn get_k(self: &KllFloatsSketchShim) -> u16;
        fn get_num_retained(self: &KllFloatsSketchShim) -> u32;
        fn is_estimation_mode(self: &KllFloatsSketchShim) -> bool;
        fn get_min_item(self: &KllFloatsSketchShim) -> Result<f32>;
        fn get_max_item(self: &KllFloatsSketchShim) -> Result<f32>;

        fn get_quantile(self: &KllFloatsSketchShim, rank: f64, inclusive: bool) -> Result<f32>;
        fn get_rank(self: &KllFloatsSketchShim, item: f32, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &KllFloatsSketchShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &KllFloatsSketchShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_normalized_rank_error(self: &KllFloatsSketchShim, pmf: bool) -> f64;

        fn serialize(self: &KllFloatsSketchShim) -> UniquePtr<CxxVector<u8>>;
        fn to_string_summary(self: &KllFloatsSketchShim) -> String;

        fn retained_items(self: &KllFloatsSketchShim) -> UniquePtr<CxxVector<f32>>;
        fn retained_weights(self: &KllFloatsSketchShim) -> UniquePtr<CxxVector<u64>>;
        fn sorted_view(self: &KllFloatsSketchShim) -> UniquePtr<KllFloatsSortedViewShim>;

        fn ks_delta(self: &KllFloatsSketchShim, other: &KllFloatsSketchShim) -> f64;
        fn ks_threshold(self: &KllFloatsSketchShim, other: &KllFloatsSketchShim, p: f64) -> f64;
        fn ks_test(self: &KllFloatsSketchShim, other: &KllFloatsSketchShim, p: f64) -> bool;

        fn size(self: &KllFloatsSortedViewShim) -> usize;
        fn get_quantile(self: &KllFloatsSortedViewShim, rank: f64, inclusive: bool)
            -> Result<f32>;
        fn get_rank(self: &KllFloatsSortedViewShim, item: f32, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &KllFloatsSortedViewShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &KllFloatsSortedViewShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn items(self: &KllFloatsSortedViewShim) -> UniquePtr<CxxVector<f32>>;
        fn weights(self: &KllFloatsSortedViewShim) -> UniquePtr<CxxVector<u64>>;
        fn cumulative_weights(self: &KllFloatsSortedViewShim) -> UniquePtr<CxxVector<u64>>;
    }
}
```

- [ ] **Step 8: Expose the modules**

Append to `apache-datasketches-sys/src/lib.rs`:

```rust

#[cfg(feature = "kll")]
pub mod kll_doubles;
#[cfg(feature = "kll")]
pub mod kll_floats;
```

- [ ] **Step 9: Register in `build.rs`**

In `apache-datasketches-sys/build.rs`:

After the `if cfg!(feature = "tuple") { ... bridges ... }` block add:

```rust
    if cfg!(feature = "kll") {
        // Two bridges over one shim: kll_sketch_shim.h holds a single C++
        // template, and each bridge declares one of its instantiations.
        for path in ["src/kll_doubles.rs", "src/kll_floats.rs"] {
            require_exists(path);
            bridges.push(path);
        }
    }
```

In the `build.include(...)` chain, after `.include(vendor_dir.join("tuple/include"))` add `.include(vendor_dir.join("kll/include"))`, and after `.include("cpp/tuple")` add `.include("cpp/kll")`.

After the `if cfg!(feature = "tuple") { ... build.file ... }` block add:

```rust
    if cfg!(feature = "kll") {
        let path = "cpp/kll/kll_sketch_shim.cc";
        require_exists(path);
        build.file(path);
    }
```

At the end of the `rerun-if-changed` list add:

```rust
    println!("cargo:rerun-if-changed=src/kll_doubles.rs");
    println!("cargo:rerun-if-changed=src/kll_floats.rs");
    println!("cargo:rerun-if-changed=cpp/kll/kll_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/kll/kll_sketch_shim.cc");
```

- [ ] **Step 10: Run both link tests**

Run: `cargo test -p apache-datasketches-sys --features kll`
Expected: PASS (both link tests, plus `vendor_drift_test`).

- [ ] **Step 11: Check the name-uniqueness guard sees the new bridges with every other family**

Run: `cargo build -p apache-datasketches-sys --all-features`
Expected: success (the `build.rs` duplicate-name scan would panic on a collision).

- [ ] **Step 12: Format and commit**

```bash
rustfmt --edition 2021 apache-datasketches-sys/src/kll_doubles.rs apache-datasketches-sys/src/kll_floats.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/build.rs apache-datasketches-sys/tests/kll_doubles_link_test.rs apache-datasketches-sys/tests/kll_floats_link_test.rs
git add apache-datasketches-sys
git commit -m "feat(kll): add the KLL C++ shim and cxx bridges"
```

---

### Task 3: `SketchError::EmptySketch` and `#[non_exhaustive]`

**Files:**
- Modify: `apache-datasketches/src/error.rs`

**Interfaces:**
- Produces: `SketchError::EmptySketch` (unit variant); `SketchError` is `#[non_exhaustive]`.

- [ ] **Step 1: Check nothing in the workspace matches `SketchError` exhaustively**

Run: `grep -rn "SketchError::" apache-datasketches/tests apache-datasketches/examples | grep "=>"`
Expected: every hit is inside a `match` that also has a wildcard/other arm (currently three `EmptyIntersection` hits in `theta_intersection_test.rs`, `tuple_intersection_test.rs`, `theta_intersection_smoke_test.rs`). Open each and confirm a catch-all arm exists. If one lacks it, add `other => panic!("unexpected: {other:?}"),`.

- [ ] **Step 2: Edit the enum**

In `apache-datasketches/src/error.rs` replace the enum's doc comment and attributes:

```rust
/// Error type shared across all sketch families (HLL, Theta, CPC) in this
/// crate — there is no per-family error type.
#[derive(Debug, Error)]
pub enum SketchError {
```

with:

```rust
/// Error type shared across all sketch families (HLL, Theta, CPC, Tuple,
/// KLL) in this crate — there is no per-family error type.
///
/// Marked `#[non_exhaustive]` so a new family can add a variant without a
/// breaking release: a `match` on it needs a wildcard arm.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SketchError {
```

and after the `EmptyIntersection` variant add:

```rust

    /// A query that needs data (a quantile, rank, PMF/CDF, or a
    /// Kolmogorov–Smirnov test) was made against an empty sketch.
    #[error("operation is undefined for an empty sketch")]
    EmptySketch,
```

- [ ] **Step 3: Build and test the whole crate**

Run: `cargo test -p apache-datasketches`
Expected: PASS (no new tests; this proves nothing outside the crate matched exhaustively).

- [ ] **Step 4: Commit**

```bash
git add apache-datasketches/src/error.rs
git commit -m "feat!: add SketchError::EmptySketch and mark SketchError non_exhaustive"
```

---

### Task 4: `KllItem` and `KllSketch<T>` core

**Files:**
- Modify: `apache-datasketches/Cargo.toml`
- Modify: `apache-datasketches/src/lib.rs`
- Create: `apache-datasketches/src/kll/mod.rs`
- Create: `apache-datasketches/src/kll/checks.rs`
- Create: `apache-datasketches/src/kll/item.rs`
- Create: `apache-datasketches/src/kll/sketch.rs`
- Create: `apache-datasketches/tests/kll_common/mod.rs`
- Test: `apache-datasketches/tests/kll_sketch_smoke_test.rs`

**Interfaces:**
- Consumes: sys `kll_doubles::ffi`, `kll_floats::ffi` (Task 2); `SketchError::EmptySketch` (Task 3).
- Produces:
  - `pub trait KllItem: sealed::Sealed + Copy + PartialOrd + Debug + Send + Sync + 'static` for `f64`, `f32`.
  - crate-private `item::sealed::Sealed` with `type Sketch` and associated fns (names below), extended by Tasks 5 and 6.
  - `pub struct KllSketch<T: KllItem>` with: `new(u16) -> Result<Self, SketchError>`, `update(&mut self, T)`, `merge(&mut self, &KllSketch<T>)`, `is_empty`, `get_n -> u64`, `get_k -> u16`, `get_num_retained -> u32`, `is_estimation_mode`, `get_min_item -> Option<T>`, `get_max_item -> Option<T>`, `get_quantile(f64, bool) -> Result<T, SketchError>`, `get_rank(T, bool) -> Result<f64, SketchError>`, `get_pmf(&[T], bool) -> Result<Vec<f64>, SketchError>`, `get_cdf(&[T], bool) -> Result<Vec<f64>, SketchError>`, `get_normalized_rank_error(bool) -> f64`, `normalized_rank_error_for_k(u16, bool) -> Result<f64, SketchError>`, `serialize -> Vec<u8>`, `deserialize(&[u8]) -> Result<Self, SketchError>`, `to_string_summary -> String`, `iter -> impl Iterator<Item = (T, u64)>`; `Clone`, `Default`, `Send`.
  - `checks::{check_k, check_rank, check_item, check_split_points}` (all `pub(crate)`, all `-> Result<(), SketchError>`). Task 6 adds `check_p` and `KllSketch::shim()`; they are left out here because nothing would call them yet and `-D warnings` would reject the dead code.
  - test helpers in `tests/kll_common/mod.rs`: `trait TestItem: KllItem { fn from_f64(f64) -> Self; fn to_f64(self) -> f64; }`, macro `for_both_types!(module_name, generic_fn)`, `struct Normal` (Box–Muller sampler, used in Task 6).

- [ ] **Step 1: Add the safe-crate feature and targets**

In `apache-datasketches/Cargo.toml`:
- `default = ["hll", "theta", "cpc", "tuple"]` → `default = ["hll", "theta", "cpc", "tuple", "kll"]`
- after `tuple = ["apache-datasketches-sys/tuple"]` add `kll = ["apache-datasketches-sys/kll"]`
- in `[package.metadata.docs.rs]`, `features = ["hll", "theta", "cpc", "tuple"]` → `features = ["hll", "theta", "cpc", "tuple", "kll"]`
- append:

```toml
[[test]]
name = "kll_sketch_smoke_test"
required-features = ["kll"]
```

- [ ] **Step 2: Write the test helpers**

Create `apache-datasketches/tests/kll_common/mod.rs`:

```rust
//! Helpers shared by the KLL integration tests. Each test file pulls this in
//! with `#[macro_use] mod kll_common;`.

// Not every test file uses every helper.
#![allow(dead_code, unused_macros)]

use apache_datasketches::kll::KllItem;

/// The item types under test, with the lossy conversions the tests need.
/// Every value the tests feed through `from_f64` is an integer below 2^24,
/// so it is exact in `f32` too.
pub trait TestItem: KllItem {
    fn from_f64(v: f64) -> Self;
    fn to_f64(self) -> f64;
}

impl TestItem for f64 {
    fn from_f64(v: f64) -> Self {
        v
    }
    fn to_f64(self) -> f64 {
        self
    }
}

impl TestItem for f32 {
    fn from_f64(v: f64) -> Self {
        v as f32
    }
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
}

/// Generates a module holding one `#[test]` per item type, each calling the
/// generic test body `$body::<T>()`.
macro_rules! for_both_types {
    ($name:ident, $body:ident) => {
        mod $name {
            #[test]
            fn as_f64() {
                super::$body::<f64>();
            }
            #[test]
            fn as_f32() {
                super::$body::<f32>();
            }
        }
    };
}

/// Deterministic standard-normal sampler (splitmix64 + Box–Muller), so the
/// statistical tests need no `rand` dependency and never vary run to run on
/// the input side. (KLL's own compaction is still randomized.)
pub struct Normal {
    state: u64,
    spare: Option<f64>,
}

impl Normal {
    pub fn new(seed: u64) -> Self {
        Self { state: seed, spare: None }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in the open interval (0, 1), so `ln` below never sees 0.
    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub fn sample(&mut self) -> f64 {
        if let Some(s) = self.spare.take() {
            return s;
        }
        let r = (-2.0 * self.uniform().ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * self.uniform();
        self.spare = Some(r * theta.sin());
        r * theta.cos()
    }
}
```

- [ ] **Step 3: Write the failing smoke test**

Create `apache-datasketches/tests/kll_sketch_smoke_test.rs`:

```rust
#[macro_use]
mod kll_common;

use apache_datasketches::kll::KllSketch;
use apache_datasketches::SketchError;
use kll_common::TestItem;

fn filled<T: TestItem>(k: u16, n: u32) -> KllSketch<T> {
    let mut sketch = KllSketch::<T>::new(k).unwrap();
    for i in 1..=n {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

fn construction<T: TestItem>() {
    assert_eq!(KllSketch::<T>::default().get_k(), 200);
    assert_eq!(KllSketch::<T>::new(8).unwrap().get_k(), 8);
    assert_eq!(KllSketch::<T>::new(u16::MAX).unwrap().get_k(), u16::MAX);
    assert!(matches!(KllSketch::<T>::new(7), Err(SketchError::InvalidConfig(_))));
    assert!(matches!(KllSketch::<T>::new(0), Err(SketchError::InvalidConfig(_))));
}
for_both_types!(construction_t, construction);

fn exact_mode_queries<T: TestItem>() {
    let sketch = filled::<T>(200, 100);
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 100);
    assert_eq!(sketch.get_num_retained(), 100);
    assert_eq!(sketch.get_min_item().unwrap().to_f64(), 1.0);
    assert_eq!(sketch.get_max_item().unwrap().to_f64(), 100.0);
    assert_eq!(sketch.get_quantile(0.5, true).unwrap().to_f64(), 50.0);
    assert_eq!(sketch.get_rank(T::from_f64(50.0), true).unwrap(), 0.5);
    assert_eq!(sketch.get_rank(T::from_f64(50.0), false).unwrap(), 0.49);
    let split = [T::from_f64(25.0), T::from_f64(75.0)];
    assert_eq!(sketch.get_cdf(&split, true).unwrap(), vec![0.25, 0.75, 1.0]);
    assert_eq!(sketch.get_pmf(&split, true).unwrap(), vec![0.25, 0.5, 0.25]);
}
for_both_types!(exact_mode_queries_t, exact_mode_queries);

fn empty_sketch<T: TestItem>() {
    let sketch = KllSketch::<T>::default();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_n(), 0);
    assert_eq!(sketch.get_min_item(), None);
    assert_eq!(sketch.get_max_item(), None);
    assert!(matches!(sketch.get_quantile(0.5, true), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_rank(T::from_f64(0.0), true), Err(SketchError::EmptySketch)));
    let split = [T::from_f64(0.0)];
    assert!(matches!(sketch.get_pmf(&split, true), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_cdf(&split, true), Err(SketchError::EmptySketch)));
    assert_eq!(sketch.iter().count(), 0);
}
for_both_types!(empty_sketch_t, empty_sketch);

fn nan_update_is_ignored<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(T::from_f64(f64::NAN));
    assert!(sketch.is_empty());
}
for_both_types!(nan_update_is_ignored_t, nan_update_is_ignored);

fn merge_combines<T: TestItem>() {
    let mut a = filled::<T>(200, 100);
    let mut b = KllSketch::<T>::default();
    for i in 101..=200 {
        b.update(T::from_f64(f64::from(i)));
    }
    a.merge(&b);
    assert_eq!(a.get_n(), 200);
    assert_eq!(a.get_min_item().unwrap().to_f64(), 1.0);
    assert_eq!(a.get_max_item().unwrap().to_f64(), 200.0);
    assert_eq!(b.get_n(), 100, "merge must not modify its argument");
}
for_both_types!(merge_combines_t, merge_combines);

fn serialize_roundtrip<T: TestItem>() {
    let sketch = filled::<T>(200, 1000);
    let restored = KllSketch::<T>::deserialize(&sketch.serialize()).unwrap();
    assert_eq!(restored.get_n(), sketch.get_n());
    assert_eq!(restored.get_k(), sketch.get_k());
    assert_eq!(restored.get_num_retained(), sketch.get_num_retained());
    assert_eq!(restored.get_min_item(), sketch.get_min_item());
    assert_eq!(restored.get_max_item(), sketch.get_max_item());
    // Same retained items, so even estimation-mode answers are identical.
    assert_eq!(restored.get_quantile(0.5, true).unwrap(), sketch.get_quantile(0.5, true).unwrap());
}
for_both_types!(serialize_roundtrip_t, serialize_roundtrip);

fn garbage_bytes_fail<T: TestItem>() {
    assert!(matches!(
        KllSketch::<T>::deserialize(&[1, 2, 3]),
        Err(SketchError::Deserialization(_))
    ));
}
for_both_types!(garbage_bytes_fail_t, garbage_bytes_fail);

fn iter_covers_every_retained_item<T: TestItem>() {
    let sketch = filled::<T>(200, 10_000);
    let pairs: Vec<(T, u64)> = sketch.iter().collect();
    assert_eq!(pairs.len() as u32, sketch.get_num_retained());
    assert_eq!(pairs.iter().map(|&(_, w)| w).sum::<u64>(), sketch.get_n());
}
for_both_types!(iter_covers_every_retained_item_t, iter_covers_every_retained_item);

fn clone_is_independent<T: TestItem>() {
    let mut original = filled::<T>(200, 10);
    let copy = original.clone();
    original.update(T::from_f64(11.0));
    assert_eq!(original.get_n(), 11);
    assert_eq!(copy.get_n(), 10);
}
for_both_types!(clone_is_independent_t, clone_is_independent);

fn rank_error<T: TestItem>() {
    let sketch = KllSketch::<T>::default();
    assert_eq!(
        KllSketch::<T>::normalized_rank_error_for_k(200, false).unwrap(),
        sketch.get_normalized_rank_error(false)
    );
    assert!(sketch.get_normalized_rank_error(true) > sketch.get_normalized_rank_error(false));
    assert!(matches!(
        KllSketch::<T>::normalized_rank_error_for_k(7, false),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(rank_error_t, rank_error);

fn to_string_summary_mentions_k<T: TestItem>() {
    let summary = filled::<T>(200, 10).to_string_summary();
    assert!(summary.contains("200"), "summary was: {summary}");
}
for_both_types!(to_string_summary_mentions_k_t, to_string_summary_mentions_k);
```

- [ ] **Step 4: Run it to verify it fails**

Run: `cargo test -p apache-datasketches --features kll --test kll_sketch_smoke_test`
Expected: FAIL to compile — `could not find kll in apache_datasketches`.

- [ ] **Step 5: Wire the module into the crate root**

In `apache-datasketches/src/lib.rs`:
- after the `#[cfg(feature = "tuple")] pub mod tuple;` item add:

```rust

#[cfg(feature = "kll")]
pub mod kll;
```

- change the guard to:

```rust
#[cfg(not(any(
    feature = "hll",
    feature = "theta",
    feature = "cpc",
    feature = "tuple",
    feature = "kll"
)))]
compile_error!(
    "apache-datasketches: no sketch family is enabled, so this crate exposes nothing. \
     Enable at least one of the `hll`, `theta`, `cpc`, `tuple`, or `kll` features, or drop \
     `default-features = false`."
);
```

(Keep the phrase `no sketch family is enabled` exactly — CI greps for it.)

- [ ] **Step 6: Write `checks.rs`**

Create `apache-datasketches/src/kll/checks.rs`:

```rust
//! Argument validation shared by the sketch, the sorted view and the
//! Kolmogorov–Smirnov functions. It runs before crossing FFI, so callers get
//! a typed [`SketchError::InvalidConfig`] instead of a C++ exception message.

use super::KllItem;
use crate::error::SketchError;

/// Upstream `kll_constants::MIN_K`. The maximum, 65535, is `u16::MAX`, so the
/// type already enforces it.
pub(crate) const MIN_K: u16 = 8;

pub(crate) fn check_k(k: u16) -> Result<(), SketchError> {
    if k < MIN_K {
        return Err(SketchError::InvalidConfig(format!(
            "k must be in {MIN_K}..=65535, got {k}"
        )));
    }
    Ok(())
}

pub(crate) fn check_rank(rank: f64) -> Result<(), SketchError> {
    // `contains` is false for NaN, so this rejects it too.
    if !(0.0..=1.0).contains(&rank) {
        return Err(SketchError::InvalidConfig(format!(
            "rank must be in [0, 1], got {rank}"
        )));
    }
    Ok(())
}

pub(crate) fn check_item<T: KllItem>(item: T) -> Result<(), SketchError> {
    if T::is_nan_item(item) {
        return Err(SketchError::InvalidConfig("item must not be NaN".into()));
    }
    Ok(())
}

pub(crate) fn check_split_points<T: KllItem>(split_points: &[T]) -> Result<(), SketchError> {
    if u32::try_from(split_points.len()).is_err() {
        return Err(SketchError::InvalidConfig(format!(
            "at most {} split points are supported, got {}",
            u32::MAX,
            split_points.len()
        )));
    }
    if split_points.iter().any(|&p| T::is_nan_item(p)) {
        return Err(SketchError::InvalidConfig(
            "split points must not contain NaN".into(),
        ));
    }
    if !split_points.windows(2).all(|w| w[0] < w[1]) {
        return Err(SketchError::InvalidConfig(
            "split points must be unique and strictly increasing".into(),
        ));
    }
    Ok(())
}
```

- [ ] **Step 7: Write `item.rs`**

Create `apache-datasketches/src/kll/item.rs`:

```rust
//! The item types a [`KllSketch`](super::KllSketch) can hold, and the
//! per-type dispatch to the matching cxx bridge. This is the only file that
//! knows there are two bridges; everything else is written once against
//! `T::…`.

use apache_datasketches_sys::{kll_doubles::ffi as doubles, kll_floats::ffi as floats};
use cxx::{Exception, UniquePtr};
use std::fmt::Debug;
use std::pin::Pin;

/// An item type a [`KllSketch`](super::KllSketch) can hold: `f64` or `f32`.
///
/// Sealed: it cannot be implemented outside this crate, because each
/// implementation is backed by its own explicitly instantiated C++ sketch.
/// Both types match a Java class (`KllDoublesSketch`, `KllFloatsSketch`), so
/// serialized bytes are readable by the Java and Python DataSketches
/// libraries.
pub trait KllItem: sealed::Sealed + Copy + PartialOrd + Debug + Send + Sync + 'static {}

impl KllItem for f64 {}
impl KllItem for f32 {}

pub(crate) mod sealed {
    use super::*;

    /// The per-type FFI surface. Unnameable outside the crate, which is what
    /// seals [`KllItem`](super::KllItem).
    pub trait Sealed: Sized {
        type Sketch: cxx::memory::UniquePtrTarget;

        fn is_nan_item(item: Self) -> bool;

        fn sketch_new(k: u16) -> Result<UniquePtr<Self::Sketch>, Exception>;
        fn sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<Self::Sketch>, Exception>;
        fn normalized_rank_error_for_k(k: u16, pmf: bool) -> f64;

        fn clone_shim(s: &Self::Sketch) -> UniquePtr<Self::Sketch>;
        fn update(s: Pin<&mut Self::Sketch>, item: Self);
        fn merge(s: Pin<&mut Self::Sketch>, other: &Self::Sketch);
        fn is_empty(s: &Self::Sketch) -> bool;
        fn get_n(s: &Self::Sketch) -> u64;
        fn get_k(s: &Self::Sketch) -> u16;
        fn get_num_retained(s: &Self::Sketch) -> u32;
        fn is_estimation_mode(s: &Self::Sketch) -> bool;
        fn get_min_item(s: &Self::Sketch) -> Result<Self, Exception>;
        fn get_max_item(s: &Self::Sketch) -> Result<Self, Exception>;
        fn get_quantile(s: &Self::Sketch, rank: f64, inclusive: bool) -> Result<Self, Exception>;
        fn get_rank(s: &Self::Sketch, item: Self, inclusive: bool) -> Result<f64, Exception>;
        fn get_pmf(
            s: &Self::Sketch,
            split_points: &[Self],
            inclusive: bool,
        ) -> Result<Vec<f64>, Exception>;
        fn get_cdf(
            s: &Self::Sketch,
            split_points: &[Self],
            inclusive: bool,
        ) -> Result<Vec<f64>, Exception>;
        fn get_normalized_rank_error(s: &Self::Sketch, pmf: bool) -> f64;
        fn serialize(s: &Self::Sketch) -> Vec<u8>;
        fn to_string_summary(s: &Self::Sketch) -> String;
        fn retained_items(s: &Self::Sketch) -> Vec<Self>;
        fn retained_weights(s: &Self::Sketch) -> Vec<u64>;
    }
}

macro_rules! impl_sealed {
    ($t:ty, $m:ident, $sketch:ident, $new:ident, $deser:ident, $rank_err:ident) => {
        impl sealed::Sealed for $t {
            type Sketch = $m::$sketch;

            fn is_nan_item(item: Self) -> bool {
                item.is_nan()
            }

            fn sketch_new(k: u16) -> Result<UniquePtr<Self::Sketch>, Exception> {
                $m::$new(k)
            }
            fn sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<Self::Sketch>, Exception> {
                $m::$deser(bytes)
            }
            fn normalized_rank_error_for_k(k: u16, pmf: bool) -> f64 {
                $m::$rank_err(k, pmf)
            }

            fn clone_shim(s: &Self::Sketch) -> UniquePtr<Self::Sketch> {
                s.clone_shim()
            }
            fn update(s: Pin<&mut Self::Sketch>, item: Self) {
                s.update(item)
            }
            fn merge(s: Pin<&mut Self::Sketch>, other: &Self::Sketch) {
                s.merge(other)
            }
            fn is_empty(s: &Self::Sketch) -> bool {
                s.is_empty()
            }
            fn get_n(s: &Self::Sketch) -> u64 {
                s.get_n()
            }
            fn get_k(s: &Self::Sketch) -> u16 {
                s.get_k()
            }
            fn get_num_retained(s: &Self::Sketch) -> u32 {
                s.get_num_retained()
            }
            fn is_estimation_mode(s: &Self::Sketch) -> bool {
                s.is_estimation_mode()
            }
            fn get_min_item(s: &Self::Sketch) -> Result<Self, Exception> {
                s.get_min_item()
            }
            fn get_max_item(s: &Self::Sketch) -> Result<Self, Exception> {
                s.get_max_item()
            }
            fn get_quantile(
                s: &Self::Sketch,
                rank: f64,
                inclusive: bool,
            ) -> Result<Self, Exception> {
                s.get_quantile(rank, inclusive)
            }
            fn get_rank(s: &Self::Sketch, item: Self, inclusive: bool) -> Result<f64, Exception> {
                s.get_rank(item, inclusive)
            }
            fn get_pmf(
                s: &Self::Sketch,
                split_points: &[Self],
                inclusive: bool,
            ) -> Result<Vec<f64>, Exception> {
                s.get_pmf(split_points, inclusive)
                    .map(|v| v.as_slice().to_vec())
            }
            fn get_cdf(
                s: &Self::Sketch,
                split_points: &[Self],
                inclusive: bool,
            ) -> Result<Vec<f64>, Exception> {
                s.get_cdf(split_points, inclusive)
                    .map(|v| v.as_slice().to_vec())
            }
            fn get_normalized_rank_error(s: &Self::Sketch, pmf: bool) -> f64 {
                s.get_normalized_rank_error(pmf)
            }
            fn serialize(s: &Self::Sketch) -> Vec<u8> {
                s.serialize().as_slice().to_vec()
            }
            fn to_string_summary(s: &Self::Sketch) -> String {
                s.to_string_summary()
            }
            fn retained_items(s: &Self::Sketch) -> Vec<Self> {
                s.retained_items().as_slice().to_vec()
            }
            fn retained_weights(s: &Self::Sketch) -> Vec<u64> {
                s.retained_weights().as_slice().to_vec()
            }
        }
    };
}

impl_sealed!(
    f64,
    doubles,
    KllDoublesSketchShim,
    kll_doubles_new,
    kll_doubles_deserialize,
    kll_doubles_normalized_rank_error
);
impl_sealed!(
    f32,
    floats,
    KllFloatsSketchShim,
    kll_floats_new,
    kll_floats_deserialize,
    kll_floats_normalized_rank_error
);
```

(`item.is_nan()` resolves to the inherent `f64::is_nan`/`f32::is_nan`; the trait method is deliberately named `is_nan_item` so there is no question of it recursing.)

- [ ] **Step 8: Write `sketch.rs`**

Create `apache-datasketches/src/kll/sketch.rs`:

```rust
use super::checks::{check_item, check_k, check_rank, check_split_points};
use super::KllItem;
use crate::error::SketchError;
use cxx::UniquePtr;

/// Upstream `kll_constants::DEFAULT_K`: about 1.33% normalized rank error
/// for rank/quantile/CDF queries, about 1.65% for PMF queries.
const DEFAULT_K: u16 = 200;

/// A KLL quantiles sketch over `f64` or `f32` items: answers rank, quantile,
/// PMF and CDF queries over a stream using bounded memory, with an error
/// bound set by `k` (see [`Self::get_normalized_rank_error`]).
///
/// Merging is a method ([`Self::merge`]); there is no separate union type.
///
/// # Thread safety
///
/// `KllSketch` is `Send` but **not** `Sync`. Upstream's query methods
/// (`get_quantile`, `get_rank`, `get_pmf`, `get_cdf`) are logically
/// read-only but build and cache a sorted view inside the sketch, so two
/// threads querying one sketch at once would race. To share query access
/// across threads, build a `KllSortedView` once with `sorted_view()` and
/// share that (it is `Sync`).
///
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<apache_datasketches::kll::KllSketch<f64>>();
/// ```
///
/// ```
/// fn assert_send<T: Send>() {}
/// assert_send::<apache_datasketches::kll::KllSketch<f64>>();
/// ```
///
/// # Randomness
///
/// Compaction is randomized upstream and cannot be seeded, so once more than
/// `k` items have been added, two sketches fed identical input can give
/// slightly different (equally valid) answers.
pub struct KllSketch<T: KllItem> {
    inner: UniquePtr<T::Sketch>,
}

// SAFETY: the shim owns its kll_sketch outright and holds no thread-affine
// state, so moving it to another thread is sound. It is deliberately not
// Sync: see the type-level docs.
unsafe impl<T: KllItem> Send for KllSketch<T> {}

impl<T: KllItem> KllSketch<T> {
    /// Creates an empty sketch. `k` must be in `8..=65535`; larger `k` means
    /// smaller error and more memory. [`Default`] uses `k = 200`.
    pub fn new(k: u16) -> Result<Self, SketchError> {
        check_k(k)?;
        let inner =
            T::sketch_new(k).map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Reconstructs a sketch from bytes produced by [`Self::serialize`], or by
    /// the Java/Python libraries' `KllDoublesSketch`/`KllFloatsSketch` for the
    /// matching item type.
    ///
    /// An empty sketch's bytes carry no item type, so they deserialize as
    /// either `f64` or `f32`; non-empty bytes of the other item type are
    /// rejected.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = T::sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// The normalized rank error a sketch with this `k` would have: the
    /// single-sided error for rank/quantile/CDF queries, or the double-sided
    /// error for PMF queries when `pmf` is `true`.
    pub fn normalized_rank_error_for_k(k: u16, pmf: bool) -> Result<f64, SketchError> {
        check_k(k)?;
        Ok(T::normalized_rank_error_for_k(k, pmf))
    }

    /// Adds an item. NaN is silently ignored, as upstream does.
    pub fn update(&mut self, item: T) {
        T::update(self.inner.pin_mut(), item);
    }

    /// Merges `other` into this sketch. `other` may have a different `k`;
    /// the result's error is then governed by the smaller `k`.
    pub fn merge(&mut self, other: &KllSketch<T>) {
        T::merge(self.inner.pin_mut(), &other.inner);
    }

    /// Returns `true` if no items have been added.
    pub fn is_empty(&self) -> bool {
        T::is_empty(&self.inner)
    }

    /// The number of items added (NaN updates excluded).
    pub fn get_n(&self) -> u64 {
        T::get_n(&self.inner)
    }

    /// The `k` this sketch was built with.
    pub fn get_k(&self) -> u16 {
        T::get_k(&self.inner)
    }

    /// The number of items currently retained by the sketch.
    pub fn get_num_retained(&self) -> u32 {
        T::get_num_retained(&self.inner)
    }

    /// Returns `true` once the sketch has started compacting — i.e. once
    /// query results are estimates rather than exact.
    pub fn is_estimation_mode(&self) -> bool {
        T::is_estimation_mode(&self.inner)
    }

    /// The smallest item added (exact, never estimated), or `None` if empty.
    pub fn get_min_item(&self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(T::get_min_item(&self.inner).expect("a non-empty sketch has a min item"))
    }

    /// The largest item added (exact, never estimated), or `None` if empty.
    pub fn get_max_item(&self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(T::get_max_item(&self.inner).expect("a non-empty sketch has a max item"))
    }

    /// The approximate item at normalized `rank` (in `[0, 1]`). With
    /// `inclusive`, the rank of an item includes its own weight.
    pub fn get_quantile(&self, rank: f64, inclusive: bool) -> Result<T, SketchError> {
        self.require_non_empty()?;
        check_rank(rank)?;
        Ok(T::get_quantile(&self.inner, rank, inclusive)?)
    }

    /// The approximate normalized rank (in `[0, 1]`) of `item`. With
    /// `inclusive`, the item's own weight is included. `item` must not be
    /// NaN.
    pub fn get_rank(&self, item: T, inclusive: bool) -> Result<f64, SketchError> {
        self.require_non_empty()?;
        check_item(item)?;
        Ok(T::get_rank(&self.inner, item, inclusive)?)
    }

    /// The approximate probability mass in each of the `m + 1` intervals the
    /// `m` split points divide the domain into. `split_points` must be
    /// strictly increasing and NaN-free. With `inclusive`, items equal to a
    /// split point count toward the interval on its left.
    pub fn get_pmf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError> {
        self.require_non_empty()?;
        check_split_points(split_points)?;
        Ok(T::get_pmf(&self.inner, split_points, inclusive)?)
    }

    /// The cumulative analog of [`Self::get_pmf`]: `m + 1` values, the last
    /// always `1.0`.
    pub fn get_cdf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError> {
        self.require_non_empty()?;
        check_split_points(split_points)?;
        Ok(T::get_cdf(&self.inner, split_points, inclusive)?)
    }

    /// This sketch's normalized rank error: single-sided (for rank,
    /// quantile and CDF queries), or double-sided for PMF queries when `pmf`
    /// is `true`.
    pub fn get_normalized_rank_error(&self, pmf: bool) -> f64 {
        T::get_normalized_rank_error(&self.inner, pmf)
    }

    /// Serializes this sketch. The format is shared with the Java and Python
    /// libraries.
    pub fn serialize(&self) -> Vec<u8> {
        T::serialize(&self.inner)
    }

    /// A human-readable, multi-line summary of the sketch's internal state —
    /// for debugging, not for parsing.
    pub fn to_string_summary(&self) -> String {
        T::to_string_summary(&self.inner)
    }

    /// The retained items with their weights, in the sketch's internal level
    /// order (not sorted). The weights sum to [`Self::get_n`].
    ///
    /// The items are copied out once when this is called, so the iterator
    /// does not borrow the sketch.
    pub fn iter(&self) -> impl Iterator<Item = (T, u64)> {
        let items = T::retained_items(&self.inner);
        let weights = T::retained_weights(&self.inner);
        items.into_iter().zip(weights)
    }

    fn require_non_empty(&self) -> Result<(), SketchError> {
        if self.is_empty() {
            return Err(SketchError::EmptySketch);
        }
        Ok(())
    }
}

impl<T: KllItem> Clone for KllSketch<T> {
    fn clone(&self) -> Self {
        Self {
            inner: T::clone_shim(&self.inner),
        }
    }
}

impl<T: KllItem> Default for KllSketch<T> {
    /// An empty sketch with `k = 200`.
    fn default() -> Self {
        Self::new(DEFAULT_K).expect("DEFAULT_K is in range")
    }
}
```

- [ ] **Step 9: Write `mod.rs`**

Create `apache-datasketches/src/kll/mod.rs`:

```rust
//! KLL quantiles sketch family: approximate ranks, quantiles, PMF and CDF
//! over a stream of `f64` or `f32` values, in bounded memory.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::kll::KllSketch;
//!
//! let mut latencies = KllSketch::<f64>::new(200)?;
//! for ms in 1..=1000 {
//!     latencies.update(f64::from(ms));
//! }
//! let p99 = latencies.get_quantile(0.99, true)?;
//! println!("p99 latency: {p99} ms");
//! # Ok(())
//! # }
//! ```
//!
//! - [`KllSketch`] — the sketch, generic over [`KllItem`] (`f64` or `f32`).
//!   Merge sketches with [`KllSketch::merge`].

mod checks;
mod item;
mod sketch;

pub use item::KllItem;
pub use sketch::KllSketch;
```

- [ ] **Step 10: Run the smoke test**

Run: `cargo test -p apache-datasketches --features kll --test kll_sketch_smoke_test`
Expected: PASS (24 tests: 12 bodies × 2 types).

- [ ] **Step 11: Run the doctests (including the `!Sync` compile_fail)**

Run: `cargo test -p apache-datasketches --features kll --doc kll`
Expected: PASS, including `src/kll/sketch.rs - kll::sketch::KllSketch (line …) - compile fail`.

- [ ] **Step 12: Lint**

Run: `cargo clippy -p apache-datasketches --no-default-features --features kll --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 13: Format and commit**

```bash
rustfmt --edition 2021 apache-datasketches/src/lib.rs apache-datasketches/src/kll/*.rs apache-datasketches/tests/kll_common/mod.rs apache-datasketches/tests/kll_sketch_smoke_test.rs
git add apache-datasketches
git commit -m "feat(kll): add the generic KllSketch over f64 and f32"
```

---

### Task 5: `KllSortedView<T>` and concurrency

**Files:**
- Modify: `apache-datasketches/src/kll/item.rs`
- Modify: `apache-datasketches/src/kll/sketch.rs`
- Modify: `apache-datasketches/src/kll/mod.rs`
- Create: `apache-datasketches/src/kll/sorted_view.rs`
- Modify: `apache-datasketches/Cargo.toml`
- Test: `apache-datasketches/tests/kll_sorted_view_test.rs`, `apache-datasketches/tests/kll_concurrency_test.rs`

**Interfaces:**
- Consumes: `KllSketch<T>`, `Sealed` (Task 4); view methods on the sys shims (Task 2).
- Produces: `KllSketch::sorted_view(&self) -> KllSortedView<T>`; `pub struct KllSortedView<T: KllItem>` with `get_quantile(f64, bool) -> Result<T, SketchError>`, `get_rank(T, bool) -> Result<f64, SketchError>`, `get_pmf(&[T], bool) -> Result<Vec<f64>, SketchError>`, `get_cdf(&[T], bool) -> Result<Vec<f64>, SketchError>`, `len() -> usize`, `is_empty() -> bool`, `iter() -> impl Iterator<Item = SortedViewEntry<T>>`; `pub struct SortedViewEntry<T> { pub item: T, pub weight: u64, pub cumulative_weight: u64 }` deriving `Debug, Clone, Copy, PartialEq`.

- [ ] **Step 1: Register test targets**

Append to `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "kll_sorted_view_test"
required-features = ["kll"]

[[test]]
name = "kll_concurrency_test"
required-features = ["kll"]
```

- [ ] **Step 2: Write the failing sorted-view test**

Create `apache-datasketches/tests/kll_sorted_view_test.rs`:

```rust
#[macro_use]
mod kll_common;

use apache_datasketches::kll::{KllSketch, SortedViewEntry};
use apache_datasketches::SketchError;
use kll_common::TestItem;

fn filled<T: TestItem>(n: u32) -> KllSketch<T> {
    let mut sketch = KllSketch::<T>::default();
    for i in 0..n {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

// Port of upstream kll_sketch_test.cpp SECTION("sorted view").
fn upstream_three_items<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    for v in [2.0, 3.0, 1.0] {
        sketch.update(T::from_f64(v));
    }
    let view = sketch.sorted_view();
    assert_eq!(view.len(), 3);
    let entries: Vec<SortedViewEntry<T>> = view.iter().collect();
    let expected: Vec<SortedViewEntry<T>> = (1..=3u64)
        .map(|i| SortedViewEntry {
            item: T::from_f64(i as f64),
            weight: 1,
            cumulative_weight: i,
        })
        .collect();
    assert_eq!(entries, expected);
}
for_both_types!(upstream_three_items_t, upstream_three_items);

// The view and the sketch answer from the same retained items, so they must
// agree exactly even in estimation mode.
fn view_matches_sketch<T: TestItem>() {
    let sketch = filled::<T>(10_000);
    assert!(sketch.is_estimation_mode());
    let view = sketch.sorted_view();
    for i in 0..=100 {
        let rank = f64::from(i) / 100.0;
        for inclusive in [true, false] {
            assert_eq!(
                view.get_quantile(rank, inclusive).unwrap(),
                sketch.get_quantile(rank, inclusive).unwrap()
            );
        }
    }
    for v in (0..10_000).step_by(97) {
        let item = T::from_f64(f64::from(v));
        for inclusive in [true, false] {
            assert_eq!(
                view.get_rank(item, inclusive).unwrap(),
                sketch.get_rank(item, inclusive).unwrap()
            );
        }
    }
    let split: Vec<T> = [1000.0, 5000.0, 9000.0].map(T::from_f64).to_vec();
    assert_eq!(view.get_pmf(&split, true).unwrap(), sketch.get_pmf(&split, true).unwrap());
    assert_eq!(view.get_cdf(&split, false).unwrap(), sketch.get_cdf(&split, false).unwrap());
}
for_both_types!(view_matches_sketch_t, view_matches_sketch);

fn entries_are_sorted_and_cumulative<T: TestItem>() {
    let sketch = filled::<T>(10_000);
    let entries: Vec<SortedViewEntry<T>> = sketch.sorted_view().iter().collect();
    assert_eq!(entries.len() as u32, sketch.get_num_retained());
    assert!(entries.windows(2).all(|w| w[0].item <= w[1].item));
    let mut running = 0;
    for e in &entries {
        running += e.weight;
        assert_eq!(e.cumulative_weight, running);
    }
    assert_eq!(running, sketch.get_n());
}
for_both_types!(entries_are_sorted_and_cumulative_t, entries_are_sorted_and_cumulative);

fn empty_view<T: TestItem>() {
    let view = KllSketch::<T>::default().sorted_view();
    assert_eq!(view.len(), 0);
    assert!(view.is_empty());
    assert_eq!(view.iter().count(), 0);
    assert!(matches!(view.get_quantile(0.5, true), Err(SketchError::EmptySketch)));
    assert!(matches!(view.get_rank(T::from_f64(1.0), true), Err(SketchError::EmptySketch)));
}
for_both_types!(empty_view_t, empty_view);

fn view_is_a_snapshot<T: TestItem>() {
    let mut sketch = filled::<T>(10);
    let view = sketch.sorted_view();
    sketch.update(T::from_f64(100.0));
    assert_eq!(view.len(), 10);
    assert_eq!(view.get_quantile(1.0, true).unwrap().to_f64(), 9.0);
}
for_both_types!(view_is_a_snapshot_t, view_is_a_snapshot);

fn view_validates_arguments<T: TestItem>() {
    let view = filled::<T>(10).sorted_view();
    assert!(matches!(view.get_quantile(1.5, true), Err(SketchError::InvalidConfig(_))));
    assert!(matches!(view.get_rank(T::from_f64(f64::NAN), true), Err(SketchError::InvalidConfig(_))));
    let bad = [T::from_f64(2.0), T::from_f64(1.0)];
    assert!(matches!(view.get_cdf(&bad, true), Err(SketchError::InvalidConfig(_))));
}
for_both_types!(view_validates_arguments_t, view_validates_arguments);
```

- [ ] **Step 3: Write the failing concurrency test**

Create `apache-datasketches/tests/kll_concurrency_test.rs`:

```rust
use apache_datasketches::kll::KllSketch;
use std::sync::Arc;
use std::thread;

#[test]
fn sketch_moves_across_threads() {
    let mut sketch = KllSketch::<f64>::default();
    for i in 0..1000 {
        sketch.update(f64::from(i));
    }
    let n = thread::spawn(move || {
        sketch.update(1000.0);
        sketch.get_n()
    })
    .join()
    .unwrap();
    assert_eq!(n, 1001);
}

#[test]
fn per_thread_sketches_merge() {
    let handles: Vec<_> = (0..4)
        .map(|t| {
            thread::spawn(move || {
                let mut sketch = KllSketch::<f32>::default();
                for i in 0..10_000u32 {
                    sketch.update((t * 10_000 + i) as f32);
                }
                sketch
            })
        })
        .collect();
    let mut merged = KllSketch::<f32>::default();
    for h in handles {
        merged.merge(&h.join().unwrap());
    }
    assert_eq!(merged.get_n(), 40_000);
    assert_eq!(merged.get_min_item(), Some(0.0));
    assert_eq!(merged.get_max_item(), Some(39_999.0));
}

#[test]
fn sorted_view_is_shared_across_threads() {
    let mut sketch = KllSketch::<f64>::default();
    for i in 0..100_000 {
        sketch.update(f64::from(i));
    }
    let view = Arc::new(sketch.sorted_view());
    let expected = view.get_quantile(0.5, true).unwrap();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let view = Arc::clone(&view);
            thread::spawn(move || {
                (0..1000)
                    .map(|_| view.get_quantile(0.5, true).unwrap())
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    for h in handles {
        assert!(h.join().unwrap().iter().all(|&q| q == expected));
    }
}
```

- [ ] **Step 4: Run both to verify they fail**

Run: `cargo test -p apache-datasketches --features kll --test kll_sorted_view_test --test kll_concurrency_test`
Expected: FAIL to compile — `no method named sorted_view`, `unresolved import SortedViewEntry`.

- [ ] **Step 5: Extend `Sealed` with the view surface**

In `apache-datasketches/src/kll/item.rs`:

In `trait Sealed`, after `type Sketch: cxx::memory::UniquePtrTarget;` add:

```rust
        type View: cxx::memory::UniquePtrTarget;
```

and after `fn retained_weights(s: &Self::Sketch) -> Vec<u64>;` add:

```rust

        fn sorted_view(s: &Self::Sketch) -> UniquePtr<Self::View>;
        fn view_size(v: &Self::View) -> usize;
        fn view_get_quantile(v: &Self::View, rank: f64, inclusive: bool)
            -> Result<Self, Exception>;
        fn view_get_rank(v: &Self::View, item: Self, inclusive: bool) -> Result<f64, Exception>;
        fn view_get_pmf(
            v: &Self::View,
            split_points: &[Self],
            inclusive: bool,
        ) -> Result<Vec<f64>, Exception>;
        fn view_get_cdf(
            v: &Self::View,
            split_points: &[Self],
            inclusive: bool,
        ) -> Result<Vec<f64>, Exception>;
        fn view_items(v: &Self::View) -> Vec<Self>;
        fn view_weights(v: &Self::View) -> Vec<u64>;
        fn view_cumulative_weights(v: &Self::View) -> Vec<u64>;
```

Change the macro signature from `($t:ty, $m:ident, $sketch:ident, $new:ident, $deser:ident, $rank_err:ident)` to `($t:ty, $m:ident, $sketch:ident, $view:ident, $new:ident, $deser:ident, $rank_err:ident)`; after `type Sketch = $m::$sketch;` add `type View = $m::$view;`; and after the `retained_weights` fn in the macro body add:

```rust

            fn sorted_view(s: &Self::Sketch) -> UniquePtr<Self::View> {
                s.sorted_view()
            }
            fn view_size(v: &Self::View) -> usize {
                v.size()
            }
            fn view_get_quantile(
                v: &Self::View,
                rank: f64,
                inclusive: bool,
            ) -> Result<Self, Exception> {
                v.get_quantile(rank, inclusive)
            }
            fn view_get_rank(v: &Self::View, item: Self, inclusive: bool) -> Result<f64, Exception> {
                v.get_rank(item, inclusive)
            }
            fn view_get_pmf(
                v: &Self::View,
                split_points: &[Self],
                inclusive: bool,
            ) -> Result<Vec<f64>, Exception> {
                v.get_pmf(split_points, inclusive)
                    .map(|r| r.as_slice().to_vec())
            }
            fn view_get_cdf(
                v: &Self::View,
                split_points: &[Self],
                inclusive: bool,
            ) -> Result<Vec<f64>, Exception> {
                v.get_cdf(split_points, inclusive)
                    .map(|r| r.as_slice().to_vec())
            }
            fn view_items(v: &Self::View) -> Vec<Self> {
                v.items().as_slice().to_vec()
            }
            fn view_weights(v: &Self::View) -> Vec<u64> {
                v.weights().as_slice().to_vec()
            }
            fn view_cumulative_weights(v: &Self::View) -> Vec<u64> {
                v.cumulative_weights().as_slice().to_vec()
            }
```

Update the two invocations to pass the view type after the sketch type: `KllDoublesSketchShim, KllDoublesSortedViewShim,` and `KllFloatsSketchShim, KllFloatsSortedViewShim,`.

- [ ] **Step 6: Write `sorted_view.rs`**

Create `apache-datasketches/src/kll/sorted_view.rs`:

```rust
use super::checks::{check_item, check_rank, check_split_points};
use super::KllItem;
use crate::error::SketchError;
use cxx::UniquePtr;

/// One entry of a [`KllSortedView`]: a retained item, its weight, and the
/// total weight of all entries up to and including this one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SortedViewEntry<T> {
    /// The retained item.
    pub item: T,
    /// How many input items this entry stands for.
    pub weight: u64,
    /// The sum of `weight` over this entry and every entry before it; the
    /// last entry's equals the sketch's `n`.
    pub cumulative_weight: u64,
}

/// An immutable, sorted snapshot of a [`KllSketch`](super::KllSketch)'s
/// retained items, built by [`KllSketch::sorted_view`](super::KllSketch::sorted_view).
///
/// It answers the same rank, quantile, PMF and CDF queries as the sketch,
/// with identical results, and does not change when the sketch is later
/// updated. Unlike the sketch it is `Sync`, so it is the way to share query
/// access between threads (e.g. in an `Arc`).
pub struct KllSortedView<T: KllItem> {
    inner: UniquePtr<T::View>,
}

// SAFETY: upstream quantiles_sorted_view has no mutable state — every query
// is a const binary search over entries fixed at construction — so shared
// and cross-thread access are both sound.
unsafe impl<T: KllItem> Send for KllSortedView<T> {}
unsafe impl<T: KllItem> Sync for KllSortedView<T> {}

impl<T: KllItem> KllSortedView<T> {
    pub(crate) fn from_inner(inner: UniquePtr<T::View>) -> Self {
        Self { inner }
    }

    /// The number of entries (the sketch's retained item count).
    pub fn len(&self) -> usize {
        T::view_size(&self.inner)
    }

    /// Returns `true` if the view has no entries (built from an empty sketch).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Same as [`KllSketch::get_quantile`](super::KllSketch::get_quantile).
    pub fn get_quantile(&self, rank: f64, inclusive: bool) -> Result<T, SketchError> {
        self.require_non_empty()?;
        check_rank(rank)?;
        Ok(T::view_get_quantile(&self.inner, rank, inclusive)?)
    }

    /// Same as [`KllSketch::get_rank`](super::KllSketch::get_rank).
    pub fn get_rank(&self, item: T, inclusive: bool) -> Result<f64, SketchError> {
        self.require_non_empty()?;
        check_item(item)?;
        Ok(T::view_get_rank(&self.inner, item, inclusive)?)
    }

    /// Same as [`KllSketch::get_pmf`](super::KllSketch::get_pmf).
    pub fn get_pmf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError> {
        self.require_non_empty()?;
        check_split_points(split_points)?;
        Ok(T::view_get_pmf(&self.inner, split_points, inclusive)?)
    }

    /// Same as [`KllSketch::get_cdf`](super::KllSketch::get_cdf).
    pub fn get_cdf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError> {
        self.require_non_empty()?;
        check_split_points(split_points)?;
        Ok(T::view_get_cdf(&self.inner, split_points, inclusive)?)
    }

    /// The entries in ascending item order. Copied out once when called, so
    /// the iterator does not borrow the view.
    pub fn iter(&self) -> impl Iterator<Item = SortedViewEntry<T>> {
        let items = T::view_items(&self.inner);
        let weights = T::view_weights(&self.inner);
        let cumulative = T::view_cumulative_weights(&self.inner);
        items
            .into_iter()
            .zip(weights)
            .zip(cumulative)
            .map(|((item, weight), cumulative_weight)| SortedViewEntry {
                item,
                weight,
                cumulative_weight,
            })
    }

    fn require_non_empty(&self) -> Result<(), SketchError> {
        if self.is_empty() {
            return Err(SketchError::EmptySketch);
        }
        Ok(())
    }
}
```

- [ ] **Step 7: Add `sorted_view()` to the sketch**

In `apache-datasketches/src/kll/sketch.rs`, add `use super::KllSortedView;` to the imports and, after `iter()`, add:

```rust

    /// A sorted, immutable snapshot of the retained items, answering the
    /// same queries as this sketch. Build one when running many queries
    /// against a sketch that is no longer changing, or to share query access
    /// between threads (the view is `Sync`; the sketch is not). An empty
    /// sketch gives an empty view, whose queries return
    /// [`SketchError::EmptySketch`].
    pub fn sorted_view(&self) -> KllSortedView<T> {
        KllSortedView::from_inner(T::sorted_view(&self.inner))
    }
```

- [ ] **Step 8: Export from `mod.rs`**

In `apache-datasketches/src/kll/mod.rs` add `mod sorted_view;` after `mod sketch;`, add `pub use sorted_view::{KllSortedView, SortedViewEntry};` after `pub use sketch::KllSketch;`, and extend the bullet list in the module docs with:

```rust
//! - [`KllSortedView`] / [`SortedViewEntry`] — an immutable sorted snapshot
//!   of a sketch, from [`KllSketch::sorted_view`]; `Sync`, so it is how to
//!   share query access across threads.
```

- [ ] **Step 9: Run the tests**

Run: `cargo test -p apache-datasketches --features kll --test kll_sorted_view_test --test kll_concurrency_test --test kll_sketch_smoke_test`
Expected: PASS.

- [ ] **Step 10: Lint, format, commit**

```bash
cargo clippy -p apache-datasketches --no-default-features --features kll --all-targets -- -D warnings
rustfmt --edition 2021 apache-datasketches/src/kll/*.rs apache-datasketches/tests/kll_sorted_view_test.rs apache-datasketches/tests/kll_concurrency_test.rs
git add apache-datasketches
git commit -m "feat(kll): add KllSortedView, shareable across threads"
```

---

### Task 6: Kolmogorov–Smirnov functions

**Files:**
- Modify: `apache-datasketches/src/kll/item.rs`
- Create: `apache-datasketches/src/kll/kolmogorov_smirnov.rs`
- Modify: `apache-datasketches/src/kll/mod.rs`
- Modify: `apache-datasketches/Cargo.toml`
- Test: `apache-datasketches/tests/kolmogorov_smirnov_test.rs`

**Interfaces:**
- Consumes: `KllSketch::is_empty()` (Task 4); `Normal` (Task 4 helpers).
- Produces (crate-private): `checks::check_p(p: f64) -> Result<(), SketchError>`, `KllSketch::shim(&self) -> &T::Sketch`.
- Produces: `pub fn kolmogorov_smirnov_delta<T: KllItem>(a: &KllSketch<T>, b: &KllSketch<T>) -> Result<f64, SketchError>`, `pub fn kolmogorov_smirnov_threshold<T: KllItem>(a: &KllSketch<T>, b: &KllSketch<T>, p: f64) -> Result<f64, SketchError>`, `pub fn kolmogorov_smirnov_test<T: KllItem>(a: &KllSketch<T>, b: &KllSketch<T>, p: f64) -> Result<bool, SketchError>`, re-exported from `kll`.

- [ ] **Step 1: Register the target**

Append to `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "kolmogorov_smirnov_test"
required-features = ["kll"]
```

- [ ] **Step 2: Write the failing test (port of upstream `kll/test/kolmogorov_smirnov_test.cpp`)**

Create `apache-datasketches/tests/kolmogorov_smirnov_test.rs`:

```rust
//! Port of upstream kll/test/kolmogorov_smirnov_test.cpp. Upstream draws from
//! std::default_random_engine; this uses kll_common::Normal instead. The
//! assertions are statistical, so they hold for any decent normal source.

#[macro_use]
mod kll_common;

use apache_datasketches::kll::{
    kolmogorov_smirnov_delta, kolmogorov_smirnov_test, kolmogorov_smirnov_threshold, KllSketch,
};
use apache_datasketches::SketchError;
use kll_common::Normal;

/// Two sketches of size `k` fed `3k - 1` normal samples, the first shifted by
/// `shift`. Same samples on both sides, as upstream does.
fn shifted_pair(k: u16, shift: f64) -> (KllSketch<f64>, KllSketch<f64>) {
    let mut a = KllSketch::new(k).unwrap();
    let mut b = KllSketch::new(k).unwrap();
    let mut normal = Normal::new(42);
    for _ in 0..(u32::from(k) * 3 - 1) {
        let x = normal.sample();
        a.update(x + shift);
        b.update(x);
    }
    (a, b)
}

// Upstream asserts delta == 0 and test == false for two empty sketches. Those
// answers carry no information, so this binding rejects empty operands
// instead (see the spec's Errors section).
#[test]
fn empty_operands_are_rejected() {
    let empty = KllSketch::<f64>::default();
    let mut full = KllSketch::<f64>::default();
    full.update(1.0);
    for (a, b) in [(&empty, &empty), (&empty, &full), (&full, &empty)] {
        assert!(matches!(kolmogorov_smirnov_delta(a, b), Err(SketchError::EmptySketch)));
        assert!(matches!(kolmogorov_smirnov_threshold(a, b, 0.05), Err(SketchError::EmptySketch)));
        assert!(matches!(kolmogorov_smirnov_test(a, b, 0.05), Err(SketchError::EmptySketch)));
    }
}

#[test]
fn same_distribution() {
    let (a, b) = shifted_pair(200, 0.0);
    assert!(kolmogorov_smirnov_delta(&a, &b).unwrap().abs() <= 0.02);
    assert!(!kolmogorov_smirnov_test(&a, &b, 0.01).unwrap());
}

#[test]
fn very_different_distributions() {
    let (a, b) = shifted_pair(200, 100.0);
    let delta = kolmogorov_smirnov_delta(&a, &b).unwrap();
    assert!((delta - 1.0).abs() <= 1e-6, "delta was {delta}");
    assert!(delta <= 1.0);
    assert!(kolmogorov_smirnov_test(&a, &b, 0.05).unwrap());
}

#[test]
fn slightly_different_distributions() {
    let (a, b) = shifted_pair(2000, 0.05);
    let delta = kolmogorov_smirnov_delta(&a, &b).unwrap();
    assert!((delta - 0.02).abs() <= 0.01, "delta was {delta}");
    let threshold = kolmogorov_smirnov_threshold(&a, &b, 0.05).unwrap();
    assert!(delta <= threshold);
    assert!(!kolmogorov_smirnov_test(&a, &b, 0.05).unwrap());
}

#[test]
fn slightly_different_distributions_high_resolution() {
    let (a, b) = shifted_pair(8000, 0.05);
    let delta = kolmogorov_smirnov_delta(&a, &b).unwrap();
    assert!((delta - 0.02).abs() <= 0.01, "delta was {delta}");
    let threshold = kolmogorov_smirnov_threshold(&a, &b, 0.05).unwrap();
    assert!(delta > threshold);
    assert!(kolmogorov_smirnov_test(&a, &b, 0.05).unwrap());
}

#[test]
fn p_out_of_range_is_rejected() {
    let (a, b) = shifted_pair(200, 0.0);
    for p in [0.0, 1.0, -0.5, 1.5, f64::NAN] {
        assert!(matches!(kolmogorov_smirnov_threshold(&a, &b, p), Err(SketchError::InvalidConfig(_))));
        assert!(matches!(kolmogorov_smirnov_test(&a, &b, p), Err(SketchError::InvalidConfig(_))));
    }
}

#[test]
fn works_for_f32() {
    let mut a = KllSketch::<f32>::default();
    let mut b = KllSketch::<f32>::default();
    for i in 0..1000u16 {
        a.update(f32::from(i));
        b.update(f32::from(i) + 10_000.0);
    }
    assert!(kolmogorov_smirnov_test(&a, &b, 0.05).unwrap());
}
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cargo test -p apache-datasketches --features kll --test kolmogorov_smirnov_test`
Expected: FAIL to compile — `unresolved imports kolmogorov_smirnov_delta …`.

- [ ] **Step 4: Extend `Sealed` with KS**

In `apache-datasketches/src/kll/item.rs`, in `trait Sealed` after `fn view_cumulative_weights(v: &Self::View) -> Vec<u64>;` add:

```rust

        fn ks_delta(a: &Self::Sketch, b: &Self::Sketch) -> f64;
        fn ks_threshold(a: &Self::Sketch, b: &Self::Sketch, p: f64) -> f64;
        fn ks_test(a: &Self::Sketch, b: &Self::Sketch, p: f64) -> bool;
```

and in the macro body after `view_cumulative_weights` add:

```rust

            fn ks_delta(a: &Self::Sketch, b: &Self::Sketch) -> f64 {
                a.ks_delta(b)
            }
            fn ks_threshold(a: &Self::Sketch, b: &Self::Sketch, p: f64) -> f64 {
                a.ks_threshold(b, p)
            }
            fn ks_test(a: &Self::Sketch, b: &Self::Sketch, p: f64) -> bool {
                a.ks_test(b, p)
            }
```

- [ ] **Step 5: Add `check_p` and `shim()`**

Append to `apache-datasketches/src/kll/checks.rs`:

```rust

pub(crate) fn check_p(p: f64) -> Result<(), SketchError> {
    // Written as a negated conjunction so NaN is rejected too.
    if !(p > 0.0 && p < 1.0) {
        return Err(SketchError::InvalidConfig(format!(
            "p must be in (0, 1), got {p}"
        )));
    }
    Ok(())
}
```

In `apache-datasketches/src/kll/sketch.rs`, directly above `fn require_non_empty`, add:

```rust
    /// The shim, for the sibling `kolmogorov_smirnov` module.
    pub(crate) fn shim(&self) -> &T::Sketch {
        &self.inner
    }

```

- [ ] **Step 5b: Write `kolmogorov_smirnov.rs`**

Create `apache-datasketches/src/kll/kolmogorov_smirnov.rs`:

```rust
//! The Kolmogorov–Smirnov test over two KLL sketches: do they look like
//! samples of the same distribution?
//!
//! All three functions return [`SketchError::EmptySketch`] if either sketch is
//! empty. Upstream returns `delta = 0`/`test = false` for two empty sketches,
//! `delta = 1` for one, and a NaN or infinite threshold, none of which mean
//! anything; an error is more useful than a silent "not different".

use super::checks::check_p;
use super::{KllItem, KllSketch};
use crate::error::SketchError;

/// The raw delta: the largest gap between the two sketches' approximate
/// CDFs, in `[0, 1]`.
pub fn kolmogorov_smirnov_delta<T: KllItem>(
    a: &KllSketch<T>,
    b: &KllSketch<T>,
) -> Result<f64, SketchError> {
    require_non_empty(a, b)?;
    Ok(T::ks_delta(a.shim(), b.shim()))
}

/// The threshold `delta` must exceed to reject, at p-value `p` (typically
/// 0.001 to 0.1), that the sketches share a distribution. It is widened by
/// both sketches' rank error. `p` must be in `(0, 1)`.
pub fn kolmogorov_smirnov_threshold<T: KllItem>(
    a: &KllSketch<T>,
    b: &KllSketch<T>,
    p: f64,
) -> Result<f64, SketchError> {
    require_non_empty(a, b)?;
    check_p(p)?;
    Ok(T::ks_threshold(a.shim(), b.shim(), p))
}

/// `true` if the null hypothesis — that both sketches reflect the same
/// distribution — can be rejected at p-value `p`, i.e. if
/// [`kolmogorov_smirnov_delta`] exceeds [`kolmogorov_smirnov_threshold`].
/// Sketches too small to tell return `false`.
pub fn kolmogorov_smirnov_test<T: KllItem>(
    a: &KllSketch<T>,
    b: &KllSketch<T>,
    p: f64,
) -> Result<bool, SketchError> {
    require_non_empty(a, b)?;
    check_p(p)?;
    Ok(T::ks_test(a.shim(), b.shim(), p))
}

fn require_non_empty<T: KllItem>(a: &KllSketch<T>, b: &KllSketch<T>) -> Result<(), SketchError> {
    if a.is_empty() || b.is_empty() {
        return Err(SketchError::EmptySketch);
    }
    Ok(())
}
```

- [ ] **Step 6: Export from `mod.rs`**

In `apache-datasketches/src/kll/mod.rs` add `mod kolmogorov_smirnov;` (keep the `mod` lines alphabetical: `checks`, `item`, `kolmogorov_smirnov`, `sketch`, `sorted_view`), add:

```rust
pub use kolmogorov_smirnov::{
    kolmogorov_smirnov_delta, kolmogorov_smirnov_test, kolmogorov_smirnov_threshold,
};
```

and extend the doc bullet list with:

```rust
//! - [`kolmogorov_smirnov_test`] (plus [`kolmogorov_smirnov_delta`] and
//!   [`kolmogorov_smirnov_threshold`]) — whether two sketches look like
//!   samples of the same distribution.
```

- [ ] **Step 7: Run the tests**

Run: `cargo test -p apache-datasketches --features kll --test kolmogorov_smirnov_test`
Expected: PASS. If `slightly_different_distributions*` fail on the delta margin, re-run three times: a margin failure that does not reproduce is KLL's randomized compaction, and a reproducible one means a bug — do not widen margins beyond upstream's.

- [ ] **Step 8: Lint, format, commit**

```bash
cargo clippy -p apache-datasketches --no-default-features --features kll --all-targets -- -D warnings
rustfmt --edition 2021 apache-datasketches/src/kll/*.rs apache-datasketches/tests/kolmogorov_smirnov_test.rs
git add apache-datasketches
git commit -m "feat(kll): add the Kolmogorov-Smirnov test over two sketches"
```

---

### Task 7: Port the upstream KLL tests, validation and serialization compatibility

**Files:**
- Create: `apache-datasketches/tests/kll_sketch_test.rs`
- Create: `apache-datasketches/tests/kll_validation_test.rs`
- Create: `apache-datasketches/tests/kll_serialization_compat_test.rs`
- Create: `apache-datasketches/tests/data/kll_sketch_float_one_item_v1.sk` (copied)
- Modify: `apache-datasketches/Cargo.toml`
- Modify: `docs/superpowers/specs/2026-10-21-kll-rust-bindings-design.md` (one correction)

**Interfaces:**
- Consumes: full `KllSketch`/`KllSortedView` API (Tasks 4–5); `kll_common` helpers.
- Produces: tests only.

- [ ] **Step 1: Register targets and copy the fixture**

Append to `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "kll_sketch_test"
required-features = ["kll"]

[[test]]
name = "kll_validation_test"
required-features = ["kll"]

[[test]]
name = "kll_serialization_compat_test"
required-features = ["kll"]
```

```bash
mkdir -p apache-datasketches/tests/data
cp vendor/datasketches-cpp/kll/test/kll_sketch_float_one_item_v1.sk apache-datasketches/tests/data/
```

- [ ] **Step 2: Write `kll_sketch_test.rs` (port of upstream `kll/test/kll_sketch_test.cpp`)**

Create `apache-datasketches/tests/kll_sketch_test.rs`:

```rust
//! Port of upstream kll/test/kll_sketch_test.cpp, run for both f64 and f32
//! (upstream uses float).
//!
//! Sections not ported, and why:
//! - "stream serialize deserialize *": no std::istream API in this binding.
//! - "bytes serialize deserialize many ints", "out of order split points,
//!   int", "sketch of ints", "sketch of strings *", "issue #236", "type
//!   conversion: *", "max serialized size *": item types and APIs this
//!   binding does not expose.
//! - "floor of log2 of fraction": tests an internal helper.
//! - "move": Rust moves are not observable.
//! - "sorted view": ported in kll_sorted_view_test.rs.
//! - "deserialize one item v1": ported in kll_serialization_compat_test.rs.

#[macro_use]
mod kll_common;

use apache_datasketches::kll::KllSketch;
use apache_datasketches::SketchError;
use kll_common::TestItem;

const RANK_EPS_FOR_K_200: f64 = 0.0133;
const NUMERIC_NOISE_TOLERANCE: f64 = 1e-6;

fn t<T: TestItem>(v: f64) -> T {
    T::from_f64(v)
}

fn k_limits<T: TestItem>() {
    assert!(KllSketch::<T>::new(8).is_ok());
    assert!(KllSketch::<T>::new(u16::MAX).is_ok());
    assert!(matches!(KllSketch::<T>::new(7), Err(SketchError::InvalidConfig(_))));
}
for_both_types!(k_limits_t, k_limits);

fn empty<T: TestItem>() {
    let sketch = KllSketch::<T>::default();
    assert!(sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 0);
    assert_eq!(sketch.get_num_retained(), 0);
    assert_eq!(sketch.get_min_item(), None);
    assert_eq!(sketch.get_max_item(), None);
    assert!(matches!(sketch.get_rank(t(0.0), true), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_quantile(0.5, true), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_pmf(&[t(0.0)], true), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_cdf(&[t(0.0)], true), Err(SketchError::EmptySketch)));
    assert_eq!(sketch.iter().count(), 0);
}
for_both_types!(empty_t, empty);

fn get_bad_quantile<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(0.0));
    assert!(matches!(sketch.get_quantile(-1.0, true), Err(SketchError::InvalidConfig(_))));
}
for_both_types!(get_bad_quantile_t, get_bad_quantile);

fn one_item<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(1.0));
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 1);
    assert_eq!(sketch.get_num_retained(), 1);
    assert_eq!(sketch.get_rank(t(1.0), false).unwrap(), 0.0);
    assert_eq!(sketch.get_rank(t(1.0), true).unwrap(), 1.0);
    assert_eq!(sketch.get_rank(t(2.0), false).unwrap(), 1.0);
    assert_eq!(sketch.get_rank(t(f64::INFINITY), true).unwrap(), 1.0);
    assert_eq!(sketch.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch.get_max_item(), Some(t(1.0)));
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), t(1.0));
    let pairs: Vec<(T, u64)> = sketch.iter().collect();
    assert_eq!(pairs, vec![(t(1.0), 1)]);
}
for_both_types!(one_item_t, one_item);

fn nan<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(f64::NAN));
    assert!(sketch.is_empty());
    sketch.update(t(0.0));
    sketch.update(t(f64::NAN));
    assert_eq!(sketch.get_n(), 1);
}
for_both_types!(nan_t, nan);

fn many_items_exact_mode<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    let n = 200u32;
    for i in 1..=n {
        sketch.update(t(f64::from(i)));
        assert_eq!(sketch.get_n(), u64::from(i));
    }
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_num_retained(), n);
    assert_eq!(sketch.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch.get_quantile(0.0, true).unwrap(), t(1.0));
    assert_eq!(sketch.get_max_item(), Some(t(f64::from(n))));
    assert_eq!(sketch.get_quantile(1.0, true).unwrap(), t(f64::from(n)));
    for i in 1..=n {
        let item = t(f64::from(i));
        assert_eq!(sketch.get_rank(item, true).unwrap(), f64::from(i) / f64::from(n));
        assert_eq!(sketch.get_rank(item, false).unwrap(), f64::from(i - 1) / f64::from(n));
    }
}
for_both_types!(many_items_exact_mode_t, many_items_exact_mode);

fn ten_items<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    for i in 1..=10 {
        sketch.update(t(f64::from(i)));
    }
    assert_eq!(sketch.get_quantile(0.0, true).unwrap(), t(1.0));
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), t(5.0));
    assert_eq!(sketch.get_quantile(0.99, true).unwrap(), t(10.0));
    assert_eq!(sketch.get_quantile(1.0, true).unwrap(), t(10.0));
}
for_both_types!(ten_items_t, ten_items);

fn hundred_items<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    for i in 0..100 {
        sketch.update(t(f64::from(i)));
    }
    assert_eq!(sketch.get_quantile(0.0, true).unwrap(), t(0.0));
    assert_eq!(sketch.get_quantile(0.01, true).unwrap(), t(0.0));
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), t(49.0));
    assert_eq!(sketch.get_quantile(0.99, true).unwrap(), t(98.0));
    assert_eq!(sketch.get_quantile(1.0, true).unwrap(), t(99.0));
}
for_both_types!(hundred_items_t, hundred_items);

fn many_items_estimation_mode<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    let n = 1_000_000u32;
    for i in 0..n {
        sketch.update(t(f64::from(i)));
        assert_eq!(sketch.get_n(), u64::from(i) + 1);
    }
    assert!(!sketch.is_empty());
    assert!(sketch.is_estimation_mode());
    assert_eq!(sketch.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch.get_max_item(), Some(t(f64::from(n - 1))));
    for i in 0..n {
        let true_rank = f64::from(i) / f64::from(n);
        let rank = sketch.get_rank(t(f64::from(i)), false).unwrap();
        assert!(
            (rank - true_rank).abs() <= RANK_EPS_FOR_K_200,
            "item {i}: rank {rank} vs true {true_rank}"
        );
    }
    let mut count = 0u32;
    let mut total_weight = 0u64;
    for (_, w) in sketch.iter() {
        count += 1;
        total_weight += w;
    }
    assert_eq!(count, sketch.get_num_retained());
    assert_eq!(total_weight, sketch.get_n());
}
for_both_types!(many_items_estimation_mode_t, many_items_estimation_mode);

fn consistency_between_rank_and_pmf_cdf<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    let values: Vec<T> = (0..200).map(|i| t(f64::from(i))).collect();
    for &v in &values {
        sketch.update(v);
    }
    for inclusive in [false, true] {
        let ranks = sketch.get_cdf(&values, inclusive).unwrap();
        let pmf = sketch.get_pmf(&values, inclusive).unwrap();
        let mut subtotal_pmf = 0.0;
        for (i, &v) in values.iter().enumerate() {
            assert_eq!(sketch.get_rank(v, inclusive).unwrap(), ranks[i], "rank vs CDF at {i}");
            subtotal_pmf += pmf[i];
            assert!((ranks[i] - subtotal_pmf).abs() <= NUMERIC_NOISE_TOLERANCE, "CDF vs PMF at {i}");
        }
    }
}
for_both_types!(consistency_between_rank_and_pmf_cdf_t, consistency_between_rank_and_pmf_cdf);

fn bytes_serialize_deserialize_empty<T: TestItem>() {
    let sketch = KllSketch::<T>::default();
    let sketch2 = KllSketch::<T>::deserialize(&sketch.serialize()).unwrap();
    assert_eq!(sketch2.is_empty(), sketch.is_empty());
    assert_eq!(sketch2.is_estimation_mode(), sketch.is_estimation_mode());
    assert_eq!(sketch2.get_n(), sketch.get_n());
    assert_eq!(sketch2.get_num_retained(), sketch.get_num_retained());
    assert_eq!(sketch2.get_min_item(), None);
    assert_eq!(sketch2.get_max_item(), None);
    assert_eq!(sketch2.get_normalized_rank_error(false), sketch.get_normalized_rank_error(false));
    assert_eq!(sketch2.get_normalized_rank_error(true), sketch.get_normalized_rank_error(true));
}
for_both_types!(bytes_serialize_deserialize_empty_t, bytes_serialize_deserialize_empty);

fn bytes_serialize_deserialize_one_item<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(1.0));
    let sketch2 = KllSketch::<T>::deserialize(&sketch.serialize()).unwrap();
    assert!(!sketch2.is_empty());
    assert!(!sketch2.is_estimation_mode());
    assert_eq!(sketch2.get_n(), 1);
    assert_eq!(sketch2.get_num_retained(), 1);
    assert_eq!(sketch2.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch2.get_max_item(), Some(t(1.0)));
    assert_eq!(sketch2.get_quantile(0.5, true).unwrap(), t(1.0));
    assert_eq!(sketch2.get_rank(t(1.0), false).unwrap(), 0.0);
    assert_eq!(sketch2.get_rank(t(2.0), false).unwrap(), 1.0);
}
for_both_types!(bytes_serialize_deserialize_one_item_t, bytes_serialize_deserialize_one_item);

fn bytes_serialize_deserialize_three_items<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    for v in [1.0, 2.0, 3.0] {
        sketch.update(t(v));
    }
    let sketch2 = KllSketch::<T>::deserialize(&sketch.serialize()).unwrap();
    assert!(!sketch2.is_empty());
    assert!(!sketch2.is_estimation_mode());
    assert_eq!(sketch2.get_n(), 3);
    assert_eq!(sketch2.get_num_retained(), 3);
    assert_eq!(sketch2.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch2.get_max_item(), Some(t(3.0)));
}
for_both_types!(bytes_serialize_deserialize_three_items_t, bytes_serialize_deserialize_three_items);

fn bytes_serialize_deserialize_many<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    let n = 1000;
    for i in 0..n {
        sketch.update(t(f64::from(i)));
    }
    let bytes = sketch.serialize();
    let sketch2 = KllSketch::<T>::deserialize(&bytes).unwrap();
    assert_eq!(sketch2.is_empty(), sketch.is_empty());
    assert_eq!(sketch2.is_estimation_mode(), sketch.is_estimation_mode());
    assert_eq!(sketch2.get_n(), sketch.get_n());
    assert_eq!(sketch2.get_num_retained(), sketch.get_num_retained());
    assert_eq!(sketch2.get_min_item(), sketch.get_min_item());
    assert_eq!(sketch2.get_max_item(), sketch.get_max_item());
    assert_eq!(sketch2.get_normalized_rank_error(false), sketch.get_normalized_rank_error(false));
    assert_eq!(sketch2.get_normalized_rank_error(true), sketch.get_normalized_rank_error(true));
    assert_eq!(sketch2.get_quantile(0.5, true).unwrap(), sketch.get_quantile(0.5, true).unwrap());
    assert_eq!(sketch2.get_rank(t(0.0), true).unwrap(), sketch.get_rank(t(0.0), true).unwrap());
    let top = t(f64::from(n));
    assert_eq!(sketch2.get_rank(top, true).unwrap(), sketch.get_rank(top, true).unwrap());
    for len in [7, 15, bytes.len() - 1] {
        assert!(
            matches!(KllSketch::<T>::deserialize(&bytes[..len]), Err(SketchError::Deserialization(_))),
            "truncated to {len} bytes"
        );
    }
}
for_both_types!(bytes_serialize_deserialize_many_t, bytes_serialize_deserialize_many);

fn out_of_order_split_points<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(0.0));
    assert!(matches!(sketch.get_cdf(&[t(1.0), t(0.0)], true), Err(SketchError::InvalidConfig(_))));
}
for_both_types!(out_of_order_split_points_t, out_of_order_split_points);

fn nan_split_point<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(0.0));
    assert!(matches!(sketch.get_cdf(&[t(f64::NAN)], true), Err(SketchError::InvalidConfig(_))));
}
for_both_types!(nan_split_point_t, nan_split_point);

/// Two sketches over `0..n` and `n..2n`, the second fed in descending order.
fn two_halves<T: TestItem>(k1: u16, k2: u16, n: u32) -> (KllSketch<T>, KllSketch<T>) {
    let mut sketch1 = KllSketch::<T>::new(k1).unwrap();
    let mut sketch2 = KllSketch::<T>::new(k2).unwrap();
    for i in 0..n {
        sketch1.update(t(f64::from(i)));
        sketch2.update(t(f64::from(2 * n - i - 1)));
    }
    (sketch1, sketch2)
}

fn merge<T: TestItem>() {
    let n = 10_000u32;
    let (mut sketch1, sketch2) = two_halves::<T>(200, 200, n);
    assert_eq!(sketch1.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch1.get_max_item(), Some(t(f64::from(n - 1))));
    assert_eq!(sketch2.get_min_item(), Some(t(f64::from(n))));
    assert_eq!(sketch2.get_max_item(), Some(t(f64::from(2 * n - 1))));
    sketch1.merge(&sketch2);
    assert!(!sketch1.is_empty());
    assert_eq!(sketch1.get_n(), u64::from(2 * n));
    assert_eq!(sketch1.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch1.get_max_item(), Some(t(f64::from(2 * n - 1))));
    let median = sketch1.get_quantile(0.5, true).unwrap().to_f64();
    assert!((median - f64::from(n)).abs() <= f64::from(n) * RANK_EPS_FOR_K_200);
}
for_both_types!(merge_t, merge);

fn merge_lower_k<T: TestItem>() {
    let n = 10_000u32;
    let (mut sketch1, sketch2) = two_halves::<T>(256, 128, n);
    assert_eq!(sketch1.get_k(), 256);
    assert_eq!(sketch2.get_k(), 128);
    assert!(sketch1.get_normalized_rank_error(false) < sketch2.get_normalized_rank_error(false));
    assert!(sketch1.get_normalized_rank_error(true) < sketch2.get_normalized_rank_error(true));
    sketch1.merge(&sketch2);
    // sketch1 is "contaminated" by sketch2's lower k.
    assert_eq!(sketch2.get_normalized_rank_error(false), sketch1.get_normalized_rank_error(false));
    assert_eq!(sketch2.get_normalized_rank_error(true), sketch1.get_normalized_rank_error(true));
    assert_eq!(sketch1.get_n(), u64::from(2 * n));
    assert_eq!(sketch1.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch1.get_max_item(), Some(t(f64::from(2 * n - 1))));
    let median = sketch1.get_quantile(0.5, true).unwrap().to_f64();
    assert!((median - f64::from(n)).abs() <= f64::from(n) * RANK_EPS_FOR_K_200);
}
for_both_types!(merge_lower_k_t, merge_lower_k);

fn merge_exact_mode_lower_k<T: TestItem>() {
    let n = 10_000u32;
    let mut sketch1 = KllSketch::<T>::new(256).unwrap();
    let mut sketch2 = KllSketch::<T>::new(128).unwrap();
    for i in 0..n {
        sketch1.update(t(f64::from(i)));
    }
    // Merging an empty sketch with lower k must not change the rank error.
    let before = sketch1.get_normalized_rank_error(true);
    sketch1.merge(&sketch2);
    assert_eq!(sketch1.get_normalized_rank_error(true), before);
    assert_eq!(sketch1.get_n(), u64::from(n));
    assert_eq!(sketch1.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch1.get_max_item(), Some(t(f64::from(n - 1))));
    let median = sketch1.get_quantile(0.5, true).unwrap().to_f64();
    let half = f64::from(n / 2);
    assert!((median - half).abs() <= half * RANK_EPS_FOR_K_200);
    // Nor must merging an exact-mode sketch with lower k.
    sketch2.update(t(0.0));
    sketch1.merge(&sketch2);
    assert_eq!(sketch1.get_normalized_rank_error(true), before);
}
for_both_types!(merge_exact_mode_lower_k_t, merge_exact_mode_lower_k);

fn merge_min_value_from_other<T: TestItem>() {
    let mut sketch1 = KllSketch::<T>::default();
    let mut sketch2 = KllSketch::<T>::default();
    sketch1.update(t(1.0));
    sketch2.update(t(2.0));
    sketch2.merge(&sketch1);
    assert_eq!(sketch2.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch2.get_max_item(), Some(t(2.0)));
}
for_both_types!(merge_min_value_from_other_t, merge_min_value_from_other);

fn merge_min_and_max_values_from_other<T: TestItem>() {
    let mut sketch1 = KllSketch::<T>::default();
    for i in 0..1_000_000 {
        sketch1.update(t(f64::from(i)));
    }
    let mut sketch2 = KllSketch::<T>::default();
    sketch2.merge(&sketch1);
    assert_eq!(sketch2.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch2.get_max_item(), Some(t(999_999.0)));
}
for_both_types!(merge_min_and_max_values_from_other_t, merge_min_and_max_values_from_other);

fn copy<T: TestItem>() {
    let mut sketch1 = KllSketch::<T>::default();
    for i in 0..1000 {
        sketch1.update(t(f64::from(i)));
    }
    let sketch2 = sketch1.clone();
    for i in 0..1000 {
        let item = t(f64::from(i));
        assert_eq!(sketch2.get_rank(item, true).unwrap(), sketch1.get_rank(item, true).unwrap());
    }
}
for_both_types!(copy_t, copy);
```

- [ ] **Step 3: Write `kll_validation_test.rs`**

Upstream `kll_sketch_validation.cpp` cannot be ported: it only compiles under `-DKLL_VALIDATION`, which swaps upstream's random compaction bit for a deterministic one, and that hook does not exist in the headers this crate builds. This file instead pins every row of the spec's error table.

Create `apache-datasketches/tests/kll_validation_test.rs`:

```rust
//! Every row of the spec's error table, for both item types.
//!
//! Not a port of upstream kll_sketch_validation.cpp: that test only builds
//! with -DKLL_VALIDATION, which replaces the random compaction bit with a
//! deterministic one, and the binding builds without it.

#[macro_use]
mod kll_common;

use apache_datasketches::kll::KllSketch;
use apache_datasketches::SketchError;
use kll_common::TestItem;

fn invalid(r: Result<impl std::fmt::Debug, SketchError>) -> bool {
    matches!(r, Err(SketchError::InvalidConfig(_)))
}

fn filled<T: TestItem>() -> KllSketch<T> {
    let mut sketch = KllSketch::<T>::default();
    for i in 0..10 {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

fn k_out_of_range<T: TestItem>() {
    for k in [0, 1, 7] {
        assert!(invalid(KllSketch::<T>::new(k).map(|_| ())));
        assert!(invalid(KllSketch::<T>::normalized_rank_error_for_k(k, true)));
    }
}
for_both_types!(k_out_of_range_t, k_out_of_range);

fn rank_out_of_range<T: TestItem>() {
    let sketch = filled::<T>();
    for rank in [-0.0001, 1.0001, f64::NAN, f64::INFINITY] {
        assert!(invalid(sketch.get_quantile(rank, true)), "rank {rank}");
        assert!(invalid(sketch.sorted_view().get_quantile(rank, false)), "rank {rank}");
    }
    // The endpoints are valid.
    assert!(sketch.get_quantile(0.0, true).is_ok());
    assert!(sketch.get_quantile(1.0, true).is_ok());
}
for_both_types!(rank_out_of_range_t, rank_out_of_range);

fn nan_item_in_get_rank<T: TestItem>() {
    let sketch = filled::<T>();
    assert!(invalid(sketch.get_rank(T::from_f64(f64::NAN), true)));
}
for_both_types!(nan_item_in_get_rank_t, nan_item_in_get_rank);

fn bad_split_points<T: TestItem>() {
    let sketch = filled::<T>();
    let x = T::from_f64;
    let cases: [Vec<T>; 4] = [
        vec![x(f64::NAN)],
        vec![x(1.0), x(f64::NAN)],
        vec![x(2.0), x(1.0)],
        vec![x(1.0), x(1.0)],
    ];
    for split in &cases {
        assert!(invalid(sketch.get_pmf(split, true)), "{split:?}");
        assert!(invalid(sketch.get_cdf(split, true)), "{split:?}");
    }
    // No split points at all is valid: one interval holding everything.
    assert_eq!(sketch.get_cdf(&[], true).unwrap(), vec![1.0]);
}
for_both_types!(bad_split_points_t, bad_split_points);

fn empty_wins_over_bad_arguments<T: TestItem>() {
    // Upstream checks emptiness first; so does the binding.
    let sketch = KllSketch::<T>::default();
    assert!(matches!(sketch.get_quantile(2.0, true), Err(SketchError::EmptySketch)));
}
for_both_types!(empty_wins_over_bad_arguments_t, empty_wins_over_bad_arguments);
```

- [ ] **Step 4: Write `kll_serialization_compat_test.rs`**

Expected bytes below are derived by hand from upstream `kll_sketch<T>::serialize` (`kll_sketch_impl.hpp`, the `vector_bytes serialize(unsigned header_size_bytes, ...)` overload): preamble ints, serial version (1, or 2 for a single item), family 15, flags (bit 0 empty, bit 1 level-zero-sorted, bit 2 single item), `k` little-endian, `m = 8`, one unused byte, then items. If a golden assertion fails, re-derive from that source — never paste the binding's own output in as the expectation.

Create `apache-datasketches/tests/kll_serialization_compat_test.rs`:

```rust
//! Wire-format compatibility. The C++ and Java libraries share the KLL
//! format, so bytes that match upstream C++'s layout are readable by Java and
//! Python too.

use apache_datasketches::kll::KllSketch;
use apache_datasketches::SketchError;

const FIXTURE_FLOAT_ONE_ITEM_V1: &[u8] = include_bytes!("data/kll_sketch_float_one_item_v1.sk");

/// preamble_ints=2, serial_version=1, family=15, flags=EMPTY, k=200 (LE),
/// m=8, unused.
const EMPTY_K200: [u8; 8] = [2, 1, 15, 1, 200, 0, 8, 0];

// Port of upstream SECTION("deserialize one item v1").
#[test]
fn upstream_fixture_float_one_item_v1() {
    let sketch = KllSketch::<f32>::deserialize(FIXTURE_FLOAT_ONE_ITEM_V1).unwrap();
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 1);
    assert_eq!(sketch.get_num_retained(), 1);
    assert_eq!(sketch.get_min_item(), Some(1.0));
    assert_eq!(sketch.get_max_item(), Some(1.0));
}

#[test]
fn golden_empty() {
    assert_eq!(KllSketch::<f64>::default().serialize(), EMPTY_K200);
    assert_eq!(KllSketch::<f32>::default().serialize(), EMPTY_K200);
}

#[test]
fn golden_single_item() {
    // serial_version=2, flags=SINGLE_ITEM, then just the item.
    let header = [2u8, 2, 15, 4, 200, 0, 8, 0];

    let mut d = KllSketch::<f64>::default();
    d.update(1.0);
    let expected: Vec<u8> = header.iter().copied().chain(1.0f64.to_le_bytes()).collect();
    assert_eq!(d.serialize(), expected);

    let mut f = KllSketch::<f32>::default();
    f.update(1.0);
    let expected: Vec<u8> = header.iter().copied().chain(1.0f32.to_le_bytes()).collect();
    assert_eq!(f.serialize(), expected);
}

// The empty preamble carries no item type, so this is correct, not a leak.
#[test]
fn empty_bytes_deserialize_as_either_type() {
    assert!(KllSketch::<f64>::deserialize(&EMPTY_K200).unwrap().is_empty());
    assert!(KllSketch::<f32>::deserialize(&EMPTY_K200).unwrap().is_empty());
}

fn f32_bytes(n: u32) -> Vec<u8> {
    let mut s = KllSketch::<f32>::default();
    for i in 0..n {
        s.update(i as f32);
    }
    s.serialize()
}

fn f64_bytes(n: u32) -> Vec<u8> {
    let mut s = KllSketch::<f64>::default();
    for i in 0..n {
        s.update(f64::from(i));
    }
    s.serialize()
}

#[test]
fn non_empty_bytes_of_the_other_type_are_rejected() {
    // 1 = single item, 3 = n <= k, 10_000 = estimation mode.
    for n in [1, 3, 10_000] {
        assert!(
            matches!(KllSketch::<f64>::deserialize(&f32_bytes(n)), Err(SketchError::Deserialization(_))),
            "f32 bytes with n={n} accepted as f64"
        );
        assert!(
            matches!(KllSketch::<f32>::deserialize(&f64_bytes(n)), Err(SketchError::Deserialization(_))),
            "f64 bytes with n={n} accepted as f32"
        );
    }
    assert!(matches!(
        KllSketch::<f64>::deserialize(FIXTURE_FLOAT_ONE_ITEM_V1),
        Err(SketchError::Deserialization(_))
    ));
}

#[test]
fn deterministic_sketch_bytes_are_stable() {
    // n <= k never compacts, so the bytes cannot depend on the random bit.
    assert_eq!(f64_bytes(150), f64_bytes(150));
    assert_eq!(f32_bytes(150), f32_bytes(150));
}
```

- [ ] **Step 5: Run the three files**

Run: `cargo test -p apache-datasketches --features kll --release --test kll_sketch_test --test kll_validation_test --test kll_serialization_compat_test`
Expected: PASS. (`--release` only for speed; the 1M-item sections are slow in debug. Also run once without `--release` to confirm debug passes, just slower.)

If `non_empty_bytes_of_the_other_type_are_rejected` fails for some `n`, upstream's size checks let that case through: add an explicit check in the shim's `kll_doubles_deserialize`/`kll_floats_deserialize` (after upstream's deserialize, re-serialize the result and throw `std::invalid_argument` if `serialize().size() != bytes.size()`), and note it in the CHANGELOG. If `golden_single_item` fails only on the flags byte (4 vs 6), re-read `update()` in `kll_sketch_impl.hpp` to see whether a single update leaves level zero flagged sorted, and correct the expected header from the source.

- [ ] **Step 6: Correct the spec's validation-test line**

In `docs/superpowers/specs/2026-10-21-kll-rust-bindings-design.md`, replace the bullet beginning `- **`kll_validation_test.rs`** — port of `kll_sketch_validation.cpp`, plus` with:

```markdown
- **`kll_validation_test.rs`** — every row of the error table above. Not a
  port of upstream `kll_sketch_validation.cpp`: that test only compiles under
  `-DKLL_VALIDATION`, which replaces the random compaction bit with a
  deterministic one, and the binding builds without it.
```

- [ ] **Step 7: Format and commit**

```bash
rustfmt --edition 2021 apache-datasketches/tests/kll_sketch_test.rs apache-datasketches/tests/kll_validation_test.rs apache-datasketches/tests/kll_serialization_compat_test.rs
git add apache-datasketches docs/superpowers/specs/2026-10-21-kll-rust-bindings-design.md
git commit -m "test(kll): port the upstream KLL tests and pin the wire format"
```

---

### Task 8: Benchmark pair

**Files:**
- Modify: `benches/cpp_reference/bench_common.h`
- Create: `benches/cpp_reference/kll_update.cc`
- Create: `apache-datasketches/examples/bench_kll_update.rs`
- Modify: `apache-datasketches/Cargo.toml`
- Modify: `benches/cpp_reference/run.sh`, `benches/cpp_reference/compare.sh`, `benches/cpp_reference/README.md`
- Modify: `.github/workflows/ci.yml` (bench job comment)

**Interfaces:**
- Consumes: `KllSketch<f64>` API.
- Produces: `bench::report_fields(label, items, ns_per_op, fields)` in `bench_common.h`; output lines with labels `n=`, `min=`, `max=`, `retained=` (and `bytes=` on `ser`/`deser`), never `estimate=`.

- [ ] **Step 1: Add `report_fields` to `bench_common.h`**

In `benches/cpp_reference/bench_common.h`, before `} // namespace bench` add:

```cpp
// As report, but for a family whose estimates are randomized (KLL draws a
// random bit at every compaction, from an engine seeded off the clock), so
// two correct runs disagree and an estimate cannot be the cross-check. The
// caller passes the deterministic fields instead -- n, min, max, retained --
// pre-formatted as " name=value" pairs, one string per rep. Level sizes depend
// only on n and k, never on the random bit, so these must match exactly
// across reps and across languages.
inline void report_fields(const char* label, uint64_t items, std::vector<double> ns_per_op,
                          const std::vector<std::string>& fields) {
  for (size_t i = 1; i < fields.size(); ++i) {
    if (fields[i] != fields[0]) {
      fprintf(stderr,
              "error: %s rep %zu reported '%s' but rep 0 reported '%s'; the reps are not "
              "running the same workload\n",
              label, i, fields[i].c_str(), fields[0].c_str());
      exit(1);
    }
  }
  std::sort(ns_per_op.begin(), ns_per_op.end());
  const double median = ns_per_op[(ns_per_op.size() - 1) / 2];
  printf("%-9s %12llu items  %7.2f ns/op  min %7.2f  max %7.2f  %8.1f M/s  reps=%zu%s\n",
         label, static_cast<unsigned long long>(items), median, ns_per_op.front(),
         ns_per_op.back(), 1000.0 / median, ns_per_op.size(), fields[0].c_str());
}
```

- [ ] **Step 2: Write the C++ harness**

Create `benches/cpp_reference/kll_update.cc`:

```cpp
// Native C++ counterpart to apache-datasketches/examples/bench_kll_update.rs.
//
// Establishes the floor: what the vendored datasketches-cpp costs for this
// workload with no Rust, no FFI, no shim.
//
// Unlike the other pairs, the cross-check is not an estimate: KLL compaction
// is randomized, so quantiles differ run to run. Both sides print n, min, max
// and retained (and bytes for ser/deser) instead, which depend only on the
// input and k, and compare.sh requires those to match. Build and run via
// run.sh.

#include "kll_sketch.hpp"

#include "bench_common.h"

namespace {

constexpr uint16_t K = 200;
constexpr uint64_t HOT_KEY_SPACE = 1 << 10;

// get_quantile cost tracks the retained-item count, not the item count, so
// like ser/deser this is a fixed call count. The first call builds the
// sketch's cached sorted view; it happens before the timed loop.
constexpr uint64_t QUERY_CALLS = 1000000;

using sketch_t = datasketches::kll_sketch<double>;

sketch_t build() { return sketch_t(K); }

std::string fields(const sketch_t& s) {
  char buf[160];
  snprintf(buf, sizeof buf, " n=%llu min=%.0f max=%.0f retained=%u",
           static_cast<unsigned long long>(s.get_n()), s.get_min_item(), s.get_max_item(),
           s.get_num_retained());
  return buf;
}

std::string fields_bytes(const sketch_t& s, size_t bytes) {
  char buf[32];
  snprintf(buf, sizeof buf, " bytes=%zu", bytes);
  return fields(s) + buf;
}

void bench_distinct(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) sketch.update(static_cast<double>(i));
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    per_rep.push_back(fields(sketch));
  }
  bench::report_fields("distinct", items, ns_per_op, per_rep);
}

void bench_hot(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) sketch.update(static_cast<double>(i % HOT_KEY_SPACE));
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    per_rep.push_back(fields(sketch));
  }
  bench::report_fields("hot", items, ns_per_op, per_rep);
}

void bench_quantile(uint64_t items, uint64_t reps) {
  auto sketch = build();
  for (uint64_t i = 0; i < items; ++i) sketch.update(static_cast<double>(i));
  bench::keep(sketch.get_quantile(0.5));  // builds the cached sorted view
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < QUERY_CALLS; ++i) {
      total += sketch.get_quantile(static_cast<double>(i % 1000) / 1000.0);
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(QUERY_CALLS));
    bench::keep(total);
    per_rep.push_back(fields(sketch));
  }
  bench::report_fields("quantile", items, ns_per_op, per_rep);
}

void bench_serde(uint64_t items, uint64_t reps) {
  auto sketch = build();
  for (uint64_t i = 0; i < items; ++i) sketch.update(static_cast<double>(i));
  const auto reference = sketch.serialize();

  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    uint64_t total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::SER_CALLS; ++i) total += sketch.serialize().size();
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::SER_CALLS));
    bench::keep(static_cast<double>(total));
    per_rep.push_back(fields_bytes(sketch, reference.size()));
  }
  bench::report_fields("ser", items, ns_per_op, per_rep);

  ns_per_op.clear();
  per_rep.clear();
  const auto deserialize = [&reference] {
    return sketch_t::deserialize(reference.data(), reference.size());
  };
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::DESER_CALLS; ++i) {
      total += static_cast<double>(deserialize().get_num_retained());
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::DESER_CALLS));
    bench::keep(total);
    per_rep.push_back(fields_bytes(deserialize(), reference.size()));
  }
  bench::report_fields("deser", items, ns_per_op, per_rep);
}

} // namespace

int main(int argc, char** argv) {
  const bench::Config cfg = bench::parse_args(argc, argv);
  for (const uint64_t items : cfg.counts) {
    printf("k=%u items=%llu reps=%llu\n", K, static_cast<unsigned long long>(items),
           static_cast<unsigned long long>(cfg.reps));
    bench_distinct(items, cfg.reps);
    bench_hot(items, cfg.reps);
    bench_quantile(items, cfg.reps);
    bench_serde(items, cfg.reps);
  }
  return 0;
}
```

- [ ] **Step 3: Write the Rust harness**

Create `apache-datasketches/examples/bench_kll_update.rs`:

```rust
//! Throughput harness for `KllSketch<f64>`: update, query and serialization.
//!
//! Run with (release matters -- a debug build measures nothing useful):
//!   cargo run --release --example bench_kll_update --features kll
//!   cargo run --release --example bench_kll_update --features kll -- --ladder
//!
//! Accepts `[ITEMS] [--reps N] [--ladder]`, like every other harness. Pair
//! with `benches/cpp_reference/kll_update.cc` via `compare.sh`.
//!
//! Fixed parameters so runs are comparable: `k = 200`. Keep them in sync with
//! the C++ counterpart.
//!
//! KLL compaction is randomized, so unlike the other harnesses this one
//! cannot cross-check an estimate. It prints `n`, `min`, `max` and
//! `retained` (plus `bytes` for ser/deser) instead: they depend only on the
//! input and `k`, so they must match across reps and across languages.

use apache_datasketches::kll::KllSketch;
use std::hint::black_box;
use std::time::{Duration, Instant};

const K: u16 = 200;
const HOT_KEY_SPACE: u64 = 1 << 10;

/// Keep in step with `bench_common.h`; see the comments there.
const SER_CALLS: u64 = 20_000;
const DESER_CALLS: u64 = 5_000;

/// `get_quantile` cost tracks the retained-item count, not the item count,
/// so this is a fixed call count. Keep in step with `kll_update.cc`.
const QUERY_CALLS: u64 = 1_000_000;

const LADDER: [u64; 3] = [1_000_000, 10_000_000, 100_000_000];
const DEFAULT_ITEMS: u64 = 10_000_000;
const DEFAULT_REPS: usize = 3;

/// Parses `[ITEMS] [--reps N] [--ladder]`. Same rules as the other harnesses.
fn parse_args() -> (Vec<u64>, usize) {
    let mut items = None;
    let mut reps = DEFAULT_REPS;
    let mut ladder = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ladder" => ladder = true,
            "--reps" => {
                reps = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|&n| n > 0)
                    .expect("--reps needs a positive integer")
            }
            other => {
                let n = other
                    .parse()
                    .expect("item count must be a positive integer");
                assert!(n > 0, "item count must be a positive integer");
                items = Some(n);
            }
        }
    }
    assert!(
        !(ladder && items.is_some()),
        "pass an item count or --ladder, not both"
    );
    let counts = if ladder {
        LADDER.to_vec()
    } else {
        vec![items.unwrap_or(DEFAULT_ITEMS)]
    };
    (counts, reps)
}

/// One timed pass, and the deterministic fields the sketch reported after it.
struct Pass {
    elapsed: Duration,
    fields: String,
}

fn fields(sketch: &KllSketch<f64>) -> String {
    format!(
        " n={} min={:.0} max={:.0} retained={}",
        sketch.get_n(),
        sketch.get_min_item().expect("benched sketches are non-empty"),
        sketch.get_max_item().expect("benched sketches are non-empty"),
        sketch.get_num_retained()
    )
}

/// Lower median plus spread, as every harness prints; the per-rep fields are
/// asserted equal. Keep the format identical to `bench::report_fields`.
fn report(label: &str, items: u64, ops: u64, passes: &[Pass]) {
    for (i, pass) in passes.iter().enumerate() {
        assert_eq!(
            pass.fields, passes[0].fields,
            "rep {i} reported '{}' but rep 0 reported '{}': the reps are not running \
             the same workload",
            pass.fields, passes[0].fields
        );
    }
    let mut ns_per_op: Vec<f64> = passes
        .iter()
        .map(|p| p.elapsed.as_secs_f64() * 1e9 / ops as f64)
        .collect();
    ns_per_op.sort_by(f64::total_cmp);
    let median = ns_per_op[(ns_per_op.len() - 1) / 2];
    let (min, max) = (ns_per_op[0], ns_per_op[ns_per_op.len() - 1]);
    let rate = 1000.0 / median;
    let (reps, fields) = (passes.len(), &passes[0].fields);
    println!(
        "{label:9} {items:>12} items  {median:>7.2} ns/op  min {min:>7.2}  max {max:>7.2}  \
         {rate:>8.1} M/s  reps={reps}{fields}"
    );
}

fn build() -> KllSketch<f64> {
    KllSketch::new(K).expect("fixed valid k was rejected")
}

fn filled(items: u64) -> KllSketch<f64> {
    let mut sketch = build();
    for i in 0..items {
        sketch.update(i as f64);
    }
    sketch
}

fn bench_distinct(items: u64, reps: usize) {
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let mut sketch = build();
            let start = Instant::now();
            for i in 0..items {
                sketch.update(i as f64);
            }
            let elapsed = start.elapsed();
            Pass {
                elapsed,
                fields: fields(&sketch),
            }
        })
        .collect();
    report("distinct", items, items, &passes);
}

fn bench_hot(items: u64, reps: usize) {
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let mut sketch = build();
            let start = Instant::now();
            for i in 0..items {
                sketch.update((i % HOT_KEY_SPACE) as f64);
            }
            let elapsed = start.elapsed();
            Pass {
                elapsed,
                fields: fields(&sketch),
            }
        })
        .collect();
    report("hot", items, items, &passes);
}

fn bench_quantile(items: u64, reps: usize) {
    let sketch = filled(items);
    // Builds the sketch's cached sorted view outside the timed loop.
    black_box(sketch.get_quantile(0.5, true).unwrap());
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut total = 0.0;
            for i in 0..QUERY_CALLS {
                total += sketch
                    .get_quantile((i % 1000) as f64 / 1000.0, true)
                    .unwrap();
            }
            let elapsed = start.elapsed();
            black_box(total);
            Pass {
                elapsed,
                fields: fields(&sketch),
            }
        })
        .collect();
    report("quantile", items, QUERY_CALLS, &passes);
}

fn bench_serde(items: u64, reps: usize) {
    let sketch = filled(items);
    let reference = sketch.serialize();
    let with_bytes = |s: &KllSketch<f64>| format!("{} bytes={}", fields(s), reference.len());

    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut total = 0usize;
            for _ in 0..SER_CALLS {
                total += black_box(sketch.serialize()).len();
            }
            let elapsed = start.elapsed();
            black_box(total);
            Pass {
                elapsed,
                fields: with_bytes(&sketch),
            }
        })
        .collect();
    report("ser", items, SER_CALLS, &passes);

    let deserialize =
        || KllSketch::<f64>::deserialize(&reference).expect("the bytes came from serialize");
    let passes: Vec<Pass> = (0..reps)
        .map(|_| {
            let start = Instant::now();
            let mut total = 0u64;
            for _ in 0..DESER_CALLS {
                total += u64::from(deserialize().get_num_retained());
            }
            let elapsed = start.elapsed();
            black_box(total);
            Pass {
                elapsed,
                fields: with_bytes(&deserialize()),
            }
        })
        .collect();
    report("deser", items, DESER_CALLS, &passes);
}

fn main() {
    let (counts, reps) = parse_args();
    for items in counts {
        println!("k={K} items={items} reps={reps}");
        bench_distinct(items, reps);
        bench_hot(items, reps);
        bench_quantile(items, reps);
        bench_serde(items, reps);
    }
}
```

- [ ] **Step 4: Register the example**

In `apache-datasketches/Cargo.toml`, after the `bench_cpc_update` `[[example]]` add:

```toml
[[example]]
name = "bench_kll_update"
required-features = ["kll"]
```

- [ ] **Step 5: Register the pair in the scripts**

In `benches/cpp_reference/run.sh`: add `-I"$vendor/kll/include"` after `-I"$vendor/cpc/include"` in `includes`, and change `benchmarks=(hll hll_union theta cpc array_of_doubles)` to `benchmarks=(hll hll_union theta cpc kll array_of_doubles)`.

In `benches/cpp_reference/compare.sh`: add `"kll:bench_kll_update:kll"` to `pairs` after the `cpc` entry, and extend the comment above `pairs`:

```bash
# bench_kll_update cross-checks n/min/max/retained instead of an estimate:
# KLL compaction is randomized, so its estimates legitimately differ run to run.
```

- [ ] **Step 6: Run the comparison at a smoke size**

Run: `./benches/cpp_reference/compare.sh 100000 --reps 2`
Expected: exit 0; the table includes five `kll` rows (`distinct`, `hot`, `quantile`, `ser`, `deser`); the last line before the note reads `Estimates agree on every scenario: both sides ran the same workload.`

If a `kll` row fails on `min=`/`max=`/`n=`/`retained=`, a parameter differs between the two files — fix the drift, do not loosen `compare.py`.

- [ ] **Step 7: Document the pair**

In `benches/cpp_reference/README.md`, in the `## What is here` section add a bullet for `kll_update.cc` matching the style of the existing ones, and add a new section before `## Reading the numbers`:

```markdown
## The KLL pair checks fields, not an estimate

KLL compaction keeps a random half of each level it compacts, using a
`thread_local` engine seeded from the clock that upstream offers no way to
seed. Two correct runs over identical input therefore report different
quantiles, so `kll_update.cc` and `bench_kll_update.rs` cannot use an
estimate as their cross-check. They print `n`, `min`, `max` and `retained`
instead (and `bytes` on `ser`/`deser`): which items survive is random, but
how many survive at each level depends only on the input size and `k`, so
those must match exactly. `compare.py` compares every labelled field
generically, so it needed no change.

The `quantile` scenario times `get_quantile` on a filled sketch, per call
over a fixed `QUERY_CALLS`, after one untimed call has built the sketch's
cached sorted view.
```

- [ ] **Step 8: Update the CI bench comment**

In `.github/workflows/ci.yml`, in the `benches` job comment, change `this one step covers the five C++ programs, run.sh itself (...), and their five Rust counterparts.` to say `six` in both places.

- [ ] **Step 9: Commit**

```bash
rustfmt --edition 2021 apache-datasketches/examples/bench_kll_update.rs
git add benches apache-datasketches/examples/bench_kll_update.rs apache-datasketches/Cargo.toml .github/workflows/ci.yml
git commit -m "bench(kll): add the KLL harness pair, cross-checked on deterministic fields"
```

---

### Task 9: Example, docs and CI matrix

**Files:**
- Create: `apache-datasketches/examples/kll.rs`
- Modify: `apache-datasketches/Cargo.toml`
- Modify: `apache-datasketches/src/lib.rs` (crate docs)
- Modify: `README.md`, `apache-datasketches/README.md`, `apache-datasketches-sys/README.md`, `AGENTS.md`
- Modify: `.github/workflows/ci.yml` (feature matrix)

**Interfaces:**
- Consumes: full public KLL API.

- [ ] **Step 1: Write the example**

Create `apache-datasketches/examples/kll.rs`:

```rust
//! Standalone demo of the KLL quantiles sketch.
//!
//! Run with: `cargo run -p apache-datasketches --example kll --features kll`

use apache_datasketches::kll::{kolmogorov_smirnov_test, KllSketch};

fn main() {
    // A KLL sketch summarizes a stream of numbers so you can ask for ranks
    // and quantiles (p50, p99, ...) afterwards, in bounded memory. `k` trades
    // memory for accuracy; 200 gives about 1.33% rank error (1.65% for PMF).
    let mut monday = KllSketch::<f64>::new(200).expect("valid k");
    for i in 0..100_000u32 {
        // A skewed "latency" distribution, in milliseconds.
        let x = f64::from(i % 1000) / 1000.0;
        monday.update(5.0 + 200.0 * x * x * x);
    }

    println!("requests seen: {}", monday.get_n());
    println!(
        "p50 = {:.1} ms, p99 = {:.1} ms, max = {:.1} ms",
        monday.get_quantile(0.50, true).unwrap(),
        monday.get_quantile(0.99, true).unwrap(),
        monday.get_max_item().unwrap()
    );
    println!(
        "fraction of requests under 50 ms: {:.3}",
        monday.get_rank(50.0, true).unwrap()
    );

    // PMF: the share of requests falling into each bucket.
    let buckets = [10.0, 50.0, 100.0];
    let pmf = monday.get_pmf(&buckets, true).unwrap();
    println!("<=10ms {:.3}, <=50ms {:.3}, <=100ms {:.3}, >100ms {:.3}", pmf[0], pmf[1], pmf[2], pmf[3]);

    // Sketches merge, e.g. per-server sketches into a fleet-wide one.
    let mut tuesday = KllSketch::<f64>::new(200).expect("valid k");
    for i in 0..100_000u32 {
        let x = f64::from(i % 1000) / 1000.0;
        tuesday.update(5.0 + 400.0 * x * x * x);
    }
    let mut both = monday.clone();
    both.merge(&tuesday);
    println!("both days, p99 = {:.1} ms", both.get_quantile(0.99, true).unwrap());

    // Did the distribution change between the two days?
    let changed = kolmogorov_smirnov_test(&monday, &tuesday, 0.05).unwrap();
    println!("distribution changed (p = 0.05): {changed}");

    // A sorted view answers the same queries and can be shared across threads.
    let view = std::sync::Arc::new(both.sorted_view());
    println!("sorted view holds {} retained items", view.len());

    // Bytes are compatible with the Java and Python KllDoublesSketch.
    let bytes = monday.serialize();
    let restored = KllSketch::<f64>::deserialize(&bytes).expect("valid bytes");
    println!("serialized to {} bytes; restored n = {}", bytes.len(), restored.get_n());
}
```

In `apache-datasketches/Cargo.toml`, after the `tuple_generic` example entry add:

```toml
[[example]]
name = "kll"
required-features = ["kll"]
```

Run: `cargo run -p apache-datasketches --example kll --features kll`
Expected: runs to completion and prints each line above, including `distribution changed (p = 0.05): true`. (Monday is `5 + 200x³`, Tuesday `5 + 400x³`; their CDFs differ by up to `1 − (1/2)^(1/3) ≈ 0.21`, well above the ≈0.11 threshold at `k = 200`. If it prints `false`, check the arithmetic before suspecting the binding, and do not edit the expected line to match.)

- [ ] **Step 2: Crate docs in `lib.rs`**

In `apache-datasketches/src/lib.rs`:
- `//! All four sketch families are enabled by default.` → `//! All five sketch families are enabled by default.`
- after the `tuple` bullet add:

```rust
//! - `kll` (feature `kll`) — KLL quantiles sketches over `f64` or `f32`:
//!   ranks, quantiles, PMF and CDF, merging, a shareable sorted view, and a
//!   Kolmogorov–Smirnov test between two sketches.
```

- `see `hll`/`theta`/`cpc`/`tuple` in the` → `see `hll`/`theta`/`cpc`/`tuple`/`kll` in the`.

- [ ] **Step 3: READMEs**

- `README.md`: `All four sketch families` → `All five sketch families`; add `cargo run -p apache-datasketches --example kll --features kll` after the `cpc` example line; after the `- [x] `tuple` — ...` bullet add:

```markdown
- [x] `kll` — quantiles (ranks, quantiles, PMF, CDF) over `f64` or `f32`,
  with merging, a shareable sorted view, and a Kolmogorov–Smirnov test.
```

- `apache-datasketches/README.md`: in line 7, `All four sketch families — `hll`, `theta`, `cpc` and `tuple` —` → `All five sketch families — `hll`, `theta`, `cpc`, `tuple` and `kll` —`; add a `## KLL sketches` section after the Tuple section, mirroring the CPC section's shape, containing the `kll/mod.rs` doc example code block, one sentence on `!Sync` and `sorted_view()`, and `See examples/kll.rs (cargo run -p apache-datasketches --example kll --features kll) for a complete runnable demo.`; in `## Sketch families` add `- [x] KLL — `kll` feature (quantiles over `f64`/`f32`, merge, sorted view, Kolmogorov–Smirnov test).`
- `apache-datasketches-sys/README.md`: add a `kll` bullet to the feature list matching the `cpc` bullet's style (`- `kll` — raw bridge to the KLL quantiles sketch (`kll_sketch<double>` and `kll_sketch<float>`)`), and change `features = ["hll", "theta", "cpc", "tuple"]` to `features = ["hll", "theta", "cpc", "tuple", "kll"]`.

- [ ] **Step 4: AGENTS.md**

- Line 13–14: `There are four sketch families, each behind a Cargo feature: `hll`, `theta`, `cpc`, `tuple`.` → `There are five sketch families, each behind a Cargo feature: `hll`, `theta`, `cpc`, `tuple`, `kll`.`; `enables all four by default` → `enables all five by default`.
- In "Common commands", add `cargo test -p apache-datasketches --features kll` to the per-family list, `cargo run -p apache-datasketches --example kll --features kll` to the examples, and `cargo run --release -p apache-datasketches --example bench_kll_update --features kll` to the bench list.
- In "Vendored C++ sources", `(`common/`, `hll/`, `theta/`, `cpc/`, `tuple/` `include/` dirs` → add `kll/`; `outside the four directories above` → `outside the directories above`.
- After the generic-Tuple paragraph in "Layering", add:

```markdown
The `kll` feature is the one family whose shim is a C++ *template*:
`cpp/kll/kll_sketch_shim.h` defines `KllSketchShim<T>`/`KllSortedViewShim<T>`
once, the `.cc` explicitly instantiates them for `double` and `float`, and two
bridges (`kll_doubles.rs`, `kll_floats.rs`) declare the concrete aliases.
Keep those two bridge files identical modulo type. On the safe side, the
sealed `KllItem` trait in `src/kll/item.rs` is the only per-type code; add an
item type by adding an instantiation, a bridge and one `impl_sealed!` line.
KLL compaction is randomized and unseedable, so tests may assert quantiles
exactly only while `n <= k`, and its bench pair cross-checks
`n`/`min`/`max`/`retained` rather than an estimate.
```

- [ ] **Step 5: CI feature matrix**

In `.github/workflows/ci.yml` `features.strategy.matrix.features`, after `- "--no-default-features --features tuple"` add `- "--no-default-features --features kll"`, and change the comment `- "" # default: all four` to `- "" # default: all five`.

- [ ] **Step 6: Full verification**

Run each; all must succeed:

```bash
cargo build -p apache-datasketches --no-default-features --features kll
cargo test --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo build -p apache-datasketches --no-default-features 2>&1 | grep -q "no sketch family is enabled"
cargo fmt --all -- --check
```

Expected: tests pass; no clippy or rustdoc warnings; the guard grep exits 0; fmt check clean.

- [ ] **Step 7: Commit**

```bash
git add apache-datasketches README.md apache-datasketches-sys/README.md AGENTS.md .github/workflows/ci.yml
git commit -m "docs(kll): document the KLL family, add its example and CI leg"
```

---

### Task 10: Release preparation (versions and CHANGELOG)

Publishing to crates.io and tagging are **not** part of this plan: they are outward-facing and need the maintainer's explicit go-ahead after the PR merges. This task only makes the branch release-ready.

**Files:**
- Modify: `apache-datasketches-sys/Cargo.toml`, `apache-datasketches/Cargo.toml`, `Cargo.lock`
- Modify: `CHANGELOG.md`
- Modify: `apache-datasketches/src/lib.rs`, `README.md`, `apache-datasketches/README.md` (version strings in snippets)

- [ ] **Step 1: Bump versions**

- `apache-datasketches-sys/Cargo.toml`: `version = "0.2.2"` → `version = "0.2.3"`.
- `apache-datasketches/Cargo.toml`: `version = "0.2.3"` → `version = "0.3.0"`; the dependency `apache-datasketches-sys = { version = "0.2.2", ...` → `version = "0.2.3"`.
- In `apache-datasketches/src/lib.rs`, `README.md` and `apache-datasketches/README.md`, change dependency snippets `apache-datasketches = "0.2"` / `version = "0.2"` (for the safe crate) to `"0.3"`. Leave sys-crate snippets at `"0.2"`.

Run: `cargo build --workspace --all-features` (updates `Cargo.lock`).

- [ ] **Step 2: CHANGELOG**

In `CHANGELOG.md`, directly under `## [Unreleased]` and above its existing `### Added`, insert:

```markdown
`apache-datasketches` 0.3.0 and `apache-datasketches-sys` 0.2.3, both crates.

### Breaking (`apache-datasketches`)
- `SketchError` is now `#[non_exhaustive]`, and gains a variant,
  `SketchError::EmptySketch`. A `match` on `SketchError` needs a wildcard arm
  (`_ => ...`); this is the last time adding a variant will break you.

### Added
- **KLL quantiles sketch family** (`kll` feature, on by default):
  `KllSketch<T>` for `T = f64` or `f32` — rank, quantile, PMF and CDF
  queries, `merge`, serialization compatible with the Java and Python
  `KllDoublesSketch`/`KllFloatsSketch`, `iter()` over retained items, and
  `normalized_rank_error_for_k`. `KllSortedView<T>` is an immutable sorted
  snapshot answering the same queries. `kolmogorov_smirnov_delta`,
  `_threshold` and `_test` compare two sketches' distributions.

  Sharp edges, documented and tested: `KllSketch` is `Send` but not `Sync`,
  because upstream caches a sorted view inside the sketch on query — share a
  `KllSortedView` (which is `Sync`) across threads instead. Compaction is
  randomized upstream and cannot be seeded, so sketches fed identical input
  can give slightly different answers once past `k` items. Queries on an
  empty sketch, including the Kolmogorov–Smirnov functions, return
  `EmptySketch` rather than upstream's NaN/sentinel values.
- `apache-datasketches-sys`: `kll` feature with the `kll_doubles` and
  `kll_floats` bridges.
- `examples/bench_kll_update.rs` / `benches/cpp_reference/kll_update.cc`.
  The pair cross-checks `n`, `min`, `max` and `retained` instead of an
  estimate, since KLL's estimates are randomized.
```

Then, because the existing `### Added` / `### Changed` subsections under `[Unreleased]` now follow a second `### Added`, merge them: move the existing `[Unreleased]` `### Added` bullets into the new `### Added` list (after the KLL bullets), keeping the existing `### Changed` subsection as is.

Do **not** quote any KLL ns/op figure in the CHANGELOG unless it comes from `./benches/cpp_reference/compare.sh --ladder` per AGENTS.md; this plan does not require one.

- [ ] **Step 3: Verify packaging**

```bash
cargo package -p apache-datasketches-sys --allow-dirty --no-verify
cargo test --workspace --all-features
```

Expected: package lists `vendor/datasketches-cpp/kll/include/kll_sketch.hpp` and `cpp/kll/kll_sketch_shim.cc` (check with `cargo package -p apache-datasketches-sys --list --allow-dirty | grep kll`); tests pass.

- [ ] **Step 4: Commit**

```bash
git add Cargo.lock apache-datasketches-sys/Cargo.toml apache-datasketches/Cargo.toml CHANGELOG.md apache-datasketches/src/lib.rs README.md apache-datasketches/README.md
git commit -m "chore(release): prepare apache-datasketches 0.3.0 and apache-datasketches-sys 0.2.3"
```

- [ ] **Step 5: Hand back**

Stop here. Report to the maintainer that `feat/kll` is ready for a PR; opening the PR, publishing and tagging `v0.3.0` wait for their go-ahead.
