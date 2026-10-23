# REQ Rust Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the REQ (Relative Error Quantiles) sketch family (`f64` and `f32` items, sorted view) to `apache-datasketches`/`apache-datasketches-sys`, behind a new `req` feature, mirroring the KLL family's design.

**Architecture:** One C++ class template (`ReqSketchShim<T>`, `ReqSortedViewShim<T>`) wraps upstream `req_sketch<T>` and `quantiles_sorted_view<T>`, explicitly instantiated for `double`/`float` and exposed under concrete aliases. Two cxx bridges (`req_doubles.rs`, `req_floats.rs`), identical modulo type. The safe crate has a generic `ReqSketch<T: ReqItem>`; the sealed `ReqItem` trait (macro-implemented for `f64`/`f32`) is the only per-type code. Untrusted bytes are validated in the shim before upstream reads them.

**Tech Stack:** Rust 2021, `cxx`/`cxx-build` 1.x, vendored Apache DataSketches C++ 5.2.0, C++17.

**Spec:** `docs/superpowers/specs/2026-10-23-req-rust-bindings-design.md` — read it before starting any task.

**Template:** the KLL family is the reference implementation. When a task says "mirror KLL file X", open that file — it is committed and reviewed — and reproduce its structure with the REQ differences this plan lists. The KLL files are:
`apache-datasketches-sys/cpp/kll/kll_sketch_shim.{h,cc}`, `apache-datasketches-sys/src/kll_{doubles,floats}.rs`, `apache-datasketches/src/kll/{checks,item,sketch,sorted_view,mod}.rs`, `apache-datasketches/tests/kll_*.rs`, `apache-datasketches/tests/kll_common/mod.rs`, `apache-datasketches/examples/{kll,bench_kll_update}.rs`, `benches/cpp_reference/kll_update.cc`.

## Global Constraints

- Work on branch `feat/req` (already created; the spec is committed there).
- Commit messages: conventional prefix (`feat(req):`, `test(req):`, `fix(req):`, `docs(req):`, `bench(req):`, `chore(req):`). **No `Co-Authored-By` trailer and no AI attribution of any kind** in commits, PR bodies or comments.
- Every public item in `apache-datasketches` needs a `///`/`//!` doc comment; `#![warn(missing_docs)]` must stay clean.
- cxx bridge free-function names and opaque type names must be unique across the whole sys crate; prefix free functions `req_doubles_` / `req_floats_`. Methods need no prefix.
- `req_doubles.rs` and `req_floats.rs` stay identical modulo `f64`↔`f32` and `Doubles`↔`Floats`; ranks, rank bounds, RSE and PMF/CDF masses are `f64` in both.
- `k` must be even and in `4..=1024`; `Default` is `k = 12`, `RankAccuracy::High`.
- Errors: bad arguments (incl. `merge` accuracy mismatch / `n` overflow) → `SketchError::InvalidConfig`; query or rank bound on empty → `SketchError::EmptySketch`; bad bytes → `SketchError::Deserialization`; anything else → `SketchError::Cpp`.
- `ReqSketch` is `Send`, **not** `Sync`. `ReqSortedView` is `Send + Sync`.
- REQ compaction is randomized (time-seeded, unseedable). Never assert an exact quantile/rank once `is_estimation_mode()` is true; assert within upstream's bounds. `n`, `k`, min, max and every result while not in estimation mode are deterministic.
- Format Rust files with `rustfmt --edition 2021 <file>` (not `cargo fmt -- <file>`, which reformats the whole workspace).
- No performance figure may be quoted anywhere unless it comes from `./benches/cpp_reference/compare.sh --ladder`.

## File map

| File | Status | Responsibility |
|---|---|---|
| `apache-datasketches-sys/vendor/datasketches-cpp/req/include/*` | create (copy) | vendored upstream REQ headers |
| `apache-datasketches-sys/vendor/README.md` | modify | copy commands include `req/` |
| `apache-datasketches-sys/tests/vendor_drift_test.rs` | modify | `MODULES` includes `req` |
| `apache-datasketches-sys/Cargo.toml` | modify | `req` feature, link-test targets |
| `apache-datasketches-sys/build.rs` | modify | bridges, `.cc`, include dirs |
| `apache-datasketches-sys/cpp/req/req_sketch_shim.{h,cc}` | create | template shim, aliases, free fns, (Task 4) layout validation |
| `apache-datasketches-sys/src/req_{doubles,floats}.rs` | create | cxx bridges |
| `apache-datasketches-sys/src/lib.rs` | modify | module declarations |
| `apache-datasketches-sys/tests/req_{doubles,floats}_link_test.rs` | create | link tests |
| `apache-datasketches/Cargo.toml` | modify | `req` feature, test/example targets |
| `apache-datasketches/src/lib.rs` | modify | `pub mod req`, guard, crate docs |
| `apache-datasketches/src/req/{mod,checks,item,sketch,sorted_view}.rs` | create | safe layer |
| `apache-datasketches/tests/req_common/mod.rs` | create | shared test helpers |
| `apache-datasketches/tests/req_*.rs` | create | tests |
| `apache-datasketches/examples/{req,bench_req_update}.rs` | create | demo, bench harness |
| `benches/cpp_reference/req_update.cc` | create | C++ bench counterpart |
| `benches/cpp_reference/{run.sh,compare.sh,README.md}` | modify | register the pair |
| `.github/workflows/ci.yml`, `README.md`, `apache-datasketches/README.md`, `apache-datasketches-sys/README.md`, `AGENTS.md`, `CHANGELOG.md` | modify | CI leg and docs |

---

### Task 1: Vendor the REQ headers and add the sys `req` feature

**Files:**
- Create: `apache-datasketches-sys/vendor/datasketches-cpp/req/include/` (copied)
- Modify: `apache-datasketches-sys/vendor/README.md`, `apache-datasketches-sys/tests/vendor_drift_test.rs`, `apache-datasketches-sys/Cargo.toml`

**Interfaces:**
- Produces: sys feature `req`; headers `req_sketch.hpp` (+ `_impl`), `req_compactor.hpp` (+ `_impl`), `req_common.hpp` under `vendor/datasketches-cpp/req/include`.

- [ ] **Step 1: Extend the drift test (it must fail)**

In `apache-datasketches-sys/tests/vendor_drift_test.rs`, change

```rust
const MODULES: [&str; 6] = ["common", "hll", "cpc", "theta", "tuple", "kll"];
```

to

```rust
const MODULES: [&str; 7] = ["common", "hll", "cpc", "theta", "tuple", "kll", "req"];
```

- [ ] **Step 2: Run it**

Run: `cargo test -p apache-datasketches-sys --test vendor_drift_test`
Expected: FAIL with `req: missing from the vendored copy entirely`.

- [ ] **Step 3: Copy the headers**

```bash
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/req
cp -R vendor/datasketches-cpp/req/include apache-datasketches-sys/vendor/datasketches-cpp/req/include
```

- [ ] **Step 4: Run the drift test again**

Run: `cargo test -p apache-datasketches-sys --test vendor_drift_test`
Expected: PASS.

- [ ] **Step 5: Vendor README**

In `apache-datasketches-sys/vendor/README.md`: add `req/include` to the sentence listing compiled headers (after `kll/include`); in the bash block add `mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/req` after the `kll` mkdir line and `cp -R vendor/datasketches-cpp/req/include apache-datasketches-sys/vendor/datasketches-cpp/req/include` after the `kll` cp line; in the closing paragraph append `+`req/`` after `` `kll/` ``. Re-wrap any line that exceeds ~80 characters.

- [ ] **Step 6: Sys feature**

In `apache-datasketches-sys/Cargo.toml` `[features]`, after `kll = []` add `req = []`.

Run: `cargo build -p apache-datasketches-sys --features req` — Expected: success.

- [ ] **Step 7: Commit**

```bash
git add apache-datasketches-sys/vendor apache-datasketches-sys/tests/vendor_drift_test.rs apache-datasketches-sys/Cargo.toml
git commit -m "chore(req): vendor the REQ headers and add the sys req feature"
```

---

### Task 2: C++ shim, cxx bridges and link tests

**Files:**
- Create: `apache-datasketches-sys/cpp/req/req_sketch_shim.h`, `apache-datasketches-sys/cpp/req/req_sketch_shim.cc`
- Create: `apache-datasketches-sys/src/req_doubles.rs`, `apache-datasketches-sys/src/req_floats.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`, `apache-datasketches-sys/build.rs`, `apache-datasketches-sys/Cargo.toml`
- Test: `apache-datasketches-sys/tests/req_doubles_link_test.rs`, `apache-datasketches-sys/tests/req_floats_link_test.rs`

**Interfaces:**
- Consumes: sys feature `req`, vendored headers (Task 1).
- Produces (module `apache_datasketches_sys::req_doubles::ffi`; `req_floats::ffi` identical with `f32`/`Floats` in item positions):
  - opaque types `ReqDoublesSketchShim`, `ReqDoublesSortedViewShim`
  - `fn req_doubles_new(k: u16, hra: bool) -> Result<UniquePtr<ReqDoublesSketchShim>>`
  - `fn req_doubles_deserialize(bytes: &[u8]) -> Result<UniquePtr<ReqDoublesSketchShim>>`
  - `fn req_doubles_rse(k: u16, rank: f64, hra: bool, n: u64) -> f64`
  - sketch methods: `clone_shim(&self) -> UniquePtr<Self>`, `update(Pin<&mut Self>, f64)`, `merge(Pin<&mut Self>, &Self) -> Result<()>`, `is_empty`, `get_n -> u64`, `get_k -> u16`, `get_num_retained -> u32`, `is_estimation_mode`, `is_hra -> bool`, `get_min_item -> Result<f64>`, `get_max_item -> Result<f64>`, `get_quantile(f64, bool) -> Result<f64>`, `get_rank(f64, bool) -> Result<f64>`, `get_pmf(&[f64], bool) -> Result<UniquePtr<CxxVector<f64>>>`, `get_cdf(..)` (same), `get_rank_lower_bound(f64, u8) -> f64`, `get_rank_upper_bound(f64, u8) -> f64`, `serialize -> UniquePtr<CxxVector<u8>>`, `to_string_summary -> String`, `retained_items -> UniquePtr<CxxVector<f64>>`, `retained_weights -> UniquePtr<CxxVector<u64>>`, `sorted_view -> UniquePtr<ReqDoublesSortedViewShim>`
  - view methods: exactly KLL's — `size -> usize`, `get_quantile`, `get_rank`, `get_pmf`, `get_cdf`, `items`, `weights`, `cumulative_weights`

