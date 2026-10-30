# Bloom Filter Rust Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Bloom filter family (approximate set membership) — `BloomFilterBuilder`, `BloomFilter`, a zero-copy `WrappedBloomFilter<'a>` and the sealed `BloomInput` trait — to `apache-datasketches`/`apache-datasketches-sys` behind a new `bloom` feature, binding datasketches-cpp 5.3.0's `bloom_filter`.

**Architecture:** One C++ shim file wraps upstream's (non-template) `bloom_filter` as `BloomFilterShim` (owned) and `WrappedBloomFilterShim` (read-only view over Rust-owned bytes); one cxx bridge declares them; the safe layer adds a builder with a fixed default seed, a lifetime-bound wrapped view, and a sealed input trait for set operations. Untrusted bytes are validated in the shim before upstream reads them.

**Tech Stack:** Rust 2021, `cxx`/`cxx-build` 1.x, vendored Apache DataSketches C++ 5.3.0, C++17.

**Spec:** `docs/superpowers/specs/2026-10-29-bloom-filter-rust-bindings-design.md` — read it before any task.

**Templates in the repo:** the CPC family is the closest single-type analog (`apache-datasketches-sys/cpp/cpc/cpc_sketch_shim.{h,cc}`, `apache-datasketches-sys/src/cpc_sketch.rs`, `apache-datasketches/src/cpc/`). Theta's sealed `ThetaInput` (`apache-datasketches/src/theta/input.rs`, `apache-datasketches-sys/src/theta_input.rs`) and `WrappedCompactThetaSketch<'a>` (`apache-datasketches/src/theta/wrapped.rs`) are the templates for `BloomInput` and the wrapped view. KLL/REQ's `validate_*_layout` in `apache-datasketches-sys/cpp/{kll,req}/*_shim.cc` and `tests/req_deserialize_fuzz_test.rs` are the templates for hardening.

## Global Constraints

> **Amended during Task 4 (supersedes the values below):** `MAX_FILTER_SIZE_BITS` is `(1 << 32) - 64` (2^32 − 64 bits), the largest capacity upstream can deserialize; the builder and `suggest_*` reject larger sizes with `InvalidConfig`. An empty image can claim at most 512 MiB (not ~2 GiB). `num_hashes` must be in `1..=65534` (65535 hangs upstream). `update_*` route through upstream `query_and_update` to keep the bits-set count exact. See the spec's upstream-facts section.

- Work on branch `feat/bloom` (already created; the spec is committed there).
- Commit messages: conventional prefix (`feat(bloom):`, `test(bloom):`, `fix(bloom):`, `docs(bloom):`, `bench(bloom):`, `chore(bloom):`). **No `Co-Authored-By` trailer and no AI attribution of any kind.** Do not push.
- Every public item in `apache-datasketches` needs a `///`/`//!` doc comment; `missing_docs` clean.
- cxx free-function and opaque-type names unique across the sys crate; free functions prefixed `bloom_`.
- Default seed: `DEFAULT_SEED = 9001` (upstream `datasketches::DEFAULT_SEED`), overridable with `.seed(u64)`.
- Builder validation in Rust (→ `InvalidConfig`): `max_distinct_items > 0`; `target_fpp` in `(0, 1)` and not NaN; `num_bits` in `1..=MAX_FILTER_SIZE_BITS` where `MAX_FILTER_SIZE_BITS = (i32::MAX as u64 - 32) * 8`; `num_hashes >= 1`.
- Errors: bad arguments and incompatible set operations → `InvalidConfig`; bad bytes (`deserialize`, `wrap`) → `Deserialization`; anything else → `Cpp`.
- `BloomFilter`: `Clone + Send + Sync`; `bits_used(&mut self)`. `WrappedBloomFilter<'a>`: `Send + Sync`, borrows its bytes, `bits_used(&self)`.
- Empty serialized images (24 bytes) may claim a capacity up to ~2 GiB; accept them as upstream does and document the allocation in `deserialize`/`wrap` rustdoc.
- The filter is deterministic given its seed: tests may assert exact values.
- Format Rust with `rustfmt --edition 2021 <file>`. No performance figures quoted anywhere.

## File map

| File | Status | Responsibility |
|---|---|---|
| `apache-datasketches-sys/vendor/datasketches-cpp/filters/include/*` | create (copy) | vendored upstream headers |
| `apache-datasketches-sys/vendor/README.md`, `tests/vendor_drift_test.rs`, `Cargo.toml`, `build.rs`, `src/lib.rs` | modify | feature, vendoring, registration |
| `apache-datasketches-sys/cpp/bloom/bloom_filter_shim.{h,cc}` | create | shims, free fns, (Task 4) validator |
| `apache-datasketches-sys/src/bloom_filter.rs` | create | cxx bridge |
| `apache-datasketches-sys/tests/bloom_filter_link_test.rs` | create | link test |
| `apache-datasketches/src/bloom/{mod,builder,filter,wrapped,input}.rs` | create | safe layer |
| `apache-datasketches/tests/bloom_*.rs` | create | tests |
| `apache-datasketches/examples/{bloom,bench_bloom_update}.rs`, `benches/cpp_reference/bloom_update.cc` | create | example, bench pair |
| `apache-datasketches/Cargo.toml`, `src/lib.rs`, READMEs, `AGENTS.md`, `CHANGELOG.md`, `.github/workflows/ci.yml`, bench scripts/README | modify | registration and docs |

---

### Task 1: Vendor `filters/include` and add the sys `bloom` feature

**Files:** `apache-datasketches-sys/vendor/datasketches-cpp/filters/include/` (copy); `apache-datasketches-sys/vendor/README.md`; `apache-datasketches-sys/tests/vendor_drift_test.rs`; `apache-datasketches-sys/Cargo.toml`

**Interfaces:** Produces sys feature `bloom`; headers `bloom_filter.hpp`, `bloom_filter_impl.hpp`, `bloom_filter_builder_impl.hpp`, `bit_array_ops.hpp` under `vendor/datasketches-cpp/filters/include`.

- [ ] **Step 1:** In `vendor_drift_test.rs` change `const MODULES: [&str; 7] = ["common", "hll", "cpc", "theta", "tuple", "kll", "req"];` to `const MODULES: [&str; 8] = ["common", "hll", "cpc", "theta", "tuple", "kll", "req", "filters"];`
- [ ] **Step 2:** Run `cargo test -p apache-datasketches-sys --test vendor_drift_test` — Expected: FAIL, `filters: missing from the vendored copy entirely`.
- [ ] **Step 3:** `mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/filters && cp -R vendor/datasketches-cpp/filters/include apache-datasketches-sys/vendor/datasketches-cpp/filters/include`
- [ ] **Step 4:** Re-run the drift test — Expected: PASS.
- [ ] **Step 5:** In `apache-datasketches-sys/vendor/README.md` add `filters/include` to the list of compiled headers and the matching `mkdir -p …/filters` and `cp -R vendor/datasketches-cpp/filters/include …/filters/include` lines to the recipe (after the `req` lines), and `+`filters/`` to the closing paragraph; wrap at ~80 columns.
- [ ] **Step 6:** In `apache-datasketches-sys/Cargo.toml` `[features]` add `bloom = []` after `req = []`. Run `cargo build -p apache-datasketches-sys --features bloom` — Expected: success.
- [ ] **Step 7:** Commit: `git add apache-datasketches-sys/vendor apache-datasketches-sys/tests/vendor_drift_test.rs apache-datasketches-sys/Cargo.toml && git commit -m "chore(bloom): vendor the filters headers and add the sys bloom feature"`

---

### Task 2: C++ shim, cxx bridge and link test

**Files:** Create `apache-datasketches-sys/cpp/bloom/bloom_filter_shim.h`, `apache-datasketches-sys/cpp/bloom/bloom_filter_shim.cc`, `apache-datasketches-sys/src/bloom_filter.rs`, `apache-datasketches-sys/tests/bloom_filter_link_test.rs`; modify `apache-datasketches-sys/src/lib.rs`, `apache-datasketches-sys/build.rs`, `apache-datasketches-sys/Cargo.toml`

**Interfaces:**
- Produces module `apache_datasketches_sys::bloom_filter::ffi` with opaque types `BloomFilterShim`, `WrappedBloomFilterShim`; free fns `bloom_new_by_accuracy(max_distinct_items: u64, target_fpp: f64, seed: u64) -> Result<UniquePtr<BloomFilterShim>>`, `bloom_new_by_size(num_bits: u64, num_hashes: u16, seed: u64) -> Result<UniquePtr<BloomFilterShim>>`, `bloom_deserialize(bytes: &[u8]) -> Result<UniquePtr<BloomFilterShim>>`, `bloom_wrap(bytes: &[u8]) -> Result<UniquePtr<WrappedBloomFilterShim>>`, `bloom_suggest_num_hashes(max_distinct_items: u64, num_filter_bits: u64) -> Result<u16>`, `bloom_suggest_num_hashes_for_fpp(target_fpp: f64) -> Result<u16>`, `bloom_suggest_num_filter_bits(max_distinct_items: u64, target_fpp: f64) -> Result<u64>`; methods listed in the bridge below.

- [ ] **Step 1: Write the link test** — create `apache-datasketches-sys/tests/bloom_filter_link_test.rs`:

```rust
#![cfg(feature = "bloom")]

use apache_datasketches_sys::bloom_filter::ffi;

const SEED: u64 = 9001;

#[test]
fn construct_update_query_roundtrip() {
    let mut f = ffi::bloom_new_by_accuracy(1000, 0.01, SEED).unwrap();
    assert!(f.is_empty());
    for i in 0..100u64 {
        f.pin_mut().update_u64(i);
    }
    f.pin_mut().update_str("hello");
    assert!(!f.is_empty());
    assert!(f.query_u64(42));
    assert!(f.query_str("hello"));
    assert_eq!(f.get_seed(), SEED);
    assert!(f.get_num_hashes() >= 1);
    assert_eq!(f.get_capacity() % 64, 0);
    let used = f.pin_mut().get_bits_used();
    assert!(used > 0 && used <= f.get_capacity());

    let bytes = f.serialize();
    let g = ffi::bloom_deserialize(bytes.as_slice()).unwrap();
    assert!(g.query_u64(42));
    let w = ffi::bloom_wrap(bytes.as_slice()).unwrap();
    assert!(w.query_u64(42));
    assert!(w.query_str("hello"));
    assert_eq!(w.get_bits_used(), used);
    assert!(f.is_compatible_with_wrapped(&w));

    let mut h = ffi::bloom_new_by_accuracy(1000, 0.01, SEED).unwrap();
    h.pin_mut().union_with_filter(&f).unwrap();
    assert!(h.query_u64(42));
    h.pin_mut().intersect_with_wrapped(&w).unwrap();
    assert!(h.query_u64(42));
    let c = h.clone_shim();
    assert!(c.query_u64(42));
}

#[test]
fn incompatible_set_operation_returns_err() {
    let mut a = ffi::bloom_new_by_size(1024, 3, SEED).unwrap();
    let b = ffi::bloom_new_by_size(1024, 3, SEED + 1).unwrap();
    assert!(!a.is_compatible_with_filter(&b));
    assert!(a.pin_mut().union_with_filter(&b).is_err());
}

#[test]
fn invalid_args_return_err() {
    assert!(ffi::bloom_new_by_size(0, 3, SEED).is_err());
    assert!(ffi::bloom_new_by_size(64, 0, SEED).is_err());
    assert!(ffi::bloom_new_by_accuracy(0, 0.01, SEED).is_err());
    assert!(ffi::bloom_deserialize(&[1, 2, 3]).is_err());
    assert!(ffi::bloom_wrap(&[1, 2, 3]).is_err());
}

#[test]
fn suggestions() {
    assert!(ffi::bloom_suggest_num_hashes(1000, 9586).unwrap() >= 1);
    assert!(ffi::bloom_suggest_num_hashes_for_fpp(0.01).unwrap() >= 1);
    assert!(ffi::bloom_suggest_num_filter_bits(1000, 0.01).unwrap() > 1000);
}
```

Append to `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "bloom_filter_link_test"
required-features = ["bloom"]
```

- [ ] **Step 2:** Run `cargo test -p apache-datasketches-sys --features bloom --test bloom_filter_link_test` — Expected: FAIL to compile (`could not find bloom_filter`).

- [ ] **Step 3: Shim header** — create `apache-datasketches-sys/cpp/bloom/bloom_filter_shim.h`:

```cpp
#pragma once
#include <cstddef>
#include <cstdint>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "bloom_filter.hpp"

namespace apache_datasketches_rs {

class WrappedBloomFilterShim;

// Owns a datasketches::bloom_filter.
class BloomFilterShim {
public:
  explicit BloomFilterShim(datasketches::bloom_filter filter);

  std::unique_ptr<BloomFilterShim> clone_shim() const;

  void update_u64(uint64_t value);
  bool query_u64(uint64_t value) const;
  bool query_and_update_u64(uint64_t value);
  void update_i64(int64_t value);
  bool query_i64(int64_t value) const;
  bool query_and_update_i64(int64_t value);
  void update_u32(uint32_t value);
  bool query_u32(uint32_t value) const;
  bool query_and_update_u32(uint32_t value);
  void update_i32(int32_t value);
  bool query_i32(int32_t value) const;
  bool query_and_update_i32(int32_t value);
  void update_u16(uint16_t value);
  bool query_u16(uint16_t value) const;
  bool query_and_update_u16(uint16_t value);
  void update_i16(int16_t value);
  bool query_i16(int16_t value) const;
  bool query_and_update_i16(int16_t value);
  void update_u8(uint8_t value);
  bool query_u8(uint8_t value) const;
  bool query_and_update_u8(uint8_t value);
  void update_i8(int8_t value);
  bool query_i8(int8_t value) const;
  bool query_and_update_i8(int8_t value);
  void update_f64(double value);
  bool query_f64(double value) const;
  bool query_and_update_f64(double value);
  void update_f32(float value);
  bool query_f32(float value) const;
  bool query_and_update_f32(float value);
  void update_str(rust::Str value);
  bool query_str(rust::Str value) const;
  bool query_and_update_str(rust::Str value);
  void update_bytes(rust::Slice<const uint8_t> value);
  bool query_bytes(rust::Slice<const uint8_t> value) const;
  bool query_and_update_bytes(rust::Slice<const uint8_t> value);

  void union_with_filter(const BloomFilterShim& other);
  void union_with_wrapped(const WrappedBloomFilterShim& other);
  void intersect_with_filter(const BloomFilterShim& other);
  void intersect_with_wrapped(const WrappedBloomFilterShim& other);
  bool is_compatible_with_filter(const BloomFilterShim& other) const;
  bool is_compatible_with_wrapped(const WrappedBloomFilterShim& other) const;
  void invert();
  void reset();

  bool is_empty() const;
  uint64_t get_capacity() const;
  uint16_t get_num_hashes() const;
  uint64_t get_seed() const;
  uint64_t get_bits_used();  // not const upstream: recounts and caches after updates

  std::unique_ptr<std::vector<uint8_t>> serialize() const;
  rust::String to_string_summary() const;

  const datasketches::bloom_filter& inner() const { return filter_; }

private:
  datasketches::bloom_filter filter_;
};

// A read-only bloom_filter wrapping bytes owned by Rust. The safe layer's
// lifetime guarantees the bytes outlive this object. A read-only wrapped
// filter cannot be updated, so it is never "dirty" and get_bits_used never
// writes its cache — which is why it is const here.
class WrappedBloomFilterShim {
public:
  explicit WrappedBloomFilterShim(datasketches::bloom_filter filter);

  bool query_u64(uint64_t value) const;
  bool query_i64(int64_t value) const;
  bool query_u32(uint32_t value) const;
  bool query_i32(int32_t value) const;
  bool query_u16(uint16_t value) const;
  bool query_i16(int16_t value) const;
  bool query_u8(uint8_t value) const;
  bool query_i8(int8_t value) const;
  bool query_f64(double value) const;
  bool query_f32(float value) const;
  bool query_str(rust::Str value) const;
  bool query_bytes(rust::Slice<const uint8_t> value) const;

  bool is_compatible_with_filter(const BloomFilterShim& other) const;
  bool is_compatible_with_wrapped(const WrappedBloomFilterShim& other) const;

  bool is_empty() const;
  uint64_t get_capacity() const;
  uint16_t get_num_hashes() const;
  uint64_t get_seed() const;
  uint64_t get_bits_used() const;

  std::unique_ptr<BloomFilterShim> to_owned() const;
  rust::String to_string_summary() const;

  const datasketches::bloom_filter& inner() const { return filter_; }

private:
  datasketches::bloom_filter filter_;
};

std::unique_ptr<BloomFilterShim> bloom_new_by_accuracy(uint64_t max_distinct_items, double target_fpp, uint64_t seed);
std::unique_ptr<BloomFilterShim> bloom_new_by_size(uint64_t num_bits, uint16_t num_hashes, uint64_t seed);
std::unique_ptr<BloomFilterShim> bloom_deserialize(rust::Slice<const uint8_t> bytes);
std::unique_ptr<WrappedBloomFilterShim> bloom_wrap(rust::Slice<const uint8_t> bytes);
uint16_t bloom_suggest_num_hashes(uint64_t max_distinct_items, uint64_t num_filter_bits);
uint16_t bloom_suggest_num_hashes_for_fpp(double target_fpp);
uint64_t bloom_suggest_num_filter_bits(uint64_t max_distinct_items, double target_fpp);

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Shim implementation** — create `apache-datasketches-sys/cpp/bloom/bloom_filter_shim.cc`:

```cpp
#include "bloom_filter_shim.h"

#include <string>
#include <utility>

namespace apache_datasketches_rs {

BloomFilterShim::BloomFilterShim(datasketches::bloom_filter filter) : filter_(std::move(filter)) {}

std::unique_ptr<BloomFilterShim> BloomFilterShim::clone_shim() const {
  return std::make_unique<BloomFilterShim>(filter_);
}

void BloomFilterShim::update_u64(uint64_t value) { filter_.update(value); }
bool BloomFilterShim::query_u64(uint64_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_u64(uint64_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_i64(int64_t value) { filter_.update(value); }
bool BloomFilterShim::query_i64(int64_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_i64(int64_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_u32(uint32_t value) { filter_.update(value); }
bool BloomFilterShim::query_u32(uint32_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_u32(uint32_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_i32(int32_t value) { filter_.update(value); }
bool BloomFilterShim::query_i32(int32_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_i32(int32_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_u16(uint16_t value) { filter_.update(value); }
bool BloomFilterShim::query_u16(uint16_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_u16(uint16_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_i16(int16_t value) { filter_.update(value); }
bool BloomFilterShim::query_i16(int16_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_i16(int16_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_u8(uint8_t value) { filter_.update(value); }
bool BloomFilterShim::query_u8(uint8_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_u8(uint8_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_i8(int8_t value) { filter_.update(value); }
bool BloomFilterShim::query_i8(int8_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_i8(int8_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_f64(double value) { filter_.update(value); }
bool BloomFilterShim::query_f64(double value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_f64(double value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_f32(float value) { filter_.update(value); }
bool BloomFilterShim::query_f32(float value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_f32(float value) { return filter_.query_and_update(value); }

// Strings forward the borrowed bytes to the (data, length) overload. The
// empty guard is replicated because upstream's std::string overloads return
// early on an empty string (bloom_filter_impl.hpp update/query/
// query_and_update(const std::string&)) and the (data, length) overload
// would hash it instead.
void BloomFilterShim::update_str(rust::Str value) {
  if (value.empty()) return;
  filter_.update(value.data(), value.size());
}
bool BloomFilterShim::query_str(rust::Str value) const {
  if (value.empty()) return false;
  return filter_.query(value.data(), value.size());
}
bool BloomFilterShim::query_and_update_str(rust::Str value) {
  if (value.empty()) return false;
  return filter_.query_and_update(value.data(), value.size());
}
void BloomFilterShim::update_bytes(rust::Slice<const uint8_t> value) {
  filter_.update(value.data(), value.size());
}
bool BloomFilterShim::query_bytes(rust::Slice<const uint8_t> value) const {
  return filter_.query(value.data(), value.size());
}
bool BloomFilterShim::query_and_update_bytes(rust::Slice<const uint8_t> value) {
  return filter_.query_and_update(value.data(), value.size());
}

void BloomFilterShim::union_with_filter(const BloomFilterShim& other) { filter_.union_with(other.inner()); }
void BloomFilterShim::union_with_wrapped(const WrappedBloomFilterShim& other) { filter_.union_with(other.inner()); }
void BloomFilterShim::intersect_with_filter(const BloomFilterShim& other) { filter_.intersect(other.inner()); }
void BloomFilterShim::intersect_with_wrapped(const WrappedBloomFilterShim& other) { filter_.intersect(other.inner()); }
bool BloomFilterShim::is_compatible_with_filter(const BloomFilterShim& other) const { return filter_.is_compatible(other.inner()); }
bool BloomFilterShim::is_compatible_with_wrapped(const WrappedBloomFilterShim& other) const { return filter_.is_compatible(other.inner()); }
void BloomFilterShim::invert() { filter_.invert(); }
void BloomFilterShim::reset() { filter_.reset(); }

bool BloomFilterShim::is_empty() const { return filter_.is_empty(); }
uint64_t BloomFilterShim::get_capacity() const { return filter_.get_capacity(); }
uint16_t BloomFilterShim::get_num_hashes() const { return filter_.get_num_hashes(); }
uint64_t BloomFilterShim::get_seed() const { return filter_.get_seed(); }
uint64_t BloomFilterShim::get_bits_used() { return filter_.get_bits_used(); }

// Whole buffer handed over at once; see cpc_sketch_shim.cc for why not a
// rust::Vec built element by element.
std::unique_ptr<std::vector<uint8_t>> BloomFilterShim::serialize() const {
  return std::make_unique<std::vector<uint8_t>>(filter_.serialize());
}

rust::String BloomFilterShim::to_string_summary() const {
  return rust::String(std::string(filter_.to_string().c_str()));
}

WrappedBloomFilterShim::WrappedBloomFilterShim(datasketches::bloom_filter filter) : filter_(std::move(filter)) {}

bool WrappedBloomFilterShim::query_u64(uint64_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_i64(int64_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_u32(uint32_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_i32(int32_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_u16(uint16_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_i16(int16_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_u8(uint8_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_i8(int8_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_f64(double value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_f32(float value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_str(rust::Str value) const {
  if (value.empty()) return false;
  return filter_.query(value.data(), value.size());
}
bool WrappedBloomFilterShim::query_bytes(rust::Slice<const uint8_t> value) const {
  return filter_.query(value.data(), value.size());
}
bool WrappedBloomFilterShim::is_compatible_with_filter(const BloomFilterShim& other) const { return filter_.is_compatible(other.inner()); }
bool WrappedBloomFilterShim::is_compatible_with_wrapped(const WrappedBloomFilterShim& other) const { return filter_.is_compatible(other.inner()); }
bool WrappedBloomFilterShim::is_empty() const { return filter_.is_empty(); }
uint64_t WrappedBloomFilterShim::get_capacity() const { return filter_.get_capacity(); }
uint16_t WrappedBloomFilterShim::get_num_hashes() const { return filter_.get_num_hashes(); }
uint64_t WrappedBloomFilterShim::get_seed() const { return filter_.get_seed(); }
// Read-only wrapped filters are never dirty, so upstream's non-const
// get_bits_used returns the cached count without writing; the const_cast is
// therefore a read.
uint64_t WrappedBloomFilterShim::get_bits_used() const {
  return const_cast<datasketches::bloom_filter&>(filter_).get_bits_used();
}

std::unique_ptr<BloomFilterShim> WrappedBloomFilterShim::to_owned() const {
  const auto bytes = filter_.serialize();
  return std::make_unique<BloomFilterShim>(datasketches::bloom_filter::deserialize(bytes.data(), bytes.size()));
}

rust::String WrappedBloomFilterShim::to_string_summary() const {
  return rust::String(std::string(filter_.to_string().c_str()));
}

std::unique_ptr<BloomFilterShim> bloom_new_by_accuracy(uint64_t max_distinct_items, double target_fpp, uint64_t seed) {
  return std::make_unique<BloomFilterShim>(
      datasketches::bloom_filter::builder::create_by_accuracy(max_distinct_items, target_fpp, seed));
}

std::unique_ptr<BloomFilterShim> bloom_new_by_size(uint64_t num_bits, uint16_t num_hashes, uint64_t seed) {
  return std::make_unique<BloomFilterShim>(
      datasketches::bloom_filter::builder::create_by_size(num_bits, num_hashes, seed));
}

std::unique_ptr<BloomFilterShim> bloom_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<BloomFilterShim>(datasketches::bloom_filter::deserialize(bytes.data(), bytes.size()));
}

std::unique_ptr<WrappedBloomFilterShim> bloom_wrap(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<WrappedBloomFilterShim>(datasketches::bloom_filter::wrap(bytes.data(), bytes.size()));
}

uint16_t bloom_suggest_num_hashes(uint64_t max_distinct_items, uint64_t num_filter_bits) {
  return datasketches::bloom_filter::builder::suggest_num_hashes(max_distinct_items, num_filter_bits);
}

uint16_t bloom_suggest_num_hashes_for_fpp(double target_fpp) {
  return datasketches::bloom_filter::builder::suggest_num_hashes(target_fpp);
}

uint64_t bloom_suggest_num_filter_bits(uint64_t max_distinct_items, double target_fpp) {
  return datasketches::bloom_filter::builder::suggest_num_filter_bits(max_distinct_items, target_fpp);
}

} // namespace apache_datasketches_rs
```

Notes: `bloom_filter::wrap` returns `const bloom_filter`; `std::move` of a const value copies — if upstream's copy constructor of a wrapped filter deep-copies the bit array (making it owned), the view would no longer be zero-copy. Check `bloom_filter_alloc(const bloom_filter_alloc&)` in `bloom_filter_impl.hpp`: if it copies, construct the shim member directly from the `wrap(...)` call (e.g. via a private constructor taking the bytes and calling `wrap` in the member initializer, which uses guaranteed copy elision of the prvalue) so no copy occurs, and verify zero-copy in a test by checking that `get_bits_used()`/queries reflect the bytes without allocation (or simply that construction does not copy, by code reading). Record what you found in the report.

- [ ] **Step 5: Bridge** — create `apache-datasketches-sys/src/bloom_filter.rs`:

```rust
//! cxx bridge for datasketches::bloom_filter (owned) and its read-only
//! wrapped view. Free functions carry the `bloom_` prefix because cxx keys
//! their symbols on name alone (see AGENTS.md).

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("bloom_filter_shim.h");

        type BloomFilterShim;
        type WrappedBloomFilterShim;

        fn bloom_new_by_accuracy(
            max_distinct_items: u64,
            target_fpp: f64,
            seed: u64,
        ) -> Result<UniquePtr<BloomFilterShim>>;
        fn bloom_new_by_size(num_bits: u64, num_hashes: u16, seed: u64)
            -> Result<UniquePtr<BloomFilterShim>>;
        fn bloom_deserialize(bytes: &[u8]) -> Result<UniquePtr<BloomFilterShim>>;
        fn bloom_wrap(bytes: &[u8]) -> Result<UniquePtr<WrappedBloomFilterShim>>;
        fn bloom_suggest_num_hashes(max_distinct_items: u64, num_filter_bits: u64) -> Result<u16>;
        fn bloom_suggest_num_hashes_for_fpp(target_fpp: f64) -> Result<u16>;
        fn bloom_suggest_num_filter_bits(max_distinct_items: u64, target_fpp: f64) -> Result<u64>;

        fn clone_shim(self: &BloomFilterShim) -> UniquePtr<BloomFilterShim>;
        fn update_u64(self: Pin<&mut BloomFilterShim>, value: u64);
        fn query_u64(self: &BloomFilterShim, value: u64) -> bool;
        fn query_and_update_u64(self: Pin<&mut BloomFilterShim>, value: u64) -> bool;
        fn update_i64(self: Pin<&mut BloomFilterShim>, value: i64);
        fn query_i64(self: &BloomFilterShim, value: i64) -> bool;
        fn query_and_update_i64(self: Pin<&mut BloomFilterShim>, value: i64) -> bool;
        fn update_u32(self: Pin<&mut BloomFilterShim>, value: u32);
        fn query_u32(self: &BloomFilterShim, value: u32) -> bool;
        fn query_and_update_u32(self: Pin<&mut BloomFilterShim>, value: u32) -> bool;
        fn update_i32(self: Pin<&mut BloomFilterShim>, value: i32);
        fn query_i32(self: &BloomFilterShim, value: i32) -> bool;
        fn query_and_update_i32(self: Pin<&mut BloomFilterShim>, value: i32) -> bool;
        fn update_u16(self: Pin<&mut BloomFilterShim>, value: u16);
        fn query_u16(self: &BloomFilterShim, value: u16) -> bool;
        fn query_and_update_u16(self: Pin<&mut BloomFilterShim>, value: u16) -> bool;
        fn update_i16(self: Pin<&mut BloomFilterShim>, value: i16);
        fn query_i16(self: &BloomFilterShim, value: i16) -> bool;
        fn query_and_update_i16(self: Pin<&mut BloomFilterShim>, value: i16) -> bool;
        fn update_u8(self: Pin<&mut BloomFilterShim>, value: u8);
        fn query_u8(self: &BloomFilterShim, value: u8) -> bool;
        fn query_and_update_u8(self: Pin<&mut BloomFilterShim>, value: u8) -> bool;
        fn update_i8(self: Pin<&mut BloomFilterShim>, value: i8);
        fn query_i8(self: &BloomFilterShim, value: i8) -> bool;
        fn query_and_update_i8(self: Pin<&mut BloomFilterShim>, value: i8) -> bool;
        fn update_f64(self: Pin<&mut BloomFilterShim>, value: f64);
        fn query_f64(self: &BloomFilterShim, value: f64) -> bool;
        fn query_and_update_f64(self: Pin<&mut BloomFilterShim>, value: f64) -> bool;
        fn update_f32(self: Pin<&mut BloomFilterShim>, value: f32);
        fn query_f32(self: &BloomFilterShim, value: f32) -> bool;
        fn query_and_update_f32(self: Pin<&mut BloomFilterShim>, value: f32) -> bool;
        fn update_str(self: Pin<&mut BloomFilterShim>, value: &str);
        fn query_str(self: &BloomFilterShim, value: &str) -> bool;
        fn query_and_update_str(self: Pin<&mut BloomFilterShim>, value: &str) -> bool;
        fn update_bytes(self: Pin<&mut BloomFilterShim>, value: &[u8]);
        fn query_bytes(self: &BloomFilterShim, value: &[u8]) -> bool;
        fn query_and_update_bytes(self: Pin<&mut BloomFilterShim>, value: &[u8]) -> bool;

        fn union_with_filter(self: Pin<&mut BloomFilterShim>, other: &BloomFilterShim) -> Result<()>;
        fn union_with_wrapped(self: Pin<&mut BloomFilterShim>, other: &WrappedBloomFilterShim)
            -> Result<()>;
        fn intersect_with_filter(self: Pin<&mut BloomFilterShim>, other: &BloomFilterShim)
            -> Result<()>;
        fn intersect_with_wrapped(
            self: Pin<&mut BloomFilterShim>,
            other: &WrappedBloomFilterShim,
        ) -> Result<()>;
        fn is_compatible_with_filter(self: &BloomFilterShim, other: &BloomFilterShim) -> bool;
        fn is_compatible_with_wrapped(self: &BloomFilterShim, other: &WrappedBloomFilterShim) -> bool;
        fn invert(self: Pin<&mut BloomFilterShim>);
        fn reset(self: Pin<&mut BloomFilterShim>);

        fn is_empty(self: &BloomFilterShim) -> bool;
        fn get_capacity(self: &BloomFilterShim) -> u64;
        fn get_num_hashes(self: &BloomFilterShim) -> u16;
        fn get_seed(self: &BloomFilterShim) -> u64;
        fn get_bits_used(self: Pin<&mut BloomFilterShim>) -> u64;
        fn serialize(self: &BloomFilterShim) -> UniquePtr<CxxVector<u8>>;
        fn to_string_summary(self: &BloomFilterShim) -> String;

        fn query_u64(self: &WrappedBloomFilterShim, value: u64) -> bool;
        fn query_i64(self: &WrappedBloomFilterShim, value: i64) -> bool;
        fn query_u32(self: &WrappedBloomFilterShim, value: u32) -> bool;
        fn query_i32(self: &WrappedBloomFilterShim, value: i32) -> bool;
        fn query_u16(self: &WrappedBloomFilterShim, value: u16) -> bool;
        fn query_i16(self: &WrappedBloomFilterShim, value: i16) -> bool;
        fn query_u8(self: &WrappedBloomFilterShim, value: u8) -> bool;
        fn query_i8(self: &WrappedBloomFilterShim, value: i8) -> bool;
        fn query_f64(self: &WrappedBloomFilterShim, value: f64) -> bool;
        fn query_f32(self: &WrappedBloomFilterShim, value: f32) -> bool;
        fn query_str(self: &WrappedBloomFilterShim, value: &str) -> bool;
        fn query_bytes(self: &WrappedBloomFilterShim, value: &[u8]) -> bool;
        fn is_compatible_with_filter(self: &WrappedBloomFilterShim, other: &BloomFilterShim) -> bool;
        fn is_compatible_with_wrapped(self: &WrappedBloomFilterShim, other: &WrappedBloomFilterShim)
            -> bool;
        fn is_empty(self: &WrappedBloomFilterShim) -> bool;
        fn get_capacity(self: &WrappedBloomFilterShim) -> u64;
        fn get_num_hashes(self: &WrappedBloomFilterShim) -> u16;
        fn get_seed(self: &WrappedBloomFilterShim) -> u64;
        fn get_bits_used(self: &WrappedBloomFilterShim) -> u64;
        fn to_owned(self: &WrappedBloomFilterShim) -> Result<UniquePtr<BloomFilterShim>>;
        fn to_string_summary(self: &WrappedBloomFilterShim) -> String;
    }
}
```

Also write the shim's `to_owned` as declared (`Result` on the bridge because it deserializes).

- [ ] **Step 6:** Register: append to `apache-datasketches-sys/src/lib.rs` `#[cfg(feature = "bloom")] pub mod bloom_filter;`. In `build.rs`, mirroring the `req` entries: a `bloom` bridges block (`src/bloom_filter.rs`, `require_exists`), `.include(vendor_dir.join("filters/include"))` and `.include("cpp/bloom")`, a `bloom` `.cc` block (`cpp/bloom/bloom_filter_shim.cc`), and `rerun-if-changed` lines for the three new files.
- [ ] **Step 7:** Run `cargo test -p apache-datasketches-sys --features bloom` — Expected: PASS. Run `cargo build -p apache-datasketches-sys --all-features` — Expected: success.
- [ ] **Step 8:** `rustfmt --edition 2021` the new Rust files and `build.rs`/`lib.rs`; commit `git add apache-datasketches-sys && git commit -m "feat(bloom): add the Bloom filter C++ shim and cxx bridge"`

---

### Task 3: Safe layer — builder, `BloomFilter`, `WrappedBloomFilter`, `BloomInput`

**Files:** Create `apache-datasketches/src/bloom/{mod,builder,filter,wrapped,input}.rs`, `apache-datasketches/tests/bloom_filter_smoke_test.rs`, `apache-datasketches/tests/bloom_wrapped_test.rs`, `apache-datasketches/tests/bloom_concurrency_test.rs`; modify `apache-datasketches/Cargo.toml`, `apache-datasketches/src/lib.rs`

**Interfaces:**
- Consumes: `apache_datasketches_sys::bloom_filter::ffi` (Task 2).
- Produces: `bloom::{DEFAULT_SEED, BloomFilterBuilder, BloomFilter, WrappedBloomFilter, BloomInput}` with exactly the signatures in the spec's Public API (method names `update_<t>`, `query_<t>`, `query_and_update_<t>` for `t` in `u64 i64 u32 i32 u16 i16 u8 i8 f64 f32 str bytes`; `union_with`, `intersect`, `is_compatible`, `invert`, `reset`, `is_empty`, `capacity`, `num_hashes`, `seed`, `bits_used`, `serialize`, `deserialize`, `to_string_summary`; `WrappedBloomFilter::{wrap, to_owned, …}`; builder `by_accuracy`, `by_size`, `seed`, `build`, `suggest_num_hashes`, `suggest_num_hashes_for_fpp`, `suggest_num_filter_bits`); crate-private `const MAX_FILTER_SIZE_BITS: u64 = (i32::MAX as u64 - 32) * 8`.

- [ ] **Step 1: Feature and targets.** `apache-datasketches/Cargo.toml`: add `"bloom"` to `default` and the docs.rs features list, `bloom = ["apache-datasketches-sys/bloom"]`, and `[[test]]` entries (`required-features = ["bloom"]`) for `bloom_filter_smoke_test`, `bloom_wrapped_test`, `bloom_concurrency_test`. `src/lib.rs`: `#[cfg(feature = "bloom")] pub mod bloom;` and add `feature = "bloom"` / `` `bloom` `` to the `compile_error!` guard (keep the phrase `no sketch family is enabled`).

- [ ] **Step 2: Failing tests.** Create `apache-datasketches/tests/bloom_filter_smoke_test.rs`:

```rust
use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, DEFAULT_SEED};
use apache_datasketches::SketchError;

fn filled(n: u64) -> BloomFilter {
    let mut f = BloomFilterBuilder::by_accuracy(10_000, 0.01).build().unwrap();
    for i in 0..n {
        f.update_u64(i);
    }
    f
}

#[test]
fn default_seed_and_override() {
    assert_eq!(BloomFilterBuilder::by_size(1024, 3).build().unwrap().seed(), DEFAULT_SEED);
    assert_eq!(BloomFilterBuilder::by_size(1024, 3).seed(7).build().unwrap().seed(), 7);
}

#[test]
fn no_false_negatives() {
    let f = filled(5_000);
    for i in 0..5_000u64 {
        assert!(f.query_u64(i), "false negative for {i}");
    }
}

#[test]
fn every_overload_family_round_trips() {
    let mut f = BloomFilterBuilder::by_accuracy(1_000, 0.01).build().unwrap();
    f.update_u64(1); f.update_i64(-2); f.update_u32(3); f.update_i32(-4);
    f.update_u16(5); f.update_i16(-6); f.update_u8(7); f.update_i8(-8);
    f.update_f64(9.5); f.update_f32(10.5); f.update_str("eleven"); f.update_bytes(b"twelve");
    assert!(f.query_u64(1) && f.query_i64(-2) && f.query_u32(3) && f.query_i32(-4));
    assert!(f.query_u16(5) && f.query_i16(-6) && f.query_u8(7) && f.query_i8(-8));
    assert!(f.query_f64(9.5) && f.query_f32(10.5) && f.query_str("eleven") && f.query_bytes(b"twelve"));
}

#[test]
fn query_and_update_reports_prior_membership() {
    let mut f = BloomFilterBuilder::by_accuracy(1_000, 0.01).build().unwrap();
    assert!(!f.query_and_update_u64(42));
    assert!(f.query_and_update_u64(42));
}

#[test]
fn empty_string_is_ignored() {
    let mut f = BloomFilterBuilder::by_size(1024, 3).build().unwrap();
    f.update_str("");
    assert!(f.is_empty());
    assert!(!f.query_str(""));
}

#[test]
fn union_intersect_invert_reset() {
    let mut a = BloomFilterBuilder::by_size(4096, 4).build().unwrap();
    let mut b = BloomFilterBuilder::by_size(4096, 4).build().unwrap();
    a.update_u64(1);
    b.update_u64(2);
    let mut u = a.clone();
    u.union_with(&b).unwrap();
    assert!(u.query_u64(1) && u.query_u64(2));
    let mut i = u.clone();
    i.intersect(&a).unwrap();
    assert!(i.query_u64(1));
    let mut inv = a.clone();
    let before = inv.bits_used();
    inv.invert();
    assert_eq!(inv.bits_used(), inv.capacity() - before);
    inv.reset();
    assert!(inv.is_empty());
}

#[test]
fn incompatible_filters_are_rejected() {
    let mut a = BloomFilterBuilder::by_size(4096, 4).build().unwrap();
    for other in [
        BloomFilterBuilder::by_size(4096, 4).seed(1).build().unwrap(),
        BloomFilterBuilder::by_size(8192, 4).build().unwrap(),
        BloomFilterBuilder::by_size(4096, 5).build().unwrap(),
    ] {
        assert!(!a.is_compatible(&other));
        assert!(matches!(a.union_with(&other), Err(SketchError::InvalidConfig(_))));
        assert!(matches!(a.intersect(&other), Err(SketchError::InvalidConfig(_))));
    }
}

#[test]
fn invalid_builder_arguments() {
    for (n, p) in [(0u64, 0.01), (100, 0.0), (100, 1.0), (100, f64::NAN), (100, -0.1)] {
        assert!(matches!(BloomFilterBuilder::by_accuracy(n, p).build(), Err(SketchError::InvalidConfig(_))), "{n} {p}");
    }
    assert!(matches!(BloomFilterBuilder::by_size(0, 3).build(), Err(SketchError::InvalidConfig(_))));
    assert!(matches!(BloomFilterBuilder::by_size(64, 0).build(), Err(SketchError::InvalidConfig(_))));
    assert!(matches!(BloomFilterBuilder::by_size(u64::MAX, 3).build(), Err(SketchError::InvalidConfig(_))));
}

#[test]
fn serialize_roundtrip_and_capacity_rounding() {
    let mut f = filled(1_000);
    let restored = BloomFilter::deserialize(&f.serialize()).unwrap();
    assert_eq!(restored.capacity(), f.capacity());
    assert_eq!(restored.num_hashes(), f.num_hashes());
    assert_eq!(restored.seed(), f.seed());
    let mut restored = restored;
    assert_eq!(restored.bits_used(), f.bits_used());
    assert_eq!(BloomFilterBuilder::by_size(65, 3).build().unwrap().capacity(), 128);
    assert!(matches!(BloomFilter::deserialize(&[1, 2, 3]), Err(SketchError::Deserialization(_))));
}

#[test]
fn suggestions() {
    assert!(BloomFilterBuilder::suggest_num_hashes(1_000, 9_586).unwrap() >= 1);
    assert!(BloomFilterBuilder::suggest_num_hashes_for_fpp(0.01).unwrap() >= 1);
    assert!(BloomFilterBuilder::suggest_num_filter_bits(1_000, 0.01).unwrap() > 1_000);
    assert!(matches!(BloomFilterBuilder::suggest_num_filter_bits(0, 0.01), Err(SketchError::InvalidConfig(_))));
}

#[test]
fn false_positive_rate_is_near_target() {
    // Deterministic given the seed: 10k members, 100k non-members.
    let f = filled(10_000);
    let fp = (1_000_000u64..1_100_000).filter(|&i| f.query_u64(i)).count();
    let rate = fp as f64 / 100_000.0;
    assert!(rate < 0.02, "fpp {rate} far above the 0.01 target");
}
```

Create `apache-datasketches/tests/bloom_wrapped_test.rs`:

```rust
use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};
use apache_datasketches::SketchError;

fn sample() -> BloomFilter {
    let mut f = BloomFilterBuilder::by_accuracy(1_000, 0.01).build().unwrap();
    for i in 0..500u64 {
        f.update_u64(i);
    }
    f.update_str("member");
    f
}

#[test]
fn wrapped_queries_match_owned() {
    let mut f = sample();
    let bytes = f.serialize();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    for i in 0..2_000u64 {
        assert_eq!(w.query_u64(i), f.query_u64(i), "{i}");
    }
    assert!(w.query_str("member"));
    assert_eq!(w.capacity(), f.capacity());
    assert_eq!(w.num_hashes(), f.num_hashes());
    assert_eq!(w.seed(), f.seed());
    assert_eq!(w.bits_used(), f.bits_used());
    assert_eq!(w.is_empty(), f.is_empty());
}

#[test]
fn to_owned_is_independent_and_mutable() {
    let f = sample();
    let bytes = f.serialize();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    let mut owned = w.to_owned();
    owned.update_u64(1_000_000);
    assert!(owned.query_u64(1_000_000));
    assert_eq!(bytes, f.serialize(), "wrapping must not modify the bytes");
}

#[test]
fn set_operations_accept_wrapped_inputs() {
    let f = sample();
    let bytes = f.serialize();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    let mut g = BloomFilterBuilder::by_accuracy(1_000, 0.01).build().unwrap();
    assert!(g.is_compatible(&w) && w.is_compatible(&g));
    g.union_with(&w).unwrap();
    assert!(g.query_u64(42));
    g.intersect(&w).unwrap();
    assert!(g.query_u64(42));
}

#[test]
fn empty_filter_wraps() {
    let f = BloomFilterBuilder::by_size(1024, 3).build().unwrap();
    let bytes = f.serialize();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    assert!(w.is_empty());
    assert!(!w.query_u64(1));
}

#[test]
fn garbage_bytes_fail() {
    assert!(matches!(WrappedBloomFilter::wrap(&[1, 2, 3]), Err(SketchError::Deserialization(_))));
}
```

Create `apache-datasketches/tests/bloom_concurrency_test.rs`:

```rust
use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder};
use std::sync::Arc;
use std::thread;

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn types_are_send_and_sync() {
    assert_send_sync::<BloomFilter>();
    assert_send_sync::<apache_datasketches::bloom::WrappedBloomFilter<'static>>();
}

#[test]
fn shared_filter_is_queried_concurrently() {
    let mut f = BloomFilterBuilder::by_accuracy(100_000, 0.01).build().unwrap();
    for i in 0..50_000u64 {
        f.update_u64(i);
    }
    let f = Arc::new(f);
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let f = Arc::clone(&f);
            thread::spawn(move || (0..50_000u64).all(|i| f.query_u64(i)))
        })
        .collect();
    for h in handles {
        assert!(h.join().unwrap());
    }
}

#[test]
fn per_thread_filters_union() {
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            thread::spawn(move || {
                let mut f = BloomFilterBuilder::by_accuracy(100_000, 0.01).build().unwrap();
                for i in 0..10_000u64 {
                    f.update_u64(t * 10_000 + i);
                }
                f
            })
        })
        .collect();
    let mut merged = BloomFilterBuilder::by_accuracy(100_000, 0.01).build().unwrap();
    for h in handles {
        merged.union_with(&h.join().unwrap()).unwrap();
    }
    assert!((0..40_000u64).all(|i| merged.query_u64(i)));
}
```

- [ ] **Step 3:** Run the three test files — Expected: FAIL to compile (`could not find bloom`).

- [ ] **Step 4: `input.rs`** — mirror `apache-datasketches/src/theta/input.rs`: a private `sealed::Sealed` implemented for `BloomFilter` and `WrappedBloomFilter<'a>`; `pub trait BloomInput: sealed::Sealed` with a `#[doc(hidden)] fn as_bloom_input(&self) -> BloomInputRef<'_>;` where `pub(crate) enum BloomInputRef<'a> { Filter(&'a ffi::BloomFilterShim), Wrapped(&'a ffi::WrappedBloomFilterShim) }` (defined in this file in the safe crate — the sys crate needs no enum). Doc: either filter type can be passed to `union_with`, `intersect` and `is_compatible`.

- [ ] **Step 5: `builder.rs`** — `BloomFilterBuilder` storing `enum Sizing { Accuracy { max_distinct_items: u64, target_fpp: f64 }, Size { num_bits: u64, num_hashes: u16 } }` and `seed: u64` (default `DEFAULT_SEED`). `build()` validates per the Global Constraints (Accuracy: `max_distinct_items > 0`, `target_fpp > 0.0 && target_fpp < 1.0`; Size: `1..=MAX_FILTER_SIZE_BITS`, `num_hashes >= 1`) returning `InvalidConfig` with a message naming the bad argument, then calls `bloom_new_by_accuracy`/`bloom_new_by_size` mapping any exception to `InvalidConfig`. The three `suggest_*` associated functions validate the same arguments and forward. Rustdoc on `seed()` explains the default, that filters must share a seed (and size and hash count) to be combined, and the security note (a known seed lets an adversary craft colliding inputs; use a random seed for adversarial settings).

- [ ] **Step 6: `filter.rs`** — `pub struct BloomFilter { inner: UniquePtr<ffi::BloomFilterShim> }` with `unsafe impl Send` and `unsafe impl Sync` and a SAFETY comment (the shim owns the filter; every `&self` method maps to a `const` upstream method with no `mutable` state — verified in `bloom_filter.hpp`; the one caching method, `get_bits_used`, is exposed as `&mut self`). Implement every method in the Interfaces list, using a `macro_rules!` inside `filter.rs` to generate the 12×3 update/query/query_and_update methods with doc comments (e.g. "Adds `value` (as `u64`) to the filter." / "Returns `true` if `value` may have been added (no false negatives; false positives at roughly the configured rate)." / "Returns whether `value` may have been present, then adds it."). `union_with`/`intersect` first check `self.is_compatible(other)` and return `InvalidConfig` naming the mismatch (capacity, number of hashes, or seed — compare the three accessors to say which), then call the `_with_filter`/`_with_wrapped` bridge method per `BloomInputRef`, mapping any exception to `InvalidConfig`. `deserialize` maps exceptions to `Deserialization`; its rustdoc includes the empty-image allocation warning: "An empty filter serializes to 24 bytes but records its capacity; deserializing allocates the full bit array, which upstream allows up to about 2 GiB. When deserializing untrusted bytes, bound the claimed capacity (or the process's memory) yourself." `Clone` via `clone_shim`. `bits_used(&mut self)` doc explains why it takes `&mut` (upstream recounts and caches after updates; `&mut` keeps `BloomFilter` safely `Sync`).

- [ ] **Step 7: `wrapped.rs`** — `pub struct WrappedBloomFilter<'a> { inner: UniquePtr<ffi::WrappedBloomFilterShim>, _bytes: PhantomData<&'a [u8]> }`, `unsafe impl Send/Sync` with SAFETY (read-only view; never dirty; the lifetime ties it to the bytes). `wrap(bytes: &'a [u8])` maps exceptions to `Deserialization` and carries the same empty-image allocation warning. Query methods via a macro (12 types), accessors, `is_compatible`, `to_owned` (maps exceptions to `Cpp`), `to_string_summary`. Add a `compile_fail` doctest on the struct:

```rust
/// ```compile_fail
/// use apache_datasketches::bloom::{BloomFilterBuilder, WrappedBloomFilter};
/// let w = {
///     let bytes = BloomFilterBuilder::by_size(1024, 3).build().unwrap().serialize();
///     WrappedBloomFilter::wrap(&bytes).unwrap()
/// };
/// let _ = w.capacity();
/// ```
```

- [ ] **Step 8: `mod.rs`** — module docs with a doctest (build by accuracy, add a few items, query a member and a non-member, serialize/wrap), the bullet list of types, `pub const DEFAULT_SEED: u64 = 9001;` with a doc comment, and re-exports.

- [ ] **Step 9:** Run the three test files and `cargo test -p apache-datasketches --features bloom --doc bloom` — Expected: PASS, including the `compile_fail`.
- [ ] **Step 10:** `cargo clippy -p apache-datasketches --no-default-features --features bloom --all-targets -- -D warnings`; `rustfmt --edition 2021` the new files; commit `git add apache-datasketches && git commit -m "feat(bloom): add BloomFilter, WrappedBloomFilter and the builder"`

---

### Task 4: Harden deserialization and add the fuzz regression test

Investigation plus implementation (use a capable model). Acceptance: the committed fuzz test passes and fails with the validator removed (or the report shows upstream's checks already suffice for that class), every abort/UB found has a targeted regression test, existing suites pass.

**Files:** modify `apache-datasketches-sys/cpp/bloom/bloom_filter_shim.cc`; create `apache-datasketches/tests/bloom_deserialize_fuzz_test.rs`; modify `apache-datasketches/Cargo.toml`; add targeted cases to `apache-datasketches/tests/bloom_filter_smoke_test.rs` and `bloom_wrapped_test.rs`.

**Wire format** (upstream `bloom_filter_impl.hpp` `serialize`, little-endian): byte 0 preamble longs (3 empty, 4 otherwise), 1 serial version (1), 2 family id (21), 3 flags (bit 2 = empty), 4–5 `num_hashes` u16, 6–7 unused, 8–15 `seed` u64, 16–19 `num_longs` i32 (capacity in 64-bit words), 20–23 unused; if non-empty: 24–31 bits-set count u64 (or all-ones "dirty" sentinel), then `num_longs * 8` bytes of bit array.

- [ ] **Step 1:** Read `internal_deserialize_or_wrap`, the constructors it reaches, `validate_bloom_filter_num_bits_set`, `bit_array_ops`, `union_with`/`intersect`/`query*` in `bloom_filter_impl.hpp` (5.3.0). List every field crafted bytes control and what each can reach (allocation size, bounds of bit-array indexing in `internal_query` with `capacity_bits_`, `num_hashes` loop length, signedness of `num_longs` as `int32_t` vs read as `uint32_t`, flags other than empty, preamble 3 with a non-empty flag pattern, trailing bytes).
- [ ] **Step 2: Targeted failing tests** for each problem found plus these baseline cases (both `BloomFilter::deserialize` and `WrappedBloomFilter::wrap`, expecting `Err(SketchError::Deserialization(_))` unless noted): `num_hashes = 0`; `num_longs = 0`; `num_longs` above `MAX_FILTER_SIZE_BITS / 64`; non-empty image truncated inside the bit array; bits-set count wrong (not the sentinel); unknown flag bits; preamble/flag mismatch (preamble 3 without the empty flag, preamble 4 with it). A genuine empty image with a large but valid capacity still deserializes (document this; keep the test's capacity modest, e.g. 2^20 bits). Run them (RED) and record which fail today.
- [ ] **Step 3: `validate_bloom_layout`** — bounds-checked parse (follow `validate_kll_layout`'s `read_at` style) called first in `bloom_deserialize`, `bloom_wrap` and (defensively) not needed in `to_owned` (its bytes come from a validated filter). Reject everything Step 2 identifies, citing upstream lines for each rule. Keep accepting every genuine image (empty and non-empty, any seed, dirty sentinel).
- [ ] **Step 4: Fuzz test** — `[[test]] name = "bloom_deserialize_fuzz_test"`, `required-features = ["bloom"]`. Deterministic splitmix64 (copy `next_u64` from `tests/kll_common/mod.rs`), fixed seed; corpus: genuine filters (empty, sparse, dense, inverted) for several sizes, hash counts and seeds; 20 000 iterations of 1–4 mutations (byte overwrite, bit flip, truncate, append, boundary-value field overwrite) — but clamp any mutation of `num_longs` on images flagged empty so the claimed capacity stays ≤ 2^24 bits (the deliberate empty-image allowance would otherwise allocate gigabytes). For every accepted buffer, through both `deserialize` and `wrap`: queries of several types, `union_with`/`intersect` with a compatible genuine filter when `is_compatible`, `serialize`→`deserialize`, `to_owned` (wrap), `bits_used`, `to_string_summary` — none may abort. Debug runtime under ~10 s.
- [ ] **Step 5:** Run the fuzz test, the smoke/wrapped tests, the full `cargo test -p apache-datasketches --features bloom`, sys tests and clippy. Temporarily comment out the validator call to confirm the fuzz or targeted tests fail without it; restore. Record results.
- [ ] **Step 6:** Commit `git commit -m "fix(bloom): validate serialized bytes before deserializing or wrapping; add a fuzz regression test"`

---

### Task 5: Port upstream tests, validation and wire-format tests

**Files:** create `apache-datasketches/tests/bloom_filter_test.rs`, `bloom_validation_test.rs`, `bloom_serialization_compat_test.rs`; modify `apache-datasketches/Cargo.toml` (three `[[test]]` entries, `required-features = ["bloom"]`).

- [ ] **Step 1: Port `filters/test/bloom_filter_test.cpp`** (tag 5.3.0, in `vendor/datasketches-cpp`) — one Rust test per `TEST_CASE`, preserving every assertion expressible through the safe API: `invalid constructor args`, `standard constructors`, `basic operations`, `inversion`, `incompatible set operations`, `basic union`, `basic intersection`, `empty serialization`, `non-empty serialization`, `inconsistent num bits set is rejected`. Upstream passes explicit seeds; pass the same via `.seed(x)`. Exceptions map to `Err(SketchError::InvalidConfig(_))` for builder/set-op errors and `Err(SketchError::Deserialization(_))` for bytes. Skip stream I/O and allocation-specific checks, naming them in the file header with reasons; `bit_array_ops_test.cpp` and `bloom_filter_allocation_test.cpp` are not ported (internal / custom allocator) — say so.
- [ ] **Step 2: Validation test** — every row of the spec's error table, including `target_fpp` ∈ {0.0, 1.0, −0.1, NaN} rejected, `max_distinct_items = 0`, `num_bits` ∈ {0, MAX_FILTER_SIZE_BITS + 1, u64::MAX} rejected and {1, 64} accepted (`num_bits = 1` gives capacity 64; do not allocate a maximum-size filter in tests), `num_hashes = 0`, the `suggest_*` argument errors, and each incompatibility (seed, capacity, hashes) naming the parameter in the message.
- [ ] **Step 3: Wire-format test** — golden bytes derived by hand from upstream `serialize` (format in Task 4), never from the binding's output: an empty `by_size(128, 3)` filter with the default seed → `[3, 1, 21, 4, 3, 0, 0, 0, <9001 as u64 LE>, 2, 0, 0, 0, 0, 0, 0, 0]` (24 bytes; `num_longs = 128/64 = 2`); the same with seed 12345; a non-empty filter's header bytes (preamble 4, flags 0, `num_hashes`, seed, `num_longs`), its bits-set field equal to the dirty sentinel `u64::MAX` right after updates **or** the true count after `bits_used()` was called (check `serialize` writes `is_dirty_ ? DIRTY_BITS_VALUE : num_bits_set_` and that calling `bits_used(&mut self)` first clears dirty), and total length `32 + num_longs * 8`; serialization determinism (same inputs → identical bytes); `wrap` and `deserialize` agree on every query for those images.
- [ ] **Step 4:** Run the three files and clippy; commit `git commit -m "test(bloom): port the upstream Bloom filter tests and pin the wire format"`

---

### Task 6: Benchmark pair

**Files:** create `benches/cpp_reference/bloom_update.cc`, `apache-datasketches/examples/bench_bloom_update.rs`; modify `apache-datasketches/Cargo.toml`, `benches/cpp_reference/{run.sh,compare.sh,README.md}`, `.github/workflows/ci.yml` (bench job comment).

- [ ] **Step 1: Parameters (identical both sides):** `max_distinct_items = 1_000_000`, `target_fpp = 0.01`, `seed = 9001`; scenarios in order: `distinct` (update `0..items` as u64), `query` (query `0..items` — all hits — then `items..2*items` — misses; report the total hit count), `str` (update the pre-built `key_%010llu` pool, cycled, as in the other harnesses; Rust `update_str`, C++ the `(data, length)` overload with the same empty guard), `union` (two filters over disjoint halves, `OP_CALLS` fresh unions), `ser`, `deser`. Each scenario prints exact deterministic labels via `bench::report_fields` (C++) / an equivalent Rust `report` (as in `bench_kll_update.rs`): `bits_used=` after the scenario, `hits=` for `query`, `bytes=` for `ser`/`deser`. Per-rep fields must match (rebuild the filter each rep for update scenarios).
- [ ] **Step 2:** Write `bloom_update.cc` modelled on `benches/cpp_reference/kll_update.cc` (structure, `report_fields`, constants) and `bench_bloom_update.rs` modelled on `apache-datasketches/examples/bench_kll_update.rs`; register the example (`required-features = ["bloom"]`).
- [ ] **Step 3:** `run.sh`: add `-I"$vendor/filters/include"` and `bloom` to `benchmarks`; `compare.sh`: add `"bloom:bench_bloom_update:bloom"`. Run `./benches/cpp_reference/compare.sh 100000 --reps 2` — Expected: exit 0, bloom rows present, `Estimates agree on every scenario`. A field mismatch means the sides differ — fix the drift, never loosen `compare.py`.
- [ ] **Step 4:** README: table row; note the Bloom pair cross-checks exact fields (deterministic, no estimate) and that Rust `deser` pays for the shim validator; update counts ("seven" → "eight" Rust counterparts). CI `benches` comment counts likewise.
- [ ] **Step 5:** Commit `git commit -m "bench(bloom): add the Bloom filter harness pair"`

---

### Task 7: Example, docs, CI matrix, CHANGELOG

**Files:** create `apache-datasketches/examples/bloom.rs`; modify `apache-datasketches/Cargo.toml`, `apache-datasketches/src/lib.rs`, `README.md`, `apache-datasketches/README.md`, `apache-datasketches-sys/README.md`, `AGENTS.md`, `.github/workflows/ci.yml`, `CHANGELOG.md`.

- [ ] **Step 1: Example** — `apache-datasketches/examples/bloom.rs` (register, `required-features = ["bloom"]`): build `by_accuracy(100_000, 0.01)`, add 50 000 user IDs, show members are always found, measure the false-positive rate over 100 000 non-members and print it, show `query_and_update` deduplicating a stream, union two shard filters (same default seed), show an incompatible union failing (different seed) with the error, serialize and `wrap` the bytes for zero-copy lookups, and `to_owned`. Run it; it must complete.
- [ ] **Step 2: Crate docs** — `src/lib.rs`: family count → seven, a `bloom` bullet (`` - `bloom` (feature `bloom`) — Bloom filters for approximate set membership: no false negatives, a configurable false-positive rate, union/intersection, and zero-copy wrapping of serialized filters. ``), sidebar list.
- [ ] **Step 3: READMEs** (read the "KLL sketches"/"REQ sketches" sections first and match their structure): `apache-datasketches/README.md` intro count/list (wrap at ~80 columns), a "## Bloom filters" section (snippet from the `bloom/mod.rs` doctest; per-type list; compatibility/serialization paragraph incl. the default seed and that filters interoperate with Java/Python when seeds match; a "what differs" list — membership rather than an estimate, the default seed and its security note, `bits_used(&mut self)`, set operations need compatible filters, the empty-image allocation warning; example coverage), families-list bullet. Root `README.md`: count, families bullet, example command. `apache-datasketches-sys/README.md`: `bloom` bullet, toml snippet, all-features snippet.
- [ ] **Step 4: AGENTS.md** — families list/count, test/example/bench commands, `,bloom` in the all-families test line, `filters/` in the vendored-modules list, and a short Bloom paragraph (single non-template shim; wrapped view's lifetime; validator + fuzz test; default seed; deterministic bench cross-check).
- [ ] **Step 5: CI** — `features` matrix gains `- "--no-default-features --features bloom"`; default-leg comment count.
- [ ] **Step 6: CHANGELOG** — under `## [Unreleased]` add `### Added` with a Bloom filter bullet (types, default seed, zero-copy wrap, hardening + fuzz test, the empty-image allocation note) and an `apache-datasketches-sys` `bloom` feature bullet; and fix the heading `## [0.3.0] — 2026-10-27` → `## [0.3.0] — 2026-10-29` (the actual publish date). Do not edit any other released entry.
- [ ] **Step 7: Full verification**

```bash
cargo build -p apache-datasketches --no-default-features --features bloom
cargo test --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo build -p apache-datasketches --no-default-features 2>&1 | grep -q "no sketch family is enabled"
cargo fmt --all -- --check
./benches/cpp_reference/compare.sh 100000 --reps 2
```

- [ ] **Step 8:** Commit `git commit -m "docs(bloom): document the Bloom filter family, add its example, CI leg and CHANGELOG entry"`; stop (no version bump).