- [ ] **Step 1: Write the doubles link test**

Create `apache-datasketches-sys/tests/req_doubles_link_test.rs`:

```rust
#![cfg(feature = "req")]

use apache_datasketches_sys::req_doubles::ffi;

#[test]
fn construct_update_query_roundtrip() {
    let mut sketch = ffi::req_doubles_new(12, true).unwrap();
    for i in 1..=10 {
        sketch.pin_mut().update(f64::from(i));
    }
    assert_eq!(sketch.get_n(), 10);
    assert_eq!(sketch.get_k(), 12);
    assert!(sketch.is_hra());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_num_retained(), 10);
    assert_eq!(sketch.get_min_item().unwrap(), 1.0);
    assert_eq!(sketch.get_max_item().unwrap(), 10.0);
    assert_eq!(sketch.get_rank(5.0, true).unwrap(), 0.5);
    assert_eq!(sketch.get_cdf(&[5.0], true).unwrap().as_slice(), &[0.5, 1.0]);
    assert!(sketch.get_rank_lower_bound(0.5, 2) <= 0.5);
    assert!(sketch.get_rank_upper_bound(0.5, 2) >= 0.5);

    let restored = ffi::req_doubles_deserialize(sketch.serialize().as_slice()).unwrap();
    assert_eq!(restored.get_n(), 10);

    let view = sketch.sorted_view();
    assert_eq!(view.size(), 10);
    assert_eq!(view.items().as_slice()[0], 1.0);
    assert_eq!(view.cumulative_weights().as_slice()[9], 10);

    let mut copy = sketch.clone_shim();
    copy.pin_mut().merge(&sketch).unwrap();
    assert_eq!(copy.get_n(), 20);
}

#[test]
fn merging_hra_into_lra_returns_err() {
    let mut hra = ffi::req_doubles_new(12, true).unwrap();
    let mut lra = ffi::req_doubles_new(12, false).unwrap();
    hra.pin_mut().update(1.0);
    lra.pin_mut().update(1.0);
    assert!(lra.pin_mut().merge(&hra).is_err());
}

#[test]
fn query_on_empty_returns_err() {
    let sketch = ffi::req_doubles_new(12, true).unwrap();
    assert!(sketch.get_quantile(0.5, true).is_err());
}

#[test]
fn out_of_range_rank_returns_err() {
    let mut sketch = ffi::req_doubles_new(12, true).unwrap();
    for i in 1..=10 {
        sketch.pin_mut().update(f64::from(i));
    }
    let view = sketch.sorted_view();
    for rank in [f64::NAN, -0.1, 1.1, f64::INFINITY] {
        assert!(sketch.get_quantile(rank, true).is_err(), "sketch rank {rank}");
        assert!(view.get_quantile(rank, true).is_err(), "view rank {rank}");
    }
    for rank in [0.0, 1.0] {
        assert!(sketch.get_quantile(rank, true).is_ok());
        assert!(view.get_quantile(rank, true).is_ok());
    }
}

#[test]
fn rse_is_finite_and_positive() {
    let rse = ffi::req_doubles_rse(12, 0.5, true, 1000);
    assert!(rse.is_finite() && rse > 0.0, "rse was {rse}");
}
```

Create `apache-datasketches-sys/tests/req_floats_link_test.rs` with the same five tests, written out in full, with these changes: `use apache_datasketches_sys::req_floats::ffi;`, every `req_doubles_` → `req_floats_`, loop bodies `sketch.pin_mut().update(f32::from(i as u8));` with the loop variable `i` in `1..=10u8` (so `update(f32::from(i))`), item literals as `f32` (`1.0f32`, `10.0f32`, `5.0f32`, `&[5.0f32]`), and `update(1.0f32)` in the HRA/LRA test. Rank arguments (`0.5`, `NAN`, …) stay `f64`.

- [ ] **Step 2: Register the test targets**

Append to `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "req_doubles_link_test"
required-features = ["req"]

[[test]]
name = "req_floats_link_test"
required-features = ["req"]
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features req --test req_doubles_link_test`
Expected: FAIL to compile — `could not find req_doubles in apache_datasketches_sys`.

- [ ] **Step 4: Write the shim header**

Create `apache-datasketches-sys/cpp/req/req_sketch_shim.h`:

```cpp
#pragma once
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "req_sketch.hpp"

namespace apache_datasketches_rs {

// One template serves both item types, mirroring kll_sketch_shim.h. The
// sorted-view shim is a deliberate copy of KLL's rather than a shared one,
// so the `req` and `kll` features stay independent (see the REQ design doc).

template<typename T>
using req_sorted_view_t = datasketches::quantiles_sorted_view<T, std::less<T>, std::allocator<T>>;

template<typename T>
class ReqSortedViewShim {
public:
  explicit ReqSortedViewShim(req_sorted_view_t<T> view);

  size_t size() const;
  T get_quantile(double rank, bool inclusive) const;
  double get_rank(T item, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_pmf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_cdf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<T>> items() const;
  std::unique_ptr<std::vector<uint64_t>> weights() const;
  std::unique_ptr<std::vector<uint64_t>> cumulative_weights() const;

private:
  req_sorted_view_t<T> view_;
};

template<typename T>
class ReqSketchShim {
public:
  ReqSketchShim(uint16_t k, bool hra);
  explicit ReqSketchShim(datasketches::req_sketch<T> sketch);

  std::unique_ptr<ReqSketchShim> clone_shim() const;
  void update(T item);
  void merge(const ReqSketchShim& other);

  bool is_empty() const;
  uint64_t get_n() const;
  uint16_t get_k() const;
  uint32_t get_num_retained() const;
  bool is_estimation_mode() const;
  bool is_hra() const;
  T get_min_item() const;
  T get_max_item() const;

  T get_quantile(double rank, bool inclusive) const;
  double get_rank(T item, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_pmf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_cdf(rust::Slice<const T> split_points, bool inclusive) const;
  double get_rank_lower_bound(double rank, uint8_t num_std_dev) const;
  double get_rank_upper_bound(double rank, uint8_t num_std_dev) const;

  std::unique_ptr<std::vector<uint8_t>> serialize() const;
  rust::String to_string_summary() const;

  std::unique_ptr<std::vector<T>> retained_items() const;
  std::unique_ptr<std::vector<uint64_t>> retained_weights() const;
  std::unique_ptr<ReqSortedViewShim<T>> sorted_view() const;

private:
  datasketches::req_sketch<T> sketch_;
};

using ReqDoublesSketchShim = ReqSketchShim<double>;
using ReqFloatsSketchShim = ReqSketchShim<float>;
using ReqDoublesSortedViewShim = ReqSortedViewShim<double>;
using ReqFloatsSortedViewShim = ReqSortedViewShim<float>;

std::unique_ptr<ReqDoublesSketchShim> req_doubles_new(uint16_t k, bool hra);
std::unique_ptr<ReqDoublesSketchShim> req_doubles_deserialize(rust::Slice<const uint8_t> bytes);
double req_doubles_rse(uint16_t k, double rank, bool hra, uint64_t n);

std::unique_ptr<ReqFloatsSketchShim> req_floats_new(uint16_t k, bool hra);
std::unique_ptr<ReqFloatsSketchShim> req_floats_deserialize(rust::Slice<const uint8_t> bytes);
double req_floats_rse(uint16_t k, double rank, bool hra, uint64_t n);

} // namespace apache_datasketches_rs
```

- [ ] **Step 5: Write the shim implementation**

Create `apache-datasketches-sys/cpp/req/req_sketch_shim.cc`. Its content is KLL's `kll_sketch_shim.cc` (open it) with these differences and nothing else:

1. Includes: `#include "req_sketch_shim.h"`, then `<cmath>`, `<cstring>`, `<stdexcept>`, `<string>`. No `kolmogorov_smirnov.hpp`.
2. Keep the anonymous-namespace `check_rank` helper verbatim, and call it first in both `ReqSortedViewShim<T>::get_quantile` and `ReqSketchShim<T>::get_quantile`.
3. The `ReqSortedViewShim<T>` member definitions are KLL's `KllSortedViewShim<T>` definitions with `Kll` → `Req` and `kll_sorted_view_t` → `req_sorted_view_t`.
4. `ReqSketchShim<T>` members: KLL's, with `Kll` → `Req` and `kll_sketch` → `req_sketch`, except:
   - constructor: `ReqSketchShim<T>::ReqSketchShim(uint16_t k, bool hra) : sketch_(k, hra) {}`
   - add `bool ReqSketchShim<T>::is_hra() const { return sketch_.is_HRA(); }`
   - add
     ```cpp
     template<typename T>
     double ReqSketchShim<T>::get_rank_lower_bound(double rank, uint8_t num_std_dev) const {
       return sketch_.get_rank_lower_bound(rank, num_std_dev);
     }

     template<typename T>
     double ReqSketchShim<T>::get_rank_upper_bound(double rank, uint8_t num_std_dev) const {
       return sketch_.get_rank_upper_bound(rank, num_std_dev);
     }
     ```
   - delete `get_normalized_rank_error`, `ks_delta`, `ks_threshold`, `ks_test`.
5. Explicit instantiations: `template class ReqSortedViewShim<double>; template class ReqSortedViewShim<float>; template class ReqSketchShim<double>; template class ReqSketchShim<float>;`
6. Free functions (replacing KLL's; there is no layout validation and no NaN scan yet — Task 4 adds them):
   ```cpp
   std::unique_ptr<ReqDoublesSketchShim> req_doubles_new(uint16_t k, bool hra) {
     return std::make_unique<ReqDoublesSketchShim>(k, hra);
   }

   std::unique_ptr<ReqDoublesSketchShim> req_doubles_deserialize(rust::Slice<const uint8_t> bytes) {
     return std::make_unique<ReqDoublesSketchShim>(
         datasketches::req_sketch<double>::deserialize(bytes.data(), bytes.size()));
   }

   double req_doubles_rse(uint16_t k, double rank, bool hra, uint64_t n) {
     return datasketches::req_sketch<double>::get_RSE(k, rank, hra, n);
   }
   ```
   and the same three for `float` with `Floats`/`req_floats_`/`req_sketch<float>`.

Note upstream `req_sketch::get_min_item()`/`get_max_item()` return `const T&`; the shim returns `T` by value (a copy), as KLL's does.

- [ ] **Step 6: Write the doubles bridge**

Create `apache-datasketches-sys/src/req_doubles.rs`:

```rust
//! cxx bridge for `req_sketch<double>` and its sorted view.
//!
//! Keep identical to `req_floats.rs` modulo `f64`/`f32` and
//! `Doubles`/`Floats`: both declare the one C++ template in
//! `cpp/req/req_sketch_shim.h`. Free functions carry the `req_doubles_`
//! prefix because cxx keys their symbols on name alone (see AGENTS.md);
//! methods need no prefix.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("req_sketch_shim.h");

        type ReqDoublesSketchShim;
        type ReqDoublesSortedViewShim;

        fn req_doubles_new(k: u16, hra: bool) -> Result<UniquePtr<ReqDoublesSketchShim>>;
        fn req_doubles_deserialize(bytes: &[u8]) -> Result<UniquePtr<ReqDoublesSketchShim>>;
        fn req_doubles_rse(k: u16, rank: f64, hra: bool, n: u64) -> f64;

        fn clone_shim(self: &ReqDoublesSketchShim) -> UniquePtr<ReqDoublesSketchShim>;
        fn update(self: Pin<&mut ReqDoublesSketchShim>, item: f64);
        fn merge(self: Pin<&mut ReqDoublesSketchShim>, other: &ReqDoublesSketchShim)
            -> Result<()>;

        fn is_empty(self: &ReqDoublesSketchShim) -> bool;
        fn get_n(self: &ReqDoublesSketchShim) -> u64;
        fn get_k(self: &ReqDoublesSketchShim) -> u16;
        fn get_num_retained(self: &ReqDoublesSketchShim) -> u32;
        fn is_estimation_mode(self: &ReqDoublesSketchShim) -> bool;
        fn is_hra(self: &ReqDoublesSketchShim) -> bool;
        fn get_min_item(self: &ReqDoublesSketchShim) -> Result<f64>;
        fn get_max_item(self: &ReqDoublesSketchShim) -> Result<f64>;

        fn get_quantile(self: &ReqDoublesSketchShim, rank: f64, inclusive: bool) -> Result<f64>;
        fn get_rank(self: &ReqDoublesSketchShim, item: f64, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &ReqDoublesSketchShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &ReqDoublesSketchShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_rank_lower_bound(self: &ReqDoublesSketchShim, rank: f64, num_std_dev: u8) -> f64;
        fn get_rank_upper_bound(self: &ReqDoublesSketchShim, rank: f64, num_std_dev: u8) -> f64;

        fn serialize(self: &ReqDoublesSketchShim) -> UniquePtr<CxxVector<u8>>;
        fn to_string_summary(self: &ReqDoublesSketchShim) -> String;

        fn retained_items(self: &ReqDoublesSketchShim) -> UniquePtr<CxxVector<f64>>;
        fn retained_weights(self: &ReqDoublesSketchShim) -> UniquePtr<CxxVector<u64>>;
        fn sorted_view(self: &ReqDoublesSketchShim) -> UniquePtr<ReqDoublesSortedViewShim>;

        fn size(self: &ReqDoublesSortedViewShim) -> usize;
        fn get_quantile(self: &ReqDoublesSortedViewShim, rank: f64, inclusive: bool)
            -> Result<f64>;
        fn get_rank(self: &ReqDoublesSortedViewShim, item: f64, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &ReqDoublesSortedViewShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &ReqDoublesSortedViewShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn items(self: &ReqDoublesSortedViewShim) -> UniquePtr<CxxVector<f64>>;
        fn weights(self: &ReqDoublesSortedViewShim) -> UniquePtr<CxxVector<u64>>;
        fn cumulative_weights(self: &ReqDoublesSortedViewShim) -> UniquePtr<CxxVector<u64>>;
    }
}
```

`get_rank_lower_bound`/`upper_bound` and `req_*_rse` have no `Result`: upstream computes them arithmetically and never throws (verified in `req_sketch_impl.hpp` `get_rank_lb`/`get_rank_ub`/`get_RSE`); the safe layer validates their arguments.

- [ ] **Step 7: Write the floats bridge**

Create `apache-datasketches-sys/src/req_floats.rs`, written out in full: the doubles bridge with `req_sketch<double>` → `req_sketch<float>`, the doc cross-reference `req_floats.rs` → `req_doubles.rs`, `req_doubles_` → `req_floats_`, `ReqDoubles` → `ReqFloats`, and `f64` → `f32` **only** in item positions: `update`'s `item`, the return types of `get_min_item`/`get_max_item`/both `get_quantile`s (`Result<f32>`), both `get_rank`s' `item: f32`, every `split_points: &[f32]`, and `retained_items`/`items` (`CxxVector<f32>`). Everything else stays `f64`.

Verify:

```bash
diff <(sed -e 's/req_floats/req_doubles/g; s/ReqFloats/ReqDoubles/g; s/req_sketch<float>/req_sketch<double>/g' apache-datasketches-sys/src/req_floats.rs) apache-datasketches-sys/src/req_doubles.rs
```

Expected: only the item-position lines and the doc cross-reference differ.

- [ ] **Step 8: Expose the modules**

Append to `apache-datasketches-sys/src/lib.rs`:

```rust

#[cfg(feature = "req")]
pub mod req_doubles;
#[cfg(feature = "req")]
pub mod req_floats;
```

- [ ] **Step 9: Register in `build.rs`**

In `apache-datasketches-sys/build.rs`, mirroring the `kll` entries exactly:
- after the `kll` bridges block add a `req` block pushing `"src/req_doubles.rs"` and `"src/req_floats.rs"` with `require_exists`;
- add `.include(vendor_dir.join("req/include"))` after the `kll/include` line and `.include("cpp/req")` after `.include("cpp/kll")`;
- after the `kll` `.cc` block add a `req` block adding `"cpp/req/req_sketch_shim.cc"` with `require_exists`;
- append `rerun-if-changed` lines for `src/req_doubles.rs`, `src/req_floats.rs`, `cpp/req/req_sketch_shim.h`, `cpp/req/req_sketch_shim.cc`.

- [ ] **Step 10: Run the link tests and the all-features build**

Run: `cargo test -p apache-datasketches-sys --features req` — Expected: PASS (both link tests, drift test).
Run: `cargo build -p apache-datasketches-sys --all-features` — Expected: success (the duplicate-name guard sees no collision).

- [ ] **Step 11: Format and commit**

```bash
rustfmt --edition 2021 apache-datasketches-sys/src/req_doubles.rs apache-datasketches-sys/src/req_floats.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/build.rs apache-datasketches-sys/tests/req_doubles_link_test.rs apache-datasketches-sys/tests/req_floats_link_test.rs
git add apache-datasketches-sys
git commit -m "feat(req): add the REQ C++ shim and cxx bridges"
```

---

### Task 3: Safe layer — `ReqItem`, `ReqSketch<T>`, `ReqSortedView<T>`

**Files:**
- Modify: `apache-datasketches/Cargo.toml`, `apache-datasketches/src/lib.rs`
- Create: `apache-datasketches/src/req/{mod,checks,item,sketch,sorted_view}.rs`
- Create: `apache-datasketches/tests/req_common/mod.rs`
- Test: `apache-datasketches/tests/req_sketch_smoke_test.rs`, `apache-datasketches/tests/req_sorted_view_test.rs`, `apache-datasketches/tests/req_concurrency_test.rs`

**Interfaces:**
- Consumes: sys `req_doubles::ffi`, `req_floats::ffi` (Task 2).
- Produces:
  - `pub enum RankAccuracy { High, Low }` (derives `Debug, Clone, Copy, PartialEq, Eq`)
  - `pub trait ReqItem: sealed::Sealed + Copy + PartialOrd + Debug + Send + Sync + 'static` for `f64`, `f32`
  - `pub struct ReqSketch<T: ReqItem>` with every method in the spec's Public API; `Clone`, `Default` (k = 12, High), `Send`; `merge(&mut self, &ReqSketch<T>) -> Result<(), SketchError>`
  - `pub struct ReqSortedView<T: ReqItem>` (`Send + Sync`) and `pub struct ReqSortedViewEntry<T> { pub item: T, pub weight: u64, pub cumulative_weight: u64 }` (derives `Debug, Clone, Copy, PartialEq`)
  - crate-private `checks::{check_k, check_rank, check_item, check_split_points, check_num_std_dev}`, `const DEFAULT_K: u16 = 12`
  - test helpers in `tests/req_common/mod.rs`: `trait TestItem: ReqItem { fn from_f64(f64) -> Self; fn to_f64(self) -> f64; }` and macro `for_both_types!(module_name, generic_fn)`

- [ ] **Step 1: Feature and targets**

In `apache-datasketches/Cargo.toml`: add `"req"` to `default` (after `"kll"`) and to `[package.metadata.docs.rs] features`; add `req = ["apache-datasketches-sys/req"]` after the `kll` feature; append:

```toml
[[test]]
name = "req_sketch_smoke_test"
required-features = ["req"]

[[test]]
name = "req_sorted_view_test"
required-features = ["req"]

[[test]]
name = "req_concurrency_test"
required-features = ["req"]
```

- [ ] **Step 2: Test helpers**

Create `apache-datasketches/tests/req_common/mod.rs` as KLL's `tests/kll_common/mod.rs` with `KllItem` → `ReqItem`, `apache_datasketches::kll::KllItem` → `apache_datasketches::req::ReqItem`, the module doc's "KLL" → "REQ" and `kll_common` → `req_common`, and the `Normal` sampler **removed** (REQ has no KS test). Keep `#![allow(dead_code, unused_macros)]`.

- [ ] **Step 3: Write the failing smoke test**

Create `apache-datasketches/tests/req_sketch_smoke_test.rs`:

```rust
#[macro_use]
mod req_common;

use apache_datasketches::req::{RankAccuracy, ReqSketch};
use apache_datasketches::SketchError;
use req_common::TestItem;

fn filled<T: TestItem>(n: u32) -> ReqSketch<T> {
    let mut sketch = ReqSketch::<T>::default();
    for i in 1..=n {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

fn construction<T: TestItem>() {
    let d = ReqSketch::<T>::default();
    assert_eq!(d.get_k(), 12);
    assert_eq!(d.rank_accuracy(), RankAccuracy::High);
    assert_eq!(ReqSketch::<T>::new(4, RankAccuracy::Low).unwrap().rank_accuracy(), RankAccuracy::Low);
    assert_eq!(ReqSketch::<T>::new(1024, RankAccuracy::High).unwrap().get_k(), 1024);
    for k in [0, 2, 3, 13, 1025, 1026, u16::MAX] {
        assert!(
            matches!(ReqSketch::<T>::new(k, RankAccuracy::High), Err(SketchError::InvalidConfig(_))),
            "k = {k}"
        );
    }
}
for_both_types!(construction_t, construction);

fn exact_mode_queries<T: TestItem>() {
    let sketch = filled::<T>(10);
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 10);
    assert_eq!(sketch.get_num_retained(), 10);
    assert_eq!(sketch.get_min_item().unwrap().to_f64(), 1.0);
    assert_eq!(sketch.get_max_item().unwrap().to_f64(), 10.0);
    assert_eq!(sketch.get_rank(T::from_f64(5.0), true).unwrap(), 0.5);
    assert_eq!(sketch.get_rank(T::from_f64(5.0), false).unwrap(), 0.4);
    assert_eq!(sketch.get_quantile(0.5, true).unwrap().to_f64(), 5.0);
    let split = [T::from_f64(3.0), T::from_f64(7.0)];
    assert_eq!(sketch.get_cdf(&split, true).unwrap(), vec![0.3, 0.7, 1.0]);
    let pmf = sketch.get_pmf(&split, true).unwrap();
    assert_eq!(pmf.len(), 3);
    assert!((pmf.iter().sum::<f64>() - 1.0).abs() < 1e-12);
}
for_both_types!(exact_mode_queries_t, exact_mode_queries);

fn empty_sketch<T: TestItem>() {
    let sketch = ReqSketch::<T>::default();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_min_item(), None);
    assert_eq!(sketch.get_max_item(), None);
    assert!(matches!(sketch.get_quantile(0.5, true), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_rank(T::from_f64(0.0), true), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_cdf(&[T::from_f64(0.0)], true), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_rank_lower_bound(0.5, 1), Err(SketchError::EmptySketch)));
    assert!(matches!(sketch.get_rank_upper_bound(0.5, 1), Err(SketchError::EmptySketch)));
    assert_eq!(sketch.iter().count(), 0);
}
for_both_types!(empty_sketch_t, empty_sketch);

fn nan_update_is_ignored<T: TestItem>() {
    let mut sketch = ReqSketch::<T>::default();
    sketch.update(T::from_f64(f64::NAN));
    assert!(sketch.is_empty());
}
for_both_types!(nan_update_is_ignored_t, nan_update_is_ignored);

fn merge_combines<T: TestItem>() {
    let mut a = filled::<T>(10);
    let mut b = ReqSketch::<T>::default();
    for i in 11..=20 {
        b.update(T::from_f64(f64::from(i)));
    }
    a.merge(&b).unwrap();
    assert_eq!(a.get_n(), 20);
    assert_eq!(a.get_min_item().unwrap().to_f64(), 1.0);
    assert_eq!(a.get_max_item().unwrap().to_f64(), 20.0);
    assert_eq!(b.get_n(), 10, "merge must not modify its argument");
}
for_both_types!(merge_combines_t, merge_combines);

fn merge_mismatched_accuracy_fails<T: TestItem>() {
    let mut high = ReqSketch::<T>::new(12, RankAccuracy::High).unwrap();
    let mut low = ReqSketch::<T>::new(12, RankAccuracy::Low).unwrap();
    high.update(T::from_f64(1.0));
    low.update(T::from_f64(1.0));
    assert!(matches!(high.merge(&low), Err(SketchError::InvalidConfig(_))));
    assert!(matches!(low.merge(&high), Err(SketchError::InvalidConfig(_))));
    assert_eq!(high.get_n(), 1, "a rejected merge must not change the sketch");
}
for_both_types!(merge_mismatched_accuracy_fails_t, merge_mismatched_accuracy_fails);

fn rank_bounds<T: TestItem>() {
    let sketch = filled::<T>(10_000);
    for std_dev in 1..=3 {
        let lb = sketch.get_rank_lower_bound(0.5, std_dev).unwrap();
        let ub = sketch.get_rank_upper_bound(0.5, std_dev).unwrap();
        assert!(lb <= 0.5 && 0.5 <= ub, "std_dev {std_dev}: [{lb}, {ub}]");
    }
    for bad in [0, 4] {
        assert!(matches!(sketch.get_rank_lower_bound(0.5, bad), Err(SketchError::InvalidConfig(_))));
        assert!(matches!(sketch.get_rank_upper_bound(0.5, bad), Err(SketchError::InvalidConfig(_))));
    }
    assert!(matches!(sketch.get_rank_lower_bound(1.5, 1), Err(SketchError::InvalidConfig(_))));
}
for_both_types!(rank_bounds_t, rank_bounds);

fn relative_standard_error<T: TestItem>() {
    let rse = ReqSketch::<T>::relative_standard_error(12, 0.5, RankAccuracy::High, 1000).unwrap();
    assert!(rse.is_finite() && rse > 0.0);
    assert!(matches!(
        ReqSketch::<T>::relative_standard_error(13, 0.5, RankAccuracy::High, 1000),
        Err(SketchError::InvalidConfig(_))
    ));
    assert!(matches!(
        ReqSketch::<T>::relative_standard_error(12, f64::NAN, RankAccuracy::High, 1000),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(relative_standard_error_t, relative_standard_error);

fn serialize_roundtrip<T: TestItem>() {
    for accuracy in [RankAccuracy::High, RankAccuracy::Low] {
        let mut sketch = ReqSketch::<T>::new(12, accuracy).unwrap();
        for i in 0..10_000 {
            sketch.update(T::from_f64(f64::from(i)));
        }
        let restored = ReqSketch::<T>::deserialize(&sketch.serialize()).unwrap();
        assert_eq!(restored.get_n(), sketch.get_n());
        assert_eq!(restored.get_k(), sketch.get_k());
        assert_eq!(restored.rank_accuracy(), accuracy);
        assert_eq!(restored.get_num_retained(), sketch.get_num_retained());
        assert_eq!(restored.get_min_item(), sketch.get_min_item());
        assert_eq!(restored.get_max_item(), sketch.get_max_item());
        assert_eq!(restored.get_quantile(0.5, true).unwrap(), sketch.get_quantile(0.5, true).unwrap());
    }
}
for_both_types!(serialize_roundtrip_t, serialize_roundtrip);

fn garbage_bytes_fail<T: TestItem>() {
    assert!(matches!(ReqSketch::<T>::deserialize(&[1, 2, 3]), Err(SketchError::Deserialization(_))));
}
for_both_types!(garbage_bytes_fail_t, garbage_bytes_fail);

fn iter_covers_every_retained_item<T: TestItem>() {
    let sketch = filled::<T>(10_000);
    let pairs: Vec<(T, u64)> = sketch.iter().collect();
    assert_eq!(pairs.len() as u32, sketch.get_num_retained());
    assert_eq!(pairs.iter().map(|&(_, w)| w).sum::<u64>(), sketch.get_n());
}
for_both_types!(iter_covers_every_retained_item_t, iter_covers_every_retained_item);

fn clone_is_independent<T: TestItem>() {
    let mut original = filled::<T>(10);
    let copy = original.clone();
    original.update(T::from_f64(11.0));
    assert_eq!(copy.get_n(), 10);
}
for_both_types!(clone_is_independent_t, clone_is_independent);

fn to_string_summary_is_non_empty<T: TestItem>() {
    assert!(!filled::<T>(10).to_string_summary().is_empty());
}
for_both_types!(to_string_summary_is_non_empty_t, to_string_summary_is_non_empty);
```

- [ ] **Step 4: Write the failing sorted-view and concurrency tests**

Create `apache-datasketches/tests/req_sorted_view_test.rs`: KLL's `tests/kll_sorted_view_test.rs` with `kll_common` → `req_common`, `apache_datasketches::kll::{KllSketch, SortedViewEntry}` → `apache_datasketches::req::{ReqSketch, ReqSortedViewEntry}`, `KllSketch` → `ReqSketch`, `SortedViewEntry` → `ReqSortedViewEntry`, the `upstream_three_items` comment pointing at upstream REQ's `get_rank equivalence` instead of KLL's "sorted view" section, and `view_matches_sketch` asserting `sketch.is_estimation_mode()` on a 10 000-item sketch as it does for KLL.

Create `apache-datasketches/tests/req_concurrency_test.rs`: KLL's `tests/kll_concurrency_test.rs` with `apache_datasketches::kll::KllSketch` → `apache_datasketches::req::ReqSketch`, `KllSketch` → `ReqSketch`, and in `per_thread_sketches_merge` `merged.merge(&h.join().unwrap());` → `merged.merge(&h.join().unwrap()).unwrap();`.

- [ ] **Step 5: Run to verify they fail**

Run: `cargo test -p apache-datasketches --features req --test req_sketch_smoke_test --test req_sorted_view_test --test req_concurrency_test`
Expected: FAIL to compile — `could not find req in apache_datasketches`.

- [ ] **Step 6: Wire the module into the crate root**

In `apache-datasketches/src/lib.rs` add, after the `kll` module:

```rust

#[cfg(feature = "req")]
pub mod req;
```

and add `feature = "req"` to the `compile_error!` guard's `any(...)` list and `` `req` `` to its message (keep the phrase `no sketch family is enabled`).

- [ ] **Step 7: Write `checks.rs`**

Create `apache-datasketches/src/req/checks.rs`:

```rust
//! Argument validation shared by the sketch and the sorted view. It runs
//! before crossing FFI, so callers get a typed [`SketchError::InvalidConfig`]
//! instead of a C++ exception message.

use super::ReqItem;
use crate::error::SketchError;

/// Upstream `req_constants::MIN_K`.
pub(crate) const MIN_K: u16 = 4;
/// Upstream's documented upper bound for `k` (`req_sketch.hpp`); the
/// constructor itself never checks it.
pub(crate) const MAX_K: u16 = 1024;

/// Upstream documents `k` as even and in `[4, 1024]` but silently rounds an
/// odd `k` down and clamps below 4; this enforces the documented contract.
pub(crate) fn check_k(k: u16) -> Result<(), SketchError> {
    if !(MIN_K..=MAX_K).contains(&k) || k % 2 != 0 {
        return Err(SketchError::InvalidConfig(format!(
            "k must be even and in {MIN_K}..={MAX_K}, got {k}"
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

pub(crate) fn check_num_std_dev(num_std_dev: u8) -> Result<(), SketchError> {
    if !(1..=3).contains(&num_std_dev) {
        return Err(SketchError::InvalidConfig(format!(
            "num_std_dev must be 1, 2 or 3, got {num_std_dev}"
        )));
    }
    Ok(())
}
```

followed by `check_item` and `check_split_points` copied verbatim from KLL's `checks.rs` with `KllItem` → `ReqItem`.

- [ ] **Step 8: Write `item.rs`**

Create `apache-datasketches/src/req/item.rs` as KLL's `item.rs` with these differences:

- imports: `use apache_datasketches_sys::{req_doubles::ffi as doubles, req_floats::ffi as floats};`; doc comments name `ReqSketch`/`ReqItem`, and the `ReqItem` doc says `f32` bytes interoperate with Java's `ReqSketch` and Python's float REQ sketch, while `f64` bytes are readable only by C++ and this crate.
- `trait ReqItem` / `impl ReqItem for f64 {}` / `impl ReqItem for f32 {}`.
- in `Sealed`:
  - `fn sketch_new(k: u16, hra: bool) -> Result<UniquePtr<Self::Sketch>, Exception>;`
  - replace `normalized_rank_error_for_k` with `fn rse(k: u16, rank: f64, hra: bool, n: u64) -> f64;`
  - `fn merge(s: Pin<&mut Self::Sketch>, other: &Self::Sketch) -> Result<(), Exception>;`
  - add `fn is_hra(s: &Self::Sketch) -> bool;`
  - replace `get_normalized_rank_error` with `fn rank_lower_bound(s: &Self::Sketch, rank: f64, num_std_dev: u8) -> f64;` and `fn rank_upper_bound(s: &Self::Sketch, rank: f64, num_std_dev: u8) -> f64;`
  - remove `ks_delta`, `ks_threshold`, `ks_test`.
  - every other item (`type Sketch`, `type View`, `is_nan_item`, `sketch_deserialize`, the query/serialize/iter fns, `sorted_view`, all `view_*` fns) unchanged.
- in `impl_sealed!`: parameters `($t:ty, $m:ident, $sketch:ident, $view:ident, $new:ident, $deser:ident, $rse:ident)`; bodies forward to the matching bridge fn/method (`$m::$new(k, hra)`, `$m::$rse(k, rank, hra, n)`, `s.merge(other)`, `s.is_hra()`, `s.get_rank_lower_bound(rank, num_std_dev)`, `s.get_rank_upper_bound(rank, num_std_dev)`); the rest exactly as KLL's macro.
- invocations: `impl_sealed!(f64, doubles, ReqDoublesSketchShim, ReqDoublesSortedViewShim, req_doubles_new, req_doubles_deserialize, req_doubles_rse);` and the `f32`/`floats`/`ReqFloats…`/`req_floats_…` equivalent.

- [ ] **Step 9: Write `sketch.rs`**

Create `apache-datasketches/src/req/sketch.rs`:

```rust
use super::checks::{check_item, check_k, check_num_std_dev, check_rank, check_split_points};
use super::ReqItem;
use super::ReqSortedView;
use crate::error::SketchError;
use cxx::UniquePtr;

/// The `k` Java and Python default to: about 1% relative error at 95%
/// confidence.
const DEFAULT_K: u16 = 12;

/// Which end of the rank domain gets REQ's relative-error guarantee.
///
/// Upstream calls this `hra` (high rank accuracy), a `bool` defaulting to
/// `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankAccuracy {
    /// Accuracy concentrated at high ranks (p99, p99.9, …). The default.
    High,
    /// Accuracy concentrated at low ranks (p1, p0.1, …).
    Low,
}

impl RankAccuracy {
    fn is_high(self) -> bool {
        self == RankAccuracy::High
    }

    fn from_hra(hra: bool) -> Self {
        if hra {
            RankAccuracy::High
        } else {
            RankAccuracy::Low
        }
    }
}

/// A REQ (Relative Error Quantiles) sketch over `f64` or `f32` items.
///
/// Like a KLL sketch it answers rank, quantile, PMF and CDF queries in
/// bounded memory, but its error is *relative* to the rank, so it stays
/// accurate at one end of the distribution — the high ranks by default (see
/// [`RankAccuracy`]). Merging is a method ([`Self::merge`]); there is no
/// separate union type.
///
/// # Thread safety
///
/// `ReqSketch` is `Send` but **not** `Sync`: upstream's query methods are
/// logically read-only but build and cache a sorted view inside the sketch.
/// To share query access across threads, build a `ReqSortedView` once with
/// `sorted_view()` and share that (it is `Sync`).
///
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<apache_datasketches::req::ReqSketch<f64>>();
/// ```
///
/// ```
/// fn assert_send<T: Send>() {}
/// assert_send::<apache_datasketches::req::ReqSketch<f64>>();
/// ```
///
/// # Randomness
///
/// Compaction is randomized upstream and cannot be seeded, so once the
/// sketch is in estimation mode two sketches fed identical input can give
/// slightly different (equally valid) answers.
pub struct ReqSketch<T: ReqItem> {
    inner: UniquePtr<T::Sketch>,
}

// SAFETY: the shim owns its req_sketch outright and holds no thread-affine
// state. Upstream's compaction coin flips come from namespace-level
// `thread_local` engines (common_defs.hpp), not from the sketch, so a moved
// sketch simply draws from the new thread's engine. Deliberately not Sync:
// see the type-level docs.
unsafe impl<T: ReqItem> Send for ReqSketch<T> {}
```

then the `impl<T: ReqItem> ReqSketch<T>` block, written to match KLL's `sketch.rs` method for method (same doc style, same `require_non_empty` helper, same check ordering — emptiness first, then argument checks), with these REQ-specific methods and differences:

```rust
    /// Creates an empty sketch. `k` must be even and in `4..=1024`; larger `k`
    /// means smaller error and more memory. [`Default`] uses `k = 12` and
    /// [`RankAccuracy::High`].
    pub fn new(k: u16, accuracy: RankAccuracy) -> Result<Self, SketchError> {
        check_k(k)?;
        let inner = T::sketch_new(k, accuracy.is_high())
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Which end of the rank domain this sketch is accurate at.
    pub fn rank_accuracy(&self) -> RankAccuracy {
        RankAccuracy::from_hra(T::is_hra(&self.inner))
    }

    /// Merges `other` into this sketch. `other` may have a different `k`.
    ///
    /// Returns [`SketchError::InvalidConfig`], leaving this sketch unchanged,
    /// if the two sketches have different [`RankAccuracy`] or if the merged
    /// `n` would overflow `u64`.
    pub fn merge(&mut self, other: &ReqSketch<T>) -> Result<(), SketchError> {
        if self.rank_accuracy() != other.rank_accuracy() {
            return Err(SketchError::InvalidConfig(format!(
                "cannot merge a {:?}-accuracy REQ sketch into a {:?}-accuracy one",
                other.rank_accuracy(),
                self.rank_accuracy()
            )));
        }
        if self.get_n().checked_add(other.get_n()).is_none() {
            return Err(SketchError::InvalidConfig(format!(
                "REQ merge would overflow n ({} + {})",
                self.get_n(),
                other.get_n()
            )));
        }
        Ok(T::merge(self.inner.pin_mut(), &other.inner)?)
    }

    /// An approximate lower bound on the true normalized rank of an item
    /// whose estimated rank is `rank`, at `num_std_dev` (1, 2 or 3) standard
    /// deviations.
    pub fn get_rank_lower_bound(&self, rank: f64, num_std_dev: u8) -> Result<f64, SketchError> {
        self.require_non_empty()?;
        check_rank(rank)?;
        check_num_std_dev(num_std_dev)?;
        Ok(T::rank_lower_bound(&self.inner, rank, num_std_dev))
    }

    /// The upper-bound counterpart of [`Self::get_rank_lower_bound`].
    pub fn get_rank_upper_bound(&self, rank: f64, num_std_dev: u8) -> Result<f64, SketchError> {
        self.require_non_empty()?;
        check_rank(rank)?;
        check_num_std_dev(num_std_dev)?;
        Ok(T::rank_upper_bound(&self.inner, rank, num_std_dev))
    }

    /// An a-priori estimate of the relative standard error (in `[0, 1]`) at
    /// normalized `rank` for a sketch with this `k` and `accuracy` after `n`
    /// items. Upstream `get_RSE`.
    pub fn relative_standard_error(
        k: u16,
        rank: f64,
        accuracy: RankAccuracy,
        n: u64,
    ) -> Result<f64, SketchError> {
        check_k(k)?;
        check_rank(rank)?;
        Ok(T::rse(k, rank, accuracy.is_high(), n))
    }
```

Everything else — `deserialize` (doc: matching Java/Python `ReqSketch` for `f32`; empty bytes carry no item type), `update`, `is_empty`, `get_n`, `get_k`, `get_num_retained`, `is_estimation_mode` (doc: true once the sketch has more than one level, i.e. results are estimates), `get_min_item`, `get_max_item`, `get_quantile`, `get_rank`, `get_pmf`, `get_cdf`, `serialize`, `to_string_summary`, `iter`, `sorted_view`, `require_non_empty` — is KLL's code with `Kll` → `Req`. There is no `get_normalized_rank_error`, `normalized_rank_error_for_k` or `shim()`. Finish with:

```rust
impl<T: ReqItem> Clone for ReqSketch<T> {
    fn clone(&self) -> Self {
        Self {
            inner: T::clone_shim(&self.inner),
        }
    }
}

impl<T: ReqItem> Default for ReqSketch<T> {
    /// An empty sketch with `k = 12` and [`RankAccuracy::High`].
    fn default() -> Self {
        Self::new(DEFAULT_K, RankAccuracy::High).expect("DEFAULT_K is valid")
    }
}
```

- [ ] **Step 10: Write `sorted_view.rs` and `mod.rs`**

Create `apache-datasketches/src/req/sorted_view.rs` as KLL's `sorted_view.rs` with `Kll` → `Req`, `KllItem` → `ReqItem`, `SortedViewEntry` → `ReqSortedViewEntry`, and doc links pointing at `ReqSketch`. Keep the SAFETY comment's by-value-storage argument verbatim (with `ReqItem`).

Create `apache-datasketches/src/req/mod.rs`:

```rust
//! REQ (Relative Error Quantiles) sketch family: approximate ranks,
//! quantiles, PMF and CDF over a stream of `f64` or `f32` values, with error
//! relative to the rank — accurate at the high ranks by default.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::req::{RankAccuracy, ReqSketch};
//!
//! let mut latencies = ReqSketch::<f64>::new(12, RankAccuracy::High)?;
//! for ms in 1..=100_000 {
//!     latencies.update(f64::from(ms));
//! }
//! let p999 = latencies.get_quantile(0.999, true)?;
//! println!("p99.9 latency: {p999} ms");
//! # Ok(())
//! # }
//! ```
//!
//! - [`ReqSketch`] — the sketch, generic over [`ReqItem`] (`f64` or `f32`).
//!   Merge sketches with [`ReqSketch::merge`], which fails if their
//!   [`RankAccuracy`] differs.
//! - [`ReqSortedView`] / [`ReqSortedViewEntry`] — an immutable sorted
//!   snapshot of a sketch, from [`ReqSketch::sorted_view`]; `Sync`, so it is
//!   how to share query access across threads.

mod checks;
mod item;
mod sketch;
mod sorted_view;

pub use item::ReqItem;
pub use sketch::{RankAccuracy, ReqSketch};
pub use sorted_view::{ReqSortedView, ReqSortedViewEntry};
```

- [ ] **Step 11: Run the tests and doctests**

Run: `cargo test -p apache-datasketches --features req --test req_sketch_smoke_test --test req_sorted_view_test --test req_concurrency_test`
Expected: PASS.
Run: `cargo test -p apache-datasketches --features req --doc req`
Expected: PASS, including the `compile_fail` `!Sync` doctest.

- [ ] **Step 12: Lint, format, commit**

```bash
cargo clippy -p apache-datasketches --no-default-features --features req --all-targets -- -D warnings
rustfmt --edition 2021 apache-datasketches/src/lib.rs apache-datasketches/src/req/*.rs apache-datasketches/tests/req_common/mod.rs apache-datasketches/tests/req_sketch_smoke_test.rs apache-datasketches/tests/req_sorted_view_test.rs apache-datasketches/tests/req_concurrency_test.rs
git add apache-datasketches
git commit -m "feat(req): add the generic ReqSketch and ReqSortedView over f64 and f32"
```

---

### Task 4: Harden deserialization and add the fuzz regression test

This task is investigation plus implementation. Its acceptance criterion is concrete: the committed fuzz test passes, every crash or UB the investigation finds has a targeted regression test, and the existing suites still pass. Use a capable model.

**Files:**
- Modify: `apache-datasketches-sys/cpp/req/req_sketch_shim.cc`
- Modify (only if the audit finds a lower safe `n` limit): `apache-datasketches/src/req/sketch.rs`
- Modify: `apache-datasketches/src/req/sketch.rs` (`deserialize` rustdoc: list what is rejected)
- Modify: `apache-datasketches/Cargo.toml`
- Test: `apache-datasketches/tests/req_deserialize_fuzz_test.rs`, plus targeted cases added to `apache-datasketches/tests/req_sketch_smoke_test.rs`

**Interfaces:**
- Consumes: `ReqSketch` API (Task 3); the shim (Task 2).
- Produces: `validate_req_layout(rust::Slice<const uint8_t>)` (C++, anonymous namespace) called at the top of both `req_*_deserialize`; a post-decode NaN scan; no public API change.

**Wire format** (from upstream `req_sketch::serialize` and `req_compactor::serialize`, `req_sketch_impl.hpp` / `req_compactor_impl.hpp`; all little-endian):
- Header (8 bytes): `preamble_ints` u8 (4 if estimation mode i.e. `num_levels > 1`, else 2), `serial_version` u8 (= 1), `family` u8 (= 17), `flags` u8 (bit 2 `IS_EMPTY`, bit 3 `IS_HIGH_RANK`, bit 4 `RAW_ITEMS`, bit 5 `IS_LEVEL_ZERO_SORTED`; bits 0–1 reserved), `k` u16, `num_levels` u8, `num_raw_items` u8.
- If empty: nothing more.
- If `num_levels > 1`: `n` u64, then `min` and `max` items (`sizeof(T)` each).
- If `RAW_ITEMS` (written when `n <= 4`): `num_raw_items` items.
- Else, for each of `num_levels` compactors: `state` u64, `section_size_raw` f32, `lg_weight` u8, `num_sections` u8, 2 padding bytes, `num_items` u32, then `num_items` items.
- When `num_levels == 1`, upstream derives `n` from compactor 0's item count and scans for min/max — dereferencing `begin()` even when there are no items (UB).

- [ ] **Step 1: Write the targeted failing tests**

Add to `apache-datasketches/tests/req_sketch_smoke_test.rs` (for both types, via `for_both_types!`), building bytes by hand from the format above with a small helper in the test file:

1. **Non-empty, one level, zero items** — header `[2, 1, 17, flags = (1<<3), 12, 0, 1, 0]` then one compactor with `num_items = 0`: `deserialize` must return `Err(SketchError::Deserialization(_))`. (Today this is UB; it may crash the test binary — record what happens in the RED step.)
2. **Compactor `num_items` larger than the remaining bytes** (e.g. `u32::MAX`): `Err(Deserialization)`, without first allocating `num_items * sizeof(T)`.
3. **`num_levels > 1` with `n` inconsistent with the compactor weights** (sum over compactor `i` of `num_items << lg_weight` ≠ `n`): `Err(Deserialization)`.
4. **Odd `k`, `k < 4` and `k > 1024`** in otherwise-valid non-empty bytes: `Err(Deserialization)`.
5. **NaN item** in a raw-items sketch and in a compactor: `Err(Deserialization)`.
6. **Untampered round trips** for `n` in `{1, 4, 5, 12, 100, 10_000, 1_000_000}` and both accuracies still deserialize and then `merge` both ways and answer queries.

- [ ] **Step 2: Run them (RED)**

Run: `cargo test -p apache-datasketches --features req --test req_sketch_smoke_test`
Expected: cases 1–5 fail (or crash); case 6 passes. Record the observed failure for each in the report.

- [ ] **Step 3: Implement `validate_req_layout`**

In `req_sketch_shim.cc`, add a bounds-checked parser in the anonymous namespace (use a `read_at` helper with `memcpy` and an explicit length check, as `kll_sketch_shim.cc`'s `validate_kll_layout` does — read that function first and follow its style). It must reject (throw `std::invalid_argument` with a clear message) at least:

- buffer shorter than the 8-byte header; unknown flag bits set (bits 0–1 or 6–7);
- `k` odd, `< 4` or `> 1024` (any sketch, empty or not — an empty sketch with a bad `k` would otherwise be silently rounded by the constructor);
- non-empty with `num_levels == 0`;
- `RAW_ITEMS` set but `num_levels != 1`, or `num_raw_items == 0`, or `num_raw_items > 4`, or the remaining bytes ≠ `num_raw_items * sizeof(T)`;
- for each compactor: header truncated; `num_items` such that `num_items * sizeof(T)` exceeds the remaining bytes; `lg_weight` ≠ the compactor's level index (upstream's `grow()` assigns `lg_weight = level`); `num_sections == 0`; `section_size_raw` not finite or `< 1`;
- a single-level non-raw sketch whose compactor has zero items;
- `num_levels > 1` and `n` ≠ Σ `num_items << lg_weight` (overflow-checked; reject on overflow) — and `n == 0`;
- trailing bytes after the last compactor.

Call it first in both `req_doubles_deserialize` and `req_floats_deserialize`. After upstream `deserialize` succeeds, scan the decoded sketch's retained items plus min/max for NaN and throw if any (as KLL's shim does).

- [ ] **Step 4: Audit the remaining upstream paths**

Read `req_compactor_impl.hpp` (`merge`, `compact`, `ensure_space`, `grow`, `ensure_enough_sections`, `compute_compaction_range`, `promote_evens_or_odds`) and `req_sketch_impl.hpp` (`merge`, `compress`, `grow`, `update_max_nom_size`, `setup_sorted_view`). For each field the validator lets through (`state`, `section_size_raw`, `num_sections`, the sortedness flag), determine whether a crafted-but-consistent value can reach an abort, an unbounded loop, a huge allocation or UB in a later `merge`/`update`/query. For every such path: add the tightest validator check justified by upstream's own invariants (cite the upstream lines in a comment), plus a targeted regression test. Also determine whether `n_ += other.n_` in `merge` has a lower safe limit than `u64` overflow; if it does, add a named `MERGED_N_LIMIT` constant to `sketch.rs` (documented with the upstream lines, like KLL's) and make `merge` return `InvalidConfig` at it, with a test.

- [ ] **Step 5: Write the fuzz regression test**

Add to `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "req_deserialize_fuzz_test"
required-features = ["req"]
```

Create `apache-datasketches/tests/req_deserialize_fuzz_test.rs`. Requirements:

- A deterministic splitmix64 generator (copy the `next_u64` from KLL's `tests/kll_common/mod.rs` `Normal`), fixed seed.
- Seed corpus: genuine serialized sketches for both item types, both accuracies, and `n` ∈ {0, 1, 3, 4, 5, 12, 50, 1 000, 100 000}.
- `ITERATIONS: usize = 20_000` per item type. Each iteration picks a corpus entry and applies 1–4 random mutations: overwrite a random byte, flip a random bit, truncate at a random length, append a random byte, or overwrite a random aligned 1/2/4/8-byte field with a boundary value (`0`, `1`, `0xFF…`, `0x7F…`, `0x80…`).
- For each mutated buffer: if `ReqSketch::<T>::deserialize` returns `Ok(s)`, then — and every one of these must complete without panicking or aborting — `s.merge(&genuine)` and `genuine_clone.merge(&s)` (either may return `Err`), `s.get_quantile(0.5, true)`, `s.get_rank(x, true)`, `s.get_cdf(&[x], true)`, `s.sorted_view().iter().count()`, `s.iter().count()`, `ReqSketch::<T>::deserialize(&s.serialize())`, and `s.update(x)`.
- A run must finish in under 30 seconds in debug mode; lower `ITERATIONS` if needed and say so in the report.

- [ ] **Step 6: Run everything (GREEN)**

Run: `cargo test -p apache-datasketches --features req --release --test req_deserialize_fuzz_test --test req_sketch_smoke_test`
Run: `cargo test -p apache-datasketches --features req`
Run: `cargo test -p apache-datasketches-sys --features req`
Run: `cargo clippy -p apache-datasketches --no-default-features --features req --all-targets -- -D warnings`
Expected: all pass. Also run the fuzz test once in debug and report its duration.

- [ ] **Step 7: Document and commit**

Update `ReqSketch::deserialize`'s rustdoc to list what is rejected and note that sortedness is not validated (crafted-but-consistent bytes can give wrong answers, never memory unsafety), mirroring `KllSketch::deserialize`'s doc.

```bash
rustfmt --edition 2021 apache-datasketches/src/req/sketch.rs apache-datasketches/tests/req_deserialize_fuzz_test.rs apache-datasketches/tests/req_sketch_smoke_test.rs
git add apache-datasketches-sys/cpp/req apache-datasketches
git commit -m "fix(req): validate serialized bytes before deserializing; add a fuzz regression test"
```

---

### Task 5: Port the upstream REQ tests, validation and wire-format tests

**Files:**
- Create: `apache-datasketches/tests/req_sketch_test.rs`, `apache-datasketches/tests/req_validation_test.rs`, `apache-datasketches/tests/req_serialization_compat_test.rs`
- Modify: `apache-datasketches/Cargo.toml`

**Interfaces:**
- Consumes: full `ReqSketch`/`ReqSortedView` API; `req_common` helpers.

- [ ] **Step 1: Register targets**

Append `[[test]]` entries for `req_sketch_test`, `req_validation_test` and `req_serialization_compat_test`, each with `required-features = ["req"]`.

- [ ] **Step 2: Port `req_sketch_test.cpp`**

Create `apache-datasketches/tests/req_sketch_test.rs`. Start with a header comment listing the upstream file and every skipped `TEST_CASE` with its reason: the four `stream serialize-deserialize …` cases and `serialize deserialize stream and bytes equivalence` (no `std::istream` API), the three `type conversion …` cases (no conversion API), and the commented-out Java comparison. Port every other `TEST_CASE` in `vendor/datasketches-cpp/req/test/req_sketch_test.cpp` as one generic function run for both types via `for_both_types!`, preserving every `REQUIRE`:

| upstream `TEST_CASE` (line) | Rust fn |
|---|---|
| `req sketch: empty` (37) | `empty` |
| `req sketch: single value, lra` (56) | `single_value_lra` |
| `req sketch: repeated values` (85) | `repeated_values` |
| `req sketch: exact mode` (103) | `exact_mode` |
| `req sketch: estimation mode` (155) | `estimation_mode` |
| `req sketch: byte serialize-deserialize empty` (196) | `byte_serde_empty` |
| `req sketch: byte serialize-deserialize single item` (227) | `byte_serde_single_item` |
| `req sketch: byte serialize-deserialize exact mode` (262) | `byte_serde_exact_mode` |
| `req sketch: byte serialize-deserialize estimation mode` (299) | `byte_serde_estimation_mode` |
| `req sketch: merge into empty` (343) | `merge_into_empty` |
| `req sketch: merge` (358) | `merge` |
| `req sketch: merge multiple` (374) | `merge_multiple` |
| `req sketch: merge incompatible HRA and LRA` (394) | `merge_incompatible_accuracy` |
| `get_rank equivalence` (470) | `get_rank_equivalence` |

Translation rules: `req_sketch<float> s(12)` → `ReqSketch::<T>::new(12, RankAccuracy::High).unwrap()`; `(12, false)` → `RankAccuracy::Low`; `REQUIRE_THROWS_AS(x, std::runtime_error)` on an empty sketch → `matches!(…, Err(SketchError::EmptySketch))` (or `None` for min/max); `REQUIRE_THROWS_AS(merge…, std::invalid_argument)` → `matches!(…, Err(SketchError::InvalidConfig(_)))`; `get_rank(x)` → `get_rank(x, true)`, `get_quantile(r)` → `get_quantile(r, true)`; `get_CDF(arr, n)`/`get_PMF` → `get_cdf(&arr, true)`/`get_pmf`; `for (auto pair: sketch)` → `sketch.iter()`; float literals via `T::from_f64`; `Approx(x).margin(m)` → `(a - x).abs() <= m`. Where upstream asserts an exact value in **estimation mode**, check whether the value depends on the random compaction: if it does, assert the bound upstream's margin implies and add a one-line comment saying why; never silently drop an assertion.

- [ ] **Step 3: Validation test**

Create `apache-datasketches/tests/req_validation_test.rs` covering every row of the spec's error table for both types: `k` ∈ {0, 2, 3, 5, 13, 1023, 1025, 2048} rejected and {4, 6, 12, 1024} accepted (in `new` and `relative_standard_error`); rank ∈ {−0.0001, 1.0001, NaN, ∞} rejected in `get_quantile` (sketch and view), `get_rank_lower_bound`, `get_rank_upper_bound` and `relative_standard_error`; `num_std_dev` ∈ {0, 4, 255} rejected; NaN `get_rank` item rejected; split points `[NaN]`, `[1, NaN]`, `[2, 1]`, `[1, 1]` rejected in `get_pmf`/`get_cdf`, and `&[]` accepted giving `[1.0]` from `get_cdf`; `merge` with differing accuracy rejected in both directions; emptiness reported before a bad argument (`get_quantile(2.0, true)` on an empty sketch → `EmptySketch`). Use KLL's `tests/kll_validation_test.rs` as the style reference.

- [ ] **Step 4: Wire-format compatibility test**

Create `apache-datasketches/tests/req_serialization_compat_test.rs`. Golden bytes are derived by hand from upstream `serialize` (format summarized in Task 4) — **never** from the binding's own output. A fresh sketch's single compactor is created by `grow()` with `sorted = true`, and `append` clears it once a compactor holds more than one item; verify both against `req_compactor_impl.hpp` before trusting the flag values below.

```rust
//! Wire-format compatibility. C++ and Java share the REQ format, so bytes
//! matching upstream C++'s layout are readable by Java's `ReqSketch` (float)
//! and Python's float REQ sketch. Upstream ships no REQ `.sk` fixtures, so
//! these goldens are derived by hand from `req_sketch::serialize`.

use apache_datasketches::req::{RankAccuracy, ReqSketch};
use apache_datasketches::SketchError;

const FAMILY: u8 = 17;
const IS_EMPTY: u8 = 1 << 2;
const IS_HIGH_RANK: u8 = 1 << 3;
const RAW_ITEMS: u8 = 1 << 4;
const LEVEL_ZERO_SORTED: u8 = 1 << 5;

#[test]
fn golden_empty() {
    let high = [2, 1, FAMILY, IS_EMPTY | IS_HIGH_RANK | RAW_ITEMS | LEVEL_ZERO_SORTED, 12, 0, 0, 0];
    let low = [2, 1, FAMILY, IS_EMPTY | RAW_ITEMS | LEVEL_ZERO_SORTED, 12, 0, 0, 0];
    assert_eq!(ReqSketch::<f32>::default().serialize(), high);
    assert_eq!(ReqSketch::<f64>::default().serialize(), high);
    assert_eq!(ReqSketch::<f32>::new(12, RankAccuracy::Low).unwrap().serialize(), low);
}

#[test]
fn golden_single_item() {
    // n = 1 <= 4, so raw items; one item keeps level zero sorted.
    let header = [2u8, 1, FAMILY, IS_HIGH_RANK | RAW_ITEMS | LEVEL_ZERO_SORTED, 12, 0, 1, 1];
    let mut f = ReqSketch::<f32>::default();
    f.update(1.0);
    let expected: Vec<u8> = header.iter().copied().chain(1.0f32.to_le_bytes()).collect();
    assert_eq!(f.serialize(), expected);
    let mut d = ReqSketch::<f64>::default();
    d.update(1.0);
    let expected: Vec<u8> = header.iter().copied().chain(1.0f64.to_le_bytes()).collect();
    assert_eq!(d.serialize(), expected);
}
```

Add, with the same derivation discipline:
- `golden_raw_items_unsorted`: three items `3.0, 1.0, 2.0` (f32). Raw items are written in the compactor's storage order — for HRA, `append` fills from the end (`i = capacity - num_items - 1`), so derive the exact item order from `append` and `serialize`; level zero is no longer sorted.
- `golden_one_level_exact`: an `f32` sketch with 5 items `1.0..=5.0` (`n > 4`, so not raw, but one level): preamble 2, header, then one compactor block (`state`, `section_size_raw`, `lg_weight = 0`, `num_sections`, padding, `num_items = 5`, items). Derive `state`/`section_size_raw`/`num_sections` from the `req_compactor` constructor `grow()` calls (`section_size_raw = k`, `num_sections = INIT_NUM_SECTIONS = 3`, `state = 0` — verify).
- `empty_bytes_deserialize_as_either_type`.
- `non_empty_bytes_of_the_other_type_are_rejected` for `n` ∈ {1, 3, 5, 10 000}, both directions.
- `roundtrip_preserves_accuracy_and_k` for both accuracies and `k` ∈ {4, 12, 1024}.

If a golden assertion fails, re-derive from the upstream source and record the correction in the report.

- [ ] **Step 5: Run, lint, format, commit**

Run: `cargo test -p apache-datasketches --features req --release --test req_sketch_test --test req_validation_test --test req_serialization_compat_test` (and once without `--release`).
Run: `cargo clippy -p apache-datasketches --no-default-features --features req --all-targets -- -D warnings`

```bash
rustfmt --edition 2021 apache-datasketches/tests/req_sketch_test.rs apache-datasketches/tests/req_validation_test.rs apache-datasketches/tests/req_serialization_compat_test.rs
git add apache-datasketches
git commit -m "test(req): port the upstream REQ tests and pin the wire format"
```

---

### Task 6: Benchmark pair

**Files:**
- Create: `benches/cpp_reference/req_update.cc`, `apache-datasketches/examples/bench_req_update.rs`
- Modify: `apache-datasketches/Cargo.toml`, `benches/cpp_reference/run.sh`, `benches/cpp_reference/compare.sh`, `benches/cpp_reference/README.md`, `.github/workflows/ci.yml` (bench job comment)

**Interfaces:**
- Consumes: `ReqSketch<f64>`; `bench::report_fields` in `bench_common.h` (added for KLL).

- [ ] **Step 1: Confirm which fields are deterministic**

Before writing the harness, establish whether `get_num_retained()` (and therefore serialized size) is deterministic for a fixed input. Write a throwaway test (do not commit it) that builds two `ReqSketch<f64>` (k = 12, High) from the same 1 000 000 distinct items and from 1 000 000 items cycling over 1 024 keys, 20 times each, and compares `get_num_retained()` and `serialize().len()` across runs. Record the result in the report. If either varies, the harness prints only `n`, `min`, `max` (and omits `retained`/`bytes`), and Step 5's README text says so.

- [ ] **Step 2: C++ harness**

Create `benches/cpp_reference/req_update.cc` as `benches/cpp_reference/kll_update.cc` with: `#include "req_sketch.hpp"`; `constexpr uint16_t K = 12;` and the sketch type `datasketches::req_sketch<double>` constructed as `sketch_t(K, true)`; the file comment naming `bench_req_update.rs` and explaining the field cross-check (and, if Step 1 found them nondeterministic, without `retained`/`bytes`). Scenarios, constants (`HOT_KEY_SPACE`, `QUERY_CALLS`) and output format otherwise identical. The header line prints `k=%u items=%llu reps=%llu`.

- [ ] **Step 3: Rust harness**

Create `apache-datasketches/examples/bench_req_update.rs` as `apache-datasketches/examples/bench_kll_update.rs` with `apache_datasketches::req::{RankAccuracy, ReqSketch}`, `const K: u16 = 12;`, `build()` returning `ReqSketch::new(K, RankAccuracy::High)`, module docs naming `req_update.cc` and the run commands with `--features req`, and the same field set as the C++ side.

Register it in `apache-datasketches/Cargo.toml`:

```toml
[[example]]
name = "bench_req_update"
required-features = ["req"]
```

- [ ] **Step 4: Register the pair**

`run.sh`: add `-I"$vendor/req/include"` to `includes` and `req` to `benchmarks` after `kll`. `compare.sh`: add `"req:bench_req_update:req"` to `pairs` after the `kll` entry, extending the KLL comment to cover REQ.

Run: `./benches/cpp_reference/compare.sh 100000 --reps 2`
Expected: exit 0, five `req` rows, and `Estimates agree on every scenario`. A field mismatch means a parameter differs between the two files — fix the drift; never loosen `compare.py`.

- [ ] **Step 5: Document**

`benches/cpp_reference/README.md`: add `req_update.cc` | `examples/bench_req_update.rs` to the "What is here" table; change "six Rust counterparts" to "seven"; in "Keeping them in sync" extend the KLL exception sentence to say REQ follows the same scenario set (`k` rather than `lg_k`, no `str`, adds `quantile`); retitle "The KLL pair checks fields, not an estimate" to "The KLL and REQ pairs check fields, not an estimate" and add a paragraph for REQ (randomized coin flips, which fields are deterministic per Step 1, and that Rust `deser` pays for layout validation the C++ side does not). In `.github/workflows/ci.yml`'s `benches` comment, update the program counts ("six" → "seven") and mention REQ alongside KLL as comparing deterministic fields.

- [ ] **Step 6: Commit**

```bash
rustfmt --edition 2021 apache-datasketches/examples/bench_req_update.rs
git add benches apache-datasketches/examples/bench_req_update.rs apache-datasketches/Cargo.toml .github/workflows/ci.yml
git commit -m "bench(req): add the REQ harness pair, cross-checked on deterministic fields"
```

---

### Task 7: Example, docs, CI matrix and CHANGELOG

**Files:**
- Create: `apache-datasketches/examples/req.rs`
- Modify: `apache-datasketches/Cargo.toml`, `apache-datasketches/src/lib.rs`, `README.md`, `apache-datasketches/README.md`, `apache-datasketches-sys/README.md`, `AGENTS.md`, `.github/workflows/ci.yml`, `CHANGELOG.md`

**Interfaces:**
- Consumes: the full public REQ API.

- [ ] **Step 1: Example**

Create `apache-datasketches/examples/req.rs`:

```rust
//! Standalone demo of the REQ (Relative Error Quantiles) sketch.
//!
//! Run with: `cargo run -p apache-datasketches --example req --features req`

use apache_datasketches::req::{RankAccuracy, ReqSketch};

fn main() {
    // REQ keeps its error relative to the rank, so with high-rank accuracy the
    // extreme tail (p99.9, p99.99) stays accurate even though the sketch is
    // small. `k` (even, 4..=1024) trades memory for accuracy; 12 gives about
    // 1% relative error at 95% confidence.
    let mut latencies = ReqSketch::<f64>::new(12, RankAccuracy::High).expect("valid k");
    for i in 0..1_000_000u32 {
        // A heavy-tailed "latency" distribution, in milliseconds.
        let x = f64::from(i % 10_000) / 10_000.0;
        latencies.update(1.0 + 1.0 / (1.0 - x * 0.9999));
    }

    println!("requests seen: {}", latencies.get_n());
    for rank in [0.5, 0.99, 0.999, 0.9999] {
        let q = latencies.get_quantile(rank, true).unwrap();
        let lo = latencies.get_rank_lower_bound(rank, 2).unwrap();
        let hi = latencies.get_rank_upper_bound(rank, 2).unwrap();
        println!("p{:<6} = {q:>9.2} ms   (true rank in [{lo:.5}, {hi:.5}] at 2 std devs)", rank * 100.0);
    }

    // A-priori error estimate for planning a k.
    let rse = ReqSketch::<f64>::relative_standard_error(12, 0.999, RankAccuracy::High, 1_000_000)
        .unwrap();
    println!("expected relative standard error at p99.9 with k = 12: {rse:.4}");

    // Merging requires the same RankAccuracy.
    let mut other = ReqSketch::<f64>::new(12, RankAccuracy::High).expect("valid k");
    other.update(5_000.0);
    latencies.merge(&other).expect("same accuracy merges");
    let low = ReqSketch::<f64>::new(12, RankAccuracy::Low).expect("valid k");
    println!("merging a low-accuracy sketch fails: {}", latencies.merge(&low).is_err());

    // A sorted view answers the same queries and can be shared across threads.
    let view = std::sync::Arc::new(latencies.sorted_view());
    println!("sorted view holds {} retained items", view.len());

    // f32 bytes are compatible with Java's ReqSketch; f64 bytes are C++-only.
    let bytes = latencies.serialize();
    let restored = ReqSketch::<f64>::deserialize(&bytes).expect("valid bytes");
    println!("serialized to {} bytes; restored n = {}", bytes.len(), restored.get_n());
}
```

Register it (`[[example]] name = "req" required-features = ["req"]`, after the `kll` example). Run `cargo run -p apache-datasketches --example req --features req` and confirm it completes and prints `merging a low-accuracy sketch fails: true`.

- [ ] **Step 2: Crate docs**

`apache-datasketches/src/lib.rs`: "five" → "six" families; add a `req` bullet after `kll` (`` - `req` (feature `req`) — REQ relative-error quantiles sketches over `f64` or `f32`, accurate at one end of the rank domain: ranks, quantiles, PMF and CDF, merging, and a shareable sorted view. ``); add `/`req`` to the sidebar sentence.

- [ ] **Step 3: READMEs**

Match the structure the KLL docs use (read `apache-datasketches/README.md`'s "KLL sketches" section first):
- `apache-datasketches/README.md`: family count and list in the intro (re-wrap at ~80 columns); a "## REQ sketches" section after KLL's with the `req/mod.rs` doctest code as its snippet, a per-type bullet list (`ReqSketch<T>` with its key methods and `RankAccuracy`; `ReqSortedView<T>`/`ReqSortedViewEntry<T>`), a compatibility paragraph (`f32` ↔ Java/Python, `f64` C++-only; empty bytes carry no type; corrupt bytes rejected), a "what differs" list (relative error and `RankAccuracy`; `merge` returns `Result` and rejects mixed accuracy; `!Sync`; randomized compaction; empty-sketch errors and NaN handling), and the example's coverage; a families-list bullet `- [x] REQ — `req` feature (relative-error quantiles over `f64`/`f32`; merge, sorted view).`
- Root `README.md`: family count; `req` bullet after `kll` in "Sketch families"; `cargo run -p apache-datasketches --example req --features req` after the `kll` example line.
- `apache-datasketches-sys/README.md`: `req` bullet after `kll` (`raw bridge to the REQ quantiles sketch and its sorted view C++ types, for double and float items.`); a `features = ["req"]` toml snippet after the `kll` one; add `"req"` to the all-features snippet.

- [ ] **Step 4: AGENTS.md**

Family list and count ("six"); `cargo test -p apache-datasketches --features req`, the example command and `cargo run --release -p apache-datasketches --example bench_req_update --features req` in "Common commands"; `,req` in the all-families test line; `req/` in the vendored-modules list; and a short paragraph after KLL's: REQ mirrors KLL's template-shim/sealed-trait design with its own types (deliberately not shared, to keep the features independent), its shim validates serialized bytes (`validate_req_layout`) and the committed `req_deserialize_fuzz_test` guards that, `merge` returns `Result` because mixing `RankAccuracy` is a caller error, and its bench pair cross-checks deterministic fields like KLL's.

- [ ] **Step 5: CI matrix**

In `.github/workflows/ci.yml`'s `features` matrix add `- "--no-default-features --features req"` after the `kll` leg and change `# default: all five` to `# default: all six`.

- [ ] **Step 6: CHANGELOG**

Under `## [Unreleased]` → `### Added`, add before the existing KLL bullet:

```markdown
- **REQ quantiles sketch family** (`req` feature, on by default):
  `ReqSketch<T>` for `T = f64` or `f32` — relative-error rank, quantile, PMF
  and CDF queries that stay accurate at the high ranks (or the low ranks,
  with `RankAccuracy::Low`), rank bounds and an a-priori
  `relative_standard_error`, `merge`, serialization, `iter()`, and a
  shareable `ReqSortedView<T>`. `f32` bytes are compatible with Java's
  `ReqSketch` and Python's float REQ sketch; `f64` bytes are C++-only.

  Sharp edges, documented and tested: `merge` returns `Result` and rejects
  sketches with different `RankAccuracy` (or an overflowing `n`);
  `ReqSketch` is `Send` but not `Sync` — share a `ReqSortedView` instead;
  compaction is randomized and cannot be seeded; `k` must be even and in
  `4..=1024` (upstream silently rounds an odd `k`). `deserialize` validates
  the serialized layout before upstream reads it, so corrupt bytes return
  `SketchError::Deserialization`; a committed fuzz test guards this.
```

and under the existing `apache-datasketches-sys` bullet (or a new one) add `` `req` feature with the `req_doubles` and `req_floats` bridges. ``

- [ ] **Step 7: Full verification**

```bash
cargo build -p apache-datasketches --no-default-features --features req
cargo test --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo build -p apache-datasketches --no-default-features 2>&1 | grep -q "no sketch family is enabled"
cargo fmt --all -- --check
./benches/cpp_reference/compare.sh 100000 --reps 2
```

Expected: all succeed.

- [ ] **Step 8: Commit**

```bash
git add apache-datasketches README.md apache-datasketches-sys/README.md AGENTS.md .github/workflows/ci.yml CHANGELOG.md
git commit -m "docs(req): document the REQ family, add its example, CI leg and CHANGELOG entry"
```

- [ ] **Step 9: Hand back**

Stop. Versions are not bumped here: REQ ships in 0.3.0 if that is still unpublished, otherwise in a follow-up release decided by the maintainer.
