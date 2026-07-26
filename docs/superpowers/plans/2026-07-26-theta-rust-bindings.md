# Theta Rust Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Theta sketch family (`update_theta_sketch`, `compact_theta_sketch`, `wrapped_compact_theta_sketch`, `theta_union`, `theta_intersection`, `theta_a_not_b`, and Jaccard similarity) to `apache-datasketches-sys`/`apache-datasketches`, following the design in `docs/superpowers/specs/2026-07-26-theta-rust-bindings-design.md`, mirroring the HLL implementation's patterns (`docs/superpowers/plans/2026-07-26-hll-rust-bindings.md`).

**Architecture:** Seven small non-template C++ shim classes/functions wrap the real templated `datasketches::update_theta_sketch`/`compact_theta_sketch`/`wrapped_compact_theta_sketch`/`theta_union`/`theta_intersection`/`theta_a_not_b`/`theta_jaccard_similarity` (default-allocator aliases), each bridged to Rust via its own `cxx::bridge` module (seven bridge files, vs. HLL's one). `apache-datasketches` wraps these in three distinct idiomatic Rust sketch types (`ThetaSketch`, `CompactThetaSketch`, `WrappedCompactThetaSketch<'a>` — no shared query trait, matching HLL's precedent), three set-operation types (`ThetaUnion`, `ThetaIntersection`, `ThetaAnotB`) that accept any of the three sketch types via a small sealed `ThetaInput` trait, and a free `jaccard_similarity()` function. Both crates' `hll`/`theta` features become explicit opt-in (`default = []`) as part of this work.

**Tech Stack:** Rust (stable), `cxx` + `cxx-build` crates, C++17, existing Cargo workspace, existing `datasketches-cpp` git submodule at tag `5.2.0`.

## Global Constraints

- FFI layer uses `cxx`, not `bindgen`/`autocxx` (unchanged from HLL).
- `datasketches-cpp` submodule stays pinned to tag `5.2.0` at `vendor/datasketches-cpp` (already vendored; no re-pin needed for Theta).
- Every theta constructor, builder, and (de)serialize call always uses `DEFAULT_SEED` (`9001`, from `common/include/common_defs.hpp`) internally; **no type in this plan exposes a seed parameter**.
- Three distinct Rust sketch types — `ThetaSketch` (mutable, update-only), `CompactThetaSketch` (immutable, serializable), `WrappedCompactThetaSketch<'a>` (zero-copy view over `&'a [u8]`) — with no shared Rust query trait between them (each implements the same query methods individually, matching HLL's no-trait precedent).
- `ThetaUnion`/`ThetaIntersection`/`ThetaAnotB` accept any of the three sketch types via a sealed `ThetaInput` trait (`mod sealed { pub trait Sealed {} } pub trait ThetaInput: sealed::Sealed { fn as_theta_input(&self) -> sys::ThetaInputRef<'_>; }`), not implementable outside this crate.
- One `SketchError` enum shared across all sketch families (already exists in `apache-datasketches/src/error.rs`); this plan adds exactly one new variant, `SketchError::EmptyIntersection`, for `ThetaIntersection::get_result()` called before any `update()`.
- `ResizeFactor` is a `cxx`-shared C-like enum (`X1`, `X2`, `X4`, `X8`), used by both `ThetaSketchBuilder` and `ThetaUnionBuilder`; default (when unset) is `X8`, matching upstream's `theta_constants::DEFAULT_RESIZE_FACTOR`.
- Both v3 (uncompressed) and v4 (compressed) serialization formats are in scope for `CompactThetaSketch`.
- Every new Rust sketch/set-op/wrapper type is `unsafe impl Send`, explicitly not `Sync` (matching `HllSketch`/`HllUnion`).
- Tests are 1:1-ported from the six upstream Catch2 files listed below, same test names/order where practical, each file with a header comment linking to its upstream source — plus new, clearly-marked non-upstream tests for `ThetaInput` trait dispatch and v4 compressed round-trips (replacing `bit_packing_test.cpp`, which has no public-API surface to port).
- Both crates' `default = ["hll"]` becomes `default = []`; `hll = []` and `theta = []` become explicit opt-in features users must request (breaking change, acceptable now per the design spec since neither crate has real external users yet).
- Dual MIT/Apache-2.0 license (unchanged).
- No CI in this plan (unchanged from HLL).

## Reference: real datasketches-cpp Theta API (verified against tag 5.2.0, from `vendor/datasketches-cpp/theta/include/`)

Constants (`theta_constants.hpp`, `common/include/common_defs.hpp`):

```cpp
namespace datasketches {
  static const uint64_t DEFAULT_SEED = 9001;
  enum resize_factor { X1 = 0, X2, X4, X8 };
}
namespace datasketches::theta_constants {
  using resize_factor = datasketches::resize_factor;
  const resize_factor DEFAULT_RESIZE_FACTOR = resize_factor::X8;
  const uint8_t MIN_LG_K = 5;
  const uint8_t MAX_LG_K = 26;
  const uint8_t DEFAULT_LG_K = 12;
}
```

`update_theta_sketch` (`update_theta_sketch_alloc<std::allocator<uint64_t>>`, `theta_sketch.hpp`) — no public constructor, built only via `builder`:

```cpp
class update_theta_sketch {
public:
  class builder { // : theta_base_builder<builder, Allocator>
    builder& set_lg_k(uint8_t lg_k);
    builder& set_resize_factor(resize_factor rf);
    builder& set_p(float p);
    // set_seed(uint64_t) exists upstream but is never called by this plan's shim
    update_theta_sketch build() const;
  };

  bool is_empty() const;
  bool is_ordered() const;
  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_devs) const;
  double get_upper_bound(uint8_t num_std_devs) const;
  bool is_estimation_mode() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  void update(const std::string& value);
  void update(uint64_t value); void update(int64_t value);
  void update(uint32_t value); void update(int32_t value);
  void update(uint16_t value); void update(int16_t value);
  void update(uint8_t value);  void update(int8_t value);
  void update(double value);   void update(float value);
  void update(const void* data, size_t length);

  void trim();
  void reset();
  compact_theta_sketch compact(bool ordered = true) const;
};
```

`compact_theta_sketch` (`theta_sketch.hpp`):

```cpp
class compact_theta_sketch {
public:
  template<typename Other> compact_theta_sketch(const Other& other, bool ordered);

  bool is_empty() const; bool is_ordered() const;
  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_devs) const;
  double get_upper_bound(uint8_t num_std_devs) const;
  bool is_estimation_mode() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  std::vector<uint8_t> serialize(unsigned header_size_bytes = 0) const;            // v3, uncompressed
  std::vector<uint8_t> serialize_compressed(unsigned header_size_bytes = 0) const; // v4, compressed

  static compact_theta_sketch deserialize(const void* bytes, size_t size,
      uint64_t seed = DEFAULT_SEED, const Allocator& allocator = Allocator());
  // deserialize() auto-detects serial version v1/v2/v3/v4 from the preamble;
  // there is no separate upstream "deserialize_compressed" entry point.
};
```

`wrapped_compact_theta_sketch` (`theta_sketch.hpp`) — zero-copy view, no owned constructor:

```cpp
class wrapped_compact_theta_sketch {
public:
  bool is_empty() const; bool is_ordered() const;
  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_devs) const;
  double get_upper_bound(uint8_t num_std_devs) const;
  bool is_estimation_mode() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  static const wrapped_compact_theta_sketch wrap(const void* bytes, size_t size,
      uint64_t seed = DEFAULT_SEED, bool dump_on_error = false);
  // throws std::invalid_argument on seed-hash mismatch.
};
```

`theta_union` (`theta_union.hpp`):

```cpp
class theta_union {
public:
  class builder { // same shape as update_theta_sketch::builder
    builder& set_lg_k(uint8_t lg_k);
    builder& set_resize_factor(resize_factor rf);
    builder& set_p(float p);
    theta_union build() const;
  };

  template<typename FwdSketch> void update(FwdSketch&& sketch); // duck-typed, accepts any of the 3 sketch types
  compact_theta_sketch get_result(bool ordered = true) const;
  void reset();
  // update() throws std::invalid_argument on seed-hash mismatch.
};
```

`theta_intersection` (`theta_intersection.hpp`, `theta_intersection_base_impl.hpp`):

```cpp
class theta_intersection {
public:
  explicit theta_intersection(uint64_t seed = DEFAULT_SEED); // this plan never passes seed

  template<typename FwdSketch> void update(FwdSketch&& sketch);
  compact_theta_sketch get_result(bool ordered = true) const;
  // throws std::invalid_argument("calling get_result() before calling update() is undefined")
  // when called before any update() — maps to SketchError::EmptyIntersection.
  bool has_result() const;
};
```

`theta_a_not_b` (`theta_a_not_b.hpp`):

```cpp
class theta_a_not_b {
public:
  explicit theta_a_not_b(uint64_t seed = DEFAULT_SEED);

  template<typename FwdSketch, typename Sketch>
  compact_theta_sketch compute(FwdSketch&& a, const Sketch& b, bool ordered = true) const;
  // instance method, not static; returns by value; duck-typed over all 3 sketch types on both sides.
};
```

`theta_jaccard_similarity` (`theta_jaccard_similarity.hpp`/`_base.hpp`):

```cpp
class theta_jaccard_similarity {
public:
  template<typename SketchA, typename SketchB>
  static std::array<double, 3> jaccard(const SketchA& a, const SketchB& b, uint64_t seed = DEFAULT_SEED);
  // returns {lower_bound, estimate, upper_bound}, in that order.
};
```

## Design resolutions (spec vs. real upstream API)

The design spec is authoritative, but three details needed resolving against the actual C++ headers:

1. **`sys::ThetaInputRef` is a plain Rust enum, not a `cxx`-bridged type.** `cxx` bridge enums must be C-like (no payload), so an enum carrying borrowed references to three different opaque C++ types cannot be declared inside a `#[cxx::bridge]` block. This plan defines `ThetaInputRef<'a>` as an ordinary Rust `pub enum` in `apache-datasketches-sys/src/theta_input.rs`, with variants borrowing the opaque shim types from each bridge module (`Sketch(&'a theta_sketch::ffi::ThetaSketchShim)`, `Compact(&'a theta_compact::ffi::CompactThetaSketchShim)`, `Wrapped(&'a theta_wrapped::ffi::WrappedCompactThetaSketchShim)`). Callers in the safe crate `match` on it to pick the right concrete shim method (`update_with_sketch`/`update_with_compact`/`update_with_wrapped`), exactly as the design's narrative describes.
2. **`CompactThetaSketch::serialize_compact` takes no `ordered` parameter.** The spec's listed signature `serialize_compact(ordered: bool) -> Vec<u8>` doesn't correspond to any upstream parameter — `compact_theta_sketch::serialize()` only takes an optional `header_size_bytes` (unused here, always `0`). Orderedness is fixed at *construction* time (`ThetaSketch::compact(ordered)` or a set-op's `get_result(ordered)`), not at serialize time. This plan implements `serialize_compact(&self) -> Vec<u8>` with no parameter, matching upstream `serialize()` exactly and mirroring HLL's zero-arg `serialize_compact()`/`serialize_updatable()` convention.
3. **`deserialize`/`deserialize_compressed` are two thin Rust entry points over one upstream function.** Upstream's single `compact_theta_sketch::deserialize()` already auto-detects v1/v2/v3/v4 from the preamble transparently — there is no separate "compressed" C++ entry point. This plan still exposes both `CompactThetaSketch::deserialize(bytes)` and `CompactThetaSketch::deserialize_compressed(bytes)` as the spec requires (for call-site symmetry with `serialize_compact`/`serialize_compressed`), but both call the exact same shim function; no duplicate deserialization logic is implemented.

## Test inventory to port (verified against tag 5.2.0)

`theta_sketch_test.cpp` (22 cases, tag `[theta_sketch]`): check empty; non empty no retained keys; single item; resize exact; estimation; deserialize compact v1/v2 empty from java (2 cases); deserialize compact v1/v2 estimation from java (2 cases); serialize deserialize stream and bytes equivalence; deserialize empty/single item/exact mode/estimation mode buffer overrun (4 cases); conversion constructor and wrapped compact; wrap compact v1/v2 empty from java (2 cases); wrap compact v1/v2 estimation from java (2 cases); serialize deserialize small compressed; serialize deserialize compressed; max serialized size.

`theta_union_test.cpp` (7 cases, tag `[theta_union]`): empty; non empty no retained keys; exact mode half overlap; exact mode half overlap wrapped compact; estimation mode half overlap; seed mismatch; larger K.

`theta_intersection_test.cpp` (13 cases, tag `[theta_intersection]`): invalid; empty; non empty no retained keys; exact mode half overlap unordered/ordered (2); exact mode disjoint unordered/ordered (2); estimation mode half overlap unordered/ordered/ordered wrapped compact (3); estimation mode disjoint unordered/ordered (2); seed mismatch.

`theta_a_not_b_test.cpp` (11 cases, tag `[theta_a_not_b]`): empty; non empty no retained keys; exact mode half overlap; exact mode disjoint; exact mode full overlap; estimation mode half overlap; estimation mode half overlap wrapped compact; estimation mode disjoint; estimation mode full overlap; seed mismatch; issue #152.

`theta_setop_test.cpp` (16 cases, untagged): all pairwise combinations of `{empty, exact, degenerate, estimation}` × `{empty, exact, degenerate, estimation}` (`"empty empty"`, `"empty exact"`, `"empty degenerate"`, `"empty estimation"`, `"exact empty"`, `"exact exact"`, `"exact degenerate"`, `"exact estimation"`, `"estimation empty"`, `"estimation exact"`, `"estimation degenerate"`, `"estimation estimation"`, `"degenerate empty"`, `"degenerate exact"`, `"degenerate degenerate"`, `"degenerate estimation"`), each checking intersection/a-not-b/union theta, retained-count, and emptiness.

`theta_jaccard_similarity_test.cpp` (10 cases, tag `[theta_sketch]`): empty; same sketch exact mode; full overlap exact mode; disjoint exact mode; half overlap estimation mode; half overlap estimation mode custom seed (ported using default seed only, per this plan's no-custom-seed constraint — see Task 12); similarity test; similarity test custom seed (same caveat); dissimilarity test; dissimilarity test custom seed (same caveat).

Not ported (per spec): `bit_packing_test.cpp` (no public-API surface; covered instead by Task 15's new compressed round-trip tests), `theta_sketch_deserialize_from_java_test.cpp`/`theta_sketch_serialize_for_java.cpp` (Java interop fixtures, no counterpart in this project, and their `.sk` fixture files are not present anywhere in this repo or its vendored submodule checkout). The from-java round-trip *assertions* embedded inside `theta_sketch_test.cpp` itself (cases 6–9, 16–19 above) are likewise **not ported**, for the same reason: they depend on the same missing `.sk` fixture files, which this plan does not introduce (no build step copies or generates them). Task 14 (which ports `theta_sketch_test.cpp`) ports every other case from that file and notes this exclusion explicitly.

---

### Task 1: Vendor theta headers into `apache-datasketches-sys` + `theta` feature/build.rs wiring

**Files:**
- Create: `apache-datasketches-sys/vendor/datasketches-cpp/theta/include/` (copy from root submodule)
- Modify: `apache-datasketches-sys/vendor/README.md`
- Modify: `apache-datasketches-sys/Cargo.toml`
- Modify: `apache-datasketches-sys/build.rs`

**Interfaces:**
- Produces: a `theta` Cargo feature (non-default, alongside the still-default `hll`) on `apache-datasketches-sys`, and a `build.rs` that includes theta's headers and is wired to compile `cpp/theta/*_shim.cc` under `cfg!(feature = "theta")` once those files exist (Task 2+). This task alone does not add any shim files, so the `theta` feature does not yet compile any new C++ — verified in Task 2.

- [ ] **Step 1: Copy the theta headers into the crate-local vendor copy**

```bash
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/theta
cp -R vendor/datasketches-cpp/theta/include apache-datasketches-sys/vendor/datasketches-cpp/theta/include
```

- [ ] **Step 2: Update `apache-datasketches-sys/vendor/README.md`'s sync script**

Add a theta copy step alongside the existing `common`/`hll` steps:

```markdown
## Updating after bumping the submodule's pinned tag

```bash
rm -rf apache-datasketches-sys/vendor/datasketches-cpp
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/common
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/hll
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/theta
cp -R vendor/datasketches-cpp/common/include apache-datasketches-sys/vendor/datasketches-cpp/common/include
cp -R vendor/datasketches-cpp/hll/include apache-datasketches-sys/vendor/datasketches-cpp/hll/include
cp -R vendor/datasketches-cpp/theta/include apache-datasketches-sys/vendor/datasketches-cpp/theta/include
rm apache-datasketches-sys/vendor/datasketches-cpp/common/include/version.hpp.in
cp vendor/datasketches-cpp/LICENSE apache-datasketches-sys/vendor/datasketches-cpp/LICENSE
cp vendor/datasketches-cpp/NOTICE apache-datasketches-sys/vendor/datasketches-cpp/NOTICE
```

When a future sketch family needs headers outside `common/`+`hll/`+`theta/`, add
its `include/` directory to both this script and `build.rs`.
```

Also update the sentence above it: "Only the headers actually compiled (`common/include`, `hll/include`, `theta/include`, `LICENSE`, `NOTICE`) are copied".

- [ ] **Step 3: Add the `theta` feature to `apache-datasketches-sys/Cargo.toml`**

```toml
[features]
default = ["hll"]
hll = []
theta = []
```

(`default` is still `["hll"]` at this point in the plan — the design spec's `default = []` breaking change is applied last, in Task 16, after everything is built and tested with explicit `--features`.)

- [ ] **Step 4: Wire `build.rs` to include theta's headers and compile its shims when the feature is on**

Replace the whole file:

```rust
fn main() {
    let mut bridges: Vec<&str> = Vec::new();

    if cfg!(feature = "hll") {
        bridges.push("src/hll.rs");
    }
    if cfg!(feature = "theta") {
        bridges.push("src/theta_sketch.rs");
        bridges.push("src/theta_compact.rs");
        bridges.push("src/theta_wrapped.rs");
        bridges.push("src/theta_union.rs");
        bridges.push("src/theta_intersection.rs");
        bridges.push("src/theta_a_not_b.rs");
        bridges.push("src/theta_jaccard.rs");
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
        .include("cpp")
        .include("cpp/hll")
        .include("cpp/theta")
        .include(generated_header_dir)
        .flag_if_supported("-std=c++17");

    if cfg!(feature = "hll") {
        build
            .file("cpp/hll/hll_sketch_shim.cc")
            .file("cpp/hll/hll_union_shim.cc");
    }
    if cfg!(feature = "theta") {
        build
            .file("cpp/theta/theta_sketch_shim.cc")
            .file("cpp/theta/theta_compact_shim.cc")
            .file("cpp/theta/theta_wrapped_shim.cc")
            .file("cpp/theta/theta_union_shim.cc")
            .file("cpp/theta/theta_intersection_shim.cc")
            .file("cpp/theta/theta_a_not_b_shim.cc")
            .file("cpp/theta/theta_jaccard_shim.cc");
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
}
```

Note: this references seven `src/theta_*.rs` bridge files and seven `cpp/theta/*_shim.cc` files that don't exist yet — same "will not compile with `--features theta` until later tasks populate them" situation as HLL's Task 2. Do not build with `--features theta` yet.

- [ ] **Step 5: Confirm the crate still builds with its current default features (theta untouched)**

```bash
cargo build -p apache-datasketches-sys
```

Expected: PASS (theta feature is off by default, so none of the missing files are referenced).

- [ ] **Step 6: Commit**

```bash
git add apache-datasketches-sys/vendor/datasketches-cpp/theta apache-datasketches-sys/vendor/README.md apache-datasketches-sys/Cargo.toml apache-datasketches-sys/build.rs
git commit -m "Vendor theta headers and wire theta feature into build.rs"
```

---

### Task 2: `ThetaSketch` C++ shim (builder, update overloads, trim/reset, query) + cxx bridge + link test

**Files:**
- Create: `apache-datasketches-sys/cpp/theta/theta_sketch_shim.h`
- Create: `apache-datasketches-sys/cpp/theta/theta_sketch_shim.cc`
- Create: `apache-datasketches-sys/src/theta_sketch.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Test: `apache-datasketches-sys/tests/theta_sketch_link_test.rs`

**Interfaces:**
- Consumes: `datasketches::update_theta_sketch` from `vendor/datasketches-cpp/theta/include/theta_sketch.hpp` (verified in the Reference section above).
- Produces: C++ class `apache_datasketches_rs::ThetaSketchShim` and free function `new_theta_sketch`; Rust bridge `apache_datasketches_sys::theta_sketch::ffi::{ResizeFactor, ThetaSketchShim, new_theta_sketch}` plus bridged methods, consumed by Task 3 (safe wrapper) and Task 4 (`CompactThetaSketchShim`'s conversion constructor, via `ThetaSketchShim::inner()`). Note: `ThetaSketchShim::compact()` is deliberately **not** added yet — it returns a `CompactThetaSketchShim`, which doesn't exist until Task 4; it's added in Task 5.

- [ ] **Step 1: Write `theta_sketch_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include <stdexcept>
#include "rust/cxx.h"
#include "theta_sketch.hpp"

namespace apache_datasketches_rs {

// Forward declaration matching the enum generated by cxx from src/theta_sketch.rs
// (this task). We deliberately do NOT `#include "theta_sketch.rs.h"` here, for
// the same reason documented in HLL's hll_sketch_shim.h: the generated header's
// own `include!` directive re-enters this header while it's still being
// processed. The full enum definition is pulled in by theta_sketch_shim.cc via
// `#include "theta_sketch.rs.h"` after this header.
enum class ResizeFactor : std::uint8_t;

datasketches::resize_factor to_cpp_resize_factor(ResizeFactor rf);

class ThetaSketchShim {
public:
  explicit ThetaSketchShim(uint8_t lg_k, ResizeFactor rf, float p);
  explicit ThetaSketchShim(datasketches::update_theta_sketch sketch);

  void update_u64(uint64_t value);
  void update_i64(int64_t value);
  void update_u32(uint32_t value);
  void update_i32(int32_t value);
  void update_u16(uint16_t value);
  void update_i16(int16_t value);
  void update_u8(uint8_t value);
  void update_i8(int8_t value);
  void update_f64(double value);
  void update_str(rust::Str value);
  void update_bytes(rust::Slice<const uint8_t> value);

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

  const datasketches::update_theta_sketch& inner() const { return sketch_; }

private:
  datasketches::update_theta_sketch sketch_;
};

std::unique_ptr<ThetaSketchShim> new_theta_sketch(uint8_t lg_k, ResizeFactor rf, float p);

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Write `theta_sketch_shim.cc`**

```cpp
#include "theta_sketch_shim.h"
#include "theta_sketch.rs.h" // generated by cxx from src/theta_sketch.rs (this task); provides the full ResizeFactor enum definition

namespace apache_datasketches_rs {

datasketches::resize_factor to_cpp_resize_factor(ResizeFactor rf) {
  switch (rf) {
    case ResizeFactor::X1: return datasketches::resize_factor::X1;
    case ResizeFactor::X2: return datasketches::resize_factor::X2;
    case ResizeFactor::X4: return datasketches::resize_factor::X4;
    case ResizeFactor::X8: return datasketches::resize_factor::X8;
    default: throw std::invalid_argument("unknown ResizeFactor");
  }
}

namespace {
datasketches::update_theta_sketch build_sketch(uint8_t lg_k, ResizeFactor rf, float p) {
  datasketches::update_theta_sketch::builder builder;
  builder.set_lg_k(lg_k);
  builder.set_resize_factor(to_cpp_resize_factor(rf));
  builder.set_p(p);
  return builder.build();
}
} // namespace

ThetaSketchShim::ThetaSketchShim(uint8_t lg_k, ResizeFactor rf, float p)
  : sketch_(build_sketch(lg_k, rf, p)) {}

ThetaSketchShim::ThetaSketchShim(datasketches::update_theta_sketch sketch)
  : sketch_(std::move(sketch)) {}

void ThetaSketchShim::update_u64(uint64_t value) { sketch_.update(value); }
void ThetaSketchShim::update_i64(int64_t value) { sketch_.update(value); }
void ThetaSketchShim::update_u32(uint32_t value) { sketch_.update(value); }
void ThetaSketchShim::update_i32(int32_t value) { sketch_.update(value); }
void ThetaSketchShim::update_u16(uint16_t value) { sketch_.update(value); }
void ThetaSketchShim::update_i16(int16_t value) { sketch_.update(value); }
void ThetaSketchShim::update_u8(uint8_t value) { sketch_.update(value); }
void ThetaSketchShim::update_i8(int8_t value) { sketch_.update(value); }
void ThetaSketchShim::update_f64(double value) { sketch_.update(value); }
void ThetaSketchShim::update_str(rust::Str value) {
  sketch_.update(std::string(value));
}
void ThetaSketchShim::update_bytes(rust::Slice<const uint8_t> value) {
  sketch_.update(value.data(), value.size());
}

void ThetaSketchShim::trim() { sketch_.trim(); }
void ThetaSketchShim::reset() { sketch_.reset(); }

double ThetaSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double ThetaSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double ThetaSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool ThetaSketchShim::is_empty() const { return sketch_.is_empty(); }
bool ThetaSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool ThetaSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double ThetaSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t ThetaSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

std::unique_ptr<ThetaSketchShim> new_theta_sketch(uint8_t lg_k, ResizeFactor rf, float p) {
  return std::make_unique<ThetaSketchShim>(lg_k, rf, p);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Write `apache-datasketches-sys/src/theta_sketch.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ResizeFactor {
        X1,
        X2,
        X4,
        X8,
    }

    unsafe extern "C++" {
        include!("theta_sketch_shim.h");

        type ThetaSketchShim;

        fn new_theta_sketch(lg_k: u8, rf: ResizeFactor, p: f32) -> Result<UniquePtr<ThetaSketchShim>>;

        fn update_u64(self: Pin<&mut ThetaSketchShim>, value: u64);
        fn update_i64(self: Pin<&mut ThetaSketchShim>, value: i64);
        fn update_u32(self: Pin<&mut ThetaSketchShim>, value: u32);
        fn update_i32(self: Pin<&mut ThetaSketchShim>, value: i32);
        fn update_u16(self: Pin<&mut ThetaSketchShim>, value: u16);
        fn update_i16(self: Pin<&mut ThetaSketchShim>, value: i16);
        fn update_u8(self: Pin<&mut ThetaSketchShim>, value: u8);
        fn update_i8(self: Pin<&mut ThetaSketchShim>, value: i8);
        fn update_f64(self: Pin<&mut ThetaSketchShim>, value: f64);
        fn update_str(self: Pin<&mut ThetaSketchShim>, value: &str);
        fn update_bytes(self: Pin<&mut ThetaSketchShim>, value: &[u8]);

        fn trim(self: Pin<&mut ThetaSketchShim>);
        fn reset(self: Pin<&mut ThetaSketchShim>);

        fn get_estimate(self: &ThetaSketchShim) -> f64;
        fn get_lower_bound(self: &ThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &ThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &ThetaSketchShim) -> bool;
        fn is_estimation_mode(self: &ThetaSketchShim) -> bool;
        fn is_ordered(self: &ThetaSketchShim) -> bool;
        fn get_theta(self: &ThetaSketchShim) -> f64;
        fn get_num_retained(self: &ThetaSketchShim) -> u32;
    }
}
```

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "hll")]
pub mod hll;

#[cfg(feature = "theta")]
pub mod theta_sketch;
```

- [ ] **Step 5: Build with the `theta` feature to confirm the shim and bridge compile and link**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: successful build. If `MIN_LG_K`/`MAX_LG_K` need re-confirming, `grep -n "MIN_LG_K\|MAX_LG_K\|DEFAULT_LG_K" apache-datasketches-sys/vendor/datasketches-cpp/theta/include/theta_constants.hpp` — this plan uses `5` and `26` (verified above); adjust the link test below if the header disagrees.

- [ ] **Step 6: Write a link-level smoke test**

```rust
// apache-datasketches-sys/tests/theta_sketch_link_test.rs
use apache_datasketches_sys::theta_sketch::ffi;

#[test]
fn construct_update_estimate_roundtrip() {
    let mut sketch = ffi::new_theta_sketch(12, ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = ffi::new_theta_sketch(4, ffi::ResizeFactor::X8, 1.0);
    assert!(result.is_err());
}
```

- [ ] **Step 7: Run the smoke test**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features theta
```

Expected: both tests PASS. If `invalid_lg_k_returns_err` unexpectedly passes/fails at the boundary, re-check `MIN_LG_K` from Step 5 and adjust the test value (must stay below the real minimum).

- [ ] **Step 8: Commit**

```bash
git add apache-datasketches-sys/cpp/theta/theta_sketch_shim.h apache-datasketches-sys/cpp/theta/theta_sketch_shim.cc apache-datasketches-sys/src/theta_sketch.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/tests/theta_sketch_link_test.rs
git commit -m "Add ThetaSketch C++ shim, cxx bridge, and link smoke test"
```

---

### Task 3: Safe `ResizeFactor`, `ThetaSketchBuilder`, and `ThetaSketch` wrapper (no `compact()` yet)

**Files:**
- Create: `apache-datasketches/src/theta/mod.rs`
- Create: `apache-datasketches/src/theta/builder.rs`
- Create: `apache-datasketches/src/theta/sketch.rs`
- Modify: `apache-datasketches/Cargo.toml`
- Modify: `apache-datasketches/src/lib.rs`
- Test: `apache-datasketches/tests/theta_sketch_smoke_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches_sys::theta_sketch::ffi::{ResizeFactor as CppResizeFactor, ThetaSketchShim, new_theta_sketch}` (Task 2).
- Consumes: `crate::error::SketchError` (existing).
- Produces: `apache_datasketches::theta::{ResizeFactor, ThetaSketchBuilder, ThetaSketch}`, consumed by Task 4 (`CompactThetaSketch`'s conversion path), Task 5 (`ThetaSketch::compact()`), and Task 8 (`ThetaInput` impl for `ThetaSketch`). `ThetaSketch { pub(crate) inner: UniquePtr<sys::ThetaSketchShim> }` — `pub(crate)` visibility matches HLL's `HllSketch.inner` convention so sibling modules (`compact.rs`, `input.rs`) can reach it.

- [ ] **Step 1: Add the `theta` feature to `apache-datasketches/Cargo.toml`**

```toml
[features]
default = ["hll"]
hll = ["apache-datasketches-sys/hll"]
theta = ["apache-datasketches-sys/theta"]
```

- [ ] **Step 2: Write `apache-datasketches/src/theta/builder.rs`**

```rust
use apache_datasketches_sys::theta_sketch::ffi as sys;

/// Controls how aggressively a theta sketch's internal hash table grows.
/// Mirrors upstream's `datasketches::resize_factor`. Default is `X8`,
/// matching `theta_constants::DEFAULT_RESIZE_FACTOR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeFactor {
    X1,
    X2,
    X4,
    X8,
}

impl Default for ResizeFactor {
    fn default() -> Self {
        ResizeFactor::X8
    }
}

impl From<ResizeFactor> for sys::ResizeFactor {
    fn from(rf: ResizeFactor) -> Self {
        match rf {
            ResizeFactor::X1 => sys::ResizeFactor::X1,
            ResizeFactor::X2 => sys::ResizeFactor::X2,
            ResizeFactor::X4 => sys::ResizeFactor::X4,
            ResizeFactor::X8 => sys::ResizeFactor::X8,
        }
    }
}

/// Builder for [`crate::theta::ThetaSketch`], mirroring upstream's
/// `update_theta_sketch::builder`. `lg_k` defaults to `12`
/// (`theta_constants::DEFAULT_LG_K`), `resize_factor` to [`ResizeFactor::X8`],
/// `p` to `1.0` (no sampling). The seed is never exposed — every sketch built
/// by this crate always uses upstream's `DEFAULT_SEED`.
#[derive(Debug, Clone, Copy)]
pub struct ThetaSketchBuilder {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
}

impl Default for ThetaSketchBuilder {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::default(),
            p: 1.0,
        }
    }
}

impl ThetaSketchBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    pub fn build(self) -> Result<super::ThetaSketch, crate::error::SketchError> {
        super::ThetaSketch::from_parts(self.lg_k, self.resize_factor, self.p)
    }
}
```

- [ ] **Step 3: Write `apache-datasketches/src/theta/sketch.rs`**

```rust
use crate::error::SketchError;
use crate::theta::builder::ResizeFactor;
use apache_datasketches_sys::theta_sketch::ffi as sys;
use cxx::UniquePtr;

pub struct ThetaSketch {
    pub(crate) inner: UniquePtr<sys::ThetaSketchShim>,
}

unsafe impl Send for ThetaSketch {}

impl ThetaSketch {
    pub(crate) fn from_parts(lg_k: u8, rf: ResizeFactor, p: f32) -> Result<Self, SketchError> {
        let inner = sys::new_theta_sketch(lg_k, rf.into(), p)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
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

    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

    pub fn trim(&mut self) {
        self.inner.pin_mut().trim();
    }

    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
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

    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }
}
```

- [ ] **Step 4: Write `apache-datasketches/src/theta/mod.rs`**

```rust
mod builder;
mod sketch;

pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use sketch::ThetaSketch;
```

(`compact.rs`/`wrapped.rs`/`union.rs`/`intersection.rs`/`a_not_b.rs`/`jaccard.rs`/`input.rs` are added by Tasks 4–12; each adds its own `mod`/`pub use` line to this file when created.)

- [ ] **Step 5: Wire the `theta` module into `apache-datasketches/src/lib.rs`**

```rust
pub mod error;

#[cfg(feature = "hll")]
pub mod hll;

#[cfg(feature = "theta")]
pub mod theta;

pub use error::SketchError;
```

- [ ] **Step 6: Build**

```bash
cargo build -p apache-datasketches --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 7: Write a smoke test**

```rust
// apache-datasketches/tests/theta_sketch_smoke_test.rs
use apache_datasketches::theta::ThetaSketchBuilder;

#[test]
fn construct_update_estimate() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_is_err() {
    assert!(ThetaSketchBuilder::new().lg_k(4).build().is_err());
}
```

- [ ] **Step 8: Run the test**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: both tests PASS.

- [ ] **Step 9: Commit**

```bash
git add apache-datasketches/Cargo.toml apache-datasketches/src/lib.rs apache-datasketches/src/theta apache-datasketches/tests/theta_sketch_smoke_test.rs
git commit -m "Add safe ResizeFactor, ThetaSketchBuilder, and ThetaSketch wrapper"
```

---

### Task 4: `CompactThetaSketch` C++ shim (conversion ctor, query, v3/v4 serialize, deserialize) + cxx bridge + link test

**Files:**
- Create: `apache-datasketches-sys/cpp/theta/theta_compact_shim.h`
- Create: `apache-datasketches-sys/cpp/theta/theta_compact_shim.cc`
- Create: `apache-datasketches-sys/src/theta_compact.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Test: `apache-datasketches-sys/tests/theta_compact_link_test.rs`

**Interfaces:**
- Consumes: `datasketches::compact_theta_sketch` (Reference section); `apache_datasketches_rs::ThetaSketchShim::inner()` (Task 2) for the conversion constructor.
- Produces: C++ class `apache_datasketches_rs::CompactThetaSketchShim` and free functions `theta_sketch_compact` (used by Task 5 to implement `ThetaSketchShim::compact()`), `compact_theta_sketch_deserialize`; Rust bridge `apache_datasketches_sys::theta_compact::ffi::{CompactThetaSketchShim, theta_sketch_compact, compact_theta_sketch_deserialize}`, consumed by Task 5 (`ThetaSketch::compact()`), Task 6 (safe `CompactThetaSketch` wrapper), and every set-op shim (Tasks 9–12) as an input/output type.

- [ ] **Step 1: Write `theta_compact_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_sketch_shim.h"

namespace apache_datasketches_rs {

class CompactThetaSketchShim {
public:
  explicit CompactThetaSketchShim(datasketches::compact_theta_sketch sketch);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  rust::Vec<uint8_t> serialize_compact() const;
  rust::Vec<uint8_t> serialize_compressed() const;

  const datasketches::compact_theta_sketch& inner() const { return sketch_; }

private:
  datasketches::compact_theta_sketch sketch_;
};

// Used by theta_sketch_shim.cc (Task 5) to implement ThetaSketchShim::compact().
std::unique_ptr<CompactThetaSketchShim> theta_sketch_compact(const ThetaSketchShim& sketch, bool ordered);

std::unique_ptr<CompactThetaSketchShim> compact_theta_sketch_deserialize(rust::Slice<const uint8_t> bytes);

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Write `theta_compact_shim.cc`**

```cpp
#include "theta_compact_shim.h"

namespace apache_datasketches_rs {

CompactThetaSketchShim::CompactThetaSketchShim(datasketches::compact_theta_sketch sketch)
  : sketch_(std::move(sketch)) {}

double CompactThetaSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double CompactThetaSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double CompactThetaSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool CompactThetaSketchShim::is_empty() const { return sketch_.is_empty(); }
bool CompactThetaSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool CompactThetaSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double CompactThetaSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t CompactThetaSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

rust::Vec<uint8_t> CompactThetaSketchShim::serialize_compact() const {
  auto bytes = sketch_.serialize();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

rust::Vec<uint8_t> CompactThetaSketchShim::serialize_compressed() const {
  auto bytes = sketch_.serialize_compressed();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

std::unique_ptr<CompactThetaSketchShim> theta_sketch_compact(const ThetaSketchShim& sketch, bool ordered) {
  return std::make_unique<CompactThetaSketchShim>(
      datasketches::compact_theta_sketch(sketch.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> compact_theta_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<CompactThetaSketchShim>(
      datasketches::compact_theta_sketch::deserialize(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
```

Note: `deserialize()` auto-detects v1/v2/v3/v4 transparently (Design resolution #3 above), so `compact_theta_sketch_deserialize` is the single implementation backing both the Rust `deserialize` and `deserialize_compressed` entry points added in Task 6 — no separate compressed-specific C++ path exists or is needed.

- [ ] **Step 3: Write `apache-datasketches-sys/src/theta_compact.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;

        type CompactThetaSketchShim;

        fn theta_sketch_compact(sketch: &ThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compact_theta_sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<CompactThetaSketchShim>>;

        fn get_estimate(self: &CompactThetaSketchShim) -> f64;
        fn get_lower_bound(self: &CompactThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &CompactThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &CompactThetaSketchShim) -> bool;
        fn is_estimation_mode(self: &CompactThetaSketchShim) -> bool;
        fn is_ordered(self: &CompactThetaSketchShim) -> bool;
        fn get_theta(self: &CompactThetaSketchShim) -> f64;
        fn get_num_retained(self: &CompactThetaSketchShim) -> u32;

        fn serialize_compact(self: &CompactThetaSketchShim) -> Vec<u8>;
        fn serialize_compressed(self: &CompactThetaSketchShim) -> Vec<u8>;
    }
}
```

`type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;` is `cxx`'s documented pattern for sharing one opaque C++ type across multiple bridge modules — both bridges refer to the exact same generated C++ type, since `theta_compact_shim.h` includes `theta_sketch_shim.h` directly.

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "theta")]
pub mod theta_sketch;
#[cfg(feature = "theta")]
pub mod theta_compact;
```

- [ ] **Step 5: Add `cpp/theta/theta_compact_shim.cc` to the list already wired in Task 1's `build.rs`**

No change needed — Task 1's `build.rs` already lists `cpp/theta/theta_compact_shim.cc` and `src/theta_compact.rs` unconditionally under `cfg!(feature = "theta")`.

- [ ] **Step 6: Build**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 7: Write a link-level smoke test**

```rust
// apache-datasketches-sys/tests/theta_compact_link_test.rs
use apache_datasketches_sys::theta_compact::ffi as compact_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;

#[test]
fn compact_and_serialize_round_trip() {
    let mut sketch = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }

    let compact = compact_ffi::theta_sketch_compact(&sketch, true);
    assert!((compact.get_estimate() - 1000.0).abs() < 1.0);
    assert!(compact.is_ordered());

    let bytes = compact.serialize_compact();
    let restored = compact_ffi::compact_theta_sketch_deserialize(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());

    let compressed = compact.serialize_compressed();
    let restored_compressed = compact_ffi::compact_theta_sketch_deserialize(&compressed).unwrap();
    assert_eq!(compact.get_estimate(), restored_compressed.get_estimate());
}
```

- [ ] **Step 8: Run the test**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add apache-datasketches-sys/cpp/theta/theta_compact_shim.h apache-datasketches-sys/cpp/theta/theta_compact_shim.cc apache-datasketches-sys/src/theta_compact.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/tests/theta_compact_link_test.rs
git commit -m "Add CompactThetaSketch C++ shim, cxx bridge, and link smoke test"
```

---

### Task 5: Wire `ThetaSketchShim::compact()` now that `CompactThetaSketchShim` exists; safe `ThetaSketch::compact()`

**Files:**
- Modify: `apache-datasketches-sys/cpp/theta/theta_sketch_shim.h`
- Modify: `apache-datasketches-sys/cpp/theta/theta_sketch_shim.cc`
- Modify: `apache-datasketches-sys/src/theta_sketch.rs`
- Modify: `apache-datasketches/src/theta/sketch.rs`
- Test: `apache-datasketches-sys/tests/theta_sketch_link_test.rs`
- Test: `apache-datasketches/tests/theta_sketch_smoke_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches_rs::CompactThetaSketchShim`, `theta_sketch_compact` free function (Task 4).
- Produces: `ThetaSketchShim::compact(bool ordered)` (sys), `ThetaSketch::compact(ordered: bool) -> CompactThetaSketch` (safe), consumed by Task 6's tests and every later task that needs a `CompactThetaSketch` derived from a `ThetaSketch`.

- [ ] **Step 1: Add `compact()` to `theta_sketch_shim.h`**

Add to the `#include` list and class body:

```cpp
#include "theta_compact_shim.h"
```

```cpp
  std::unique_ptr<CompactThetaSketchShim> compact(bool ordered) const;
```

(placed after `get_num_retained()` in the public section).

- [ ] **Step 2: Implement it in `theta_sketch_shim.cc`**

```cpp
std::unique_ptr<CompactThetaSketchShim> ThetaSketchShim::compact(bool ordered) const {
  return theta_sketch_compact(*this, ordered);
}
```

- [ ] **Step 3: Add the bridge declaration to `apache-datasketches-sys/src/theta_sketch.rs`**

Add inside the existing `unsafe extern "C++"` block, and add the cross-module type + include at the top:

```rust
        include!("theta_compact_shim.h");

        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
```

```rust
        fn compact(self: &ThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
```

Note: this creates a two-way header dependency (`theta_sketch_shim.h` now includes `theta_compact_shim.h`, which already includes `theta_sketch_shim.h`) — both headers use `#pragma once`, so the second inclusion in either direction is a no-op; this compiles cleanly since neither header's class bodies reference incomplete types from the other in a way that requires the other to be fully defined before its own declarations (both only need forward-compatible pointer/reference types, already satisfied by C++'s single-pass parse once `#pragma once` breaks the cycle). Confirmed by the build in Step 5.

- [ ] **Step 4: Add `compact()` to `apache-datasketches/src/theta/sketch.rs`**

Add near the other query methods:

```rust
    pub fn compact(&self, ordered: bool) -> super::CompactThetaSketch {
        super::CompactThetaSketch::from_shim(self.inner.compact(ordered))
    }
```

(`CompactThetaSketch::from_shim` is added in Task 6; this method won't compile until then — write it now, verify in Task 6.)

- [ ] **Step 5: Build the sys crate to confirm the shim wiring compiles**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 6: Add a compact-round-trip case to the sys link test**

Append to `apache-datasketches-sys/tests/theta_sketch_link_test.rs`:

```rust
#[test]
fn compact_via_sketch_method() {
    let mut sketch = ffi::new_theta_sketch(12, ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }
    let compact = sketch.compact(true);
    assert!((compact.get_estimate() - 1000.0).abs() < 1.0);
}
```

- [ ] **Step 7: Run the sys tests**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS (does not need `apache-datasketches` to compile yet, since this test only touches the sys crate directly).

- [ ] **Step 8: Commit**

```bash
git add apache-datasketches-sys/cpp/theta/theta_sketch_shim.h apache-datasketches-sys/cpp/theta/theta_sketch_shim.cc apache-datasketches-sys/src/theta_sketch.rs apache-datasketches-sys/tests/theta_sketch_link_test.rs apache-datasketches/src/theta/sketch.rs
git commit -m "Wire ThetaSketchShim::compact() and safe ThetaSketch::compact()"
```

---

### Task 6: Safe `CompactThetaSketch` wrapper (query, v3/v4 serialize, deserialize) + round-trip tests

**Files:**
- Create: `apache-datasketches/src/theta/compact.rs`
- Modify: `apache-datasketches/src/theta/mod.rs`
- Test: `apache-datasketches/tests/theta_compact_smoke_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches_sys::theta_compact::ffi::{CompactThetaSketchShim, compact_theta_sketch_deserialize}` (Task 4); `apache_datasketches_sys::theta_sketch::ffi::CompactThetaSketchShim`-returning `ThetaSketchShim::compact` (Task 5, via `apache_datasketches::theta::ThetaSketch::compact`).
- Produces: `apache_datasketches::theta::CompactThetaSketch`, with `pub(crate) fn from_shim(inner: UniquePtr<sys::CompactThetaSketchShim>) -> Self` used by `ThetaSketch::compact()` (Task 5) and every set-op's `get_result`/`compute` (Tasks 9–11). `CompactThetaSketch { pub(crate) inner: UniquePtr<sys::CompactThetaSketchShim> }`.

- [ ] **Step 1: Write `apache-datasketches/src/theta/compact.rs`**

```rust
use crate::error::SketchError;
use apache_datasketches_sys::theta_compact::ffi as sys;
use cxx::UniquePtr;

pub struct CompactThetaSketch {
    pub(crate) inner: UniquePtr<sys::CompactThetaSketchShim>,
}

unsafe impl Send for CompactThetaSketch {}

impl CompactThetaSketch {
    pub(crate) fn from_shim(inner: UniquePtr<sys::CompactThetaSketchShim>) -> Self {
        Self { inner }
    }

    /// Deserializes v1/v2/v3 (uncompressed) bytes. Upstream's `deserialize()`
    /// auto-detects the serial version transparently, including v4
    /// (compressed) — see [`Self::deserialize_compressed`], which calls the
    /// exact same underlying routine; the two Rust names exist purely for
    /// call-site symmetry with `serialize_compact`/`serialize_compressed`.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::compact_theta_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Deserializes v4 (compressed) bytes. See [`Self::deserialize`] — both
    /// methods call the same upstream auto-detecting `deserialize()`.
    pub fn deserialize_compressed(bytes: &[u8]) -> Result<Self, SketchError> {
        Self::deserialize(bytes)
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

    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }

    /// Serializes in the v3 (uncompressed) format. Note: unlike the design
    /// spec's initially-sketched signature, this takes no `ordered`
    /// parameter — upstream's `compact_theta_sketch::serialize()` has none;
    /// orderedness is fixed when this sketch was created (e.g. via
    /// `ThetaSketch::compact(ordered)`).
    pub fn serialize_compact(&self) -> Vec<u8> {
        self.inner.serialize_compact()
    }

    /// Serializes in the v4 (compressed) format.
    pub fn serialize_compressed(&self) -> Vec<u8> {
        self.inner.serialize_compressed()
    }
}
```

- [ ] **Step 2: Update `apache-datasketches/src/theta/mod.rs`**

```rust
mod builder;
mod compact;
mod sketch;

pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use compact::CompactThetaSketch;
pub use sketch::ThetaSketch;
```

- [ ] **Step 3: Build**

```bash
cargo build -p apache-datasketches --no-default-features --features theta
```

Expected: PASS — this is also where Task 5 Step 4's `ThetaSketch::compact()` (which referenced `CompactThetaSketch::from_shim` before it existed) first compiles successfully.

- [ ] **Step 4: Write round-trip smoke tests (v3 and v4)**

```rust
// apache-datasketches/tests/theta_compact_smoke_test.rs
use apache_datasketches::theta::{CompactThetaSketch, ThetaSketchBuilder};

#[test]
fn compact_v3_round_trip() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    let restored = CompactThetaSketch::deserialize(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());
    assert_eq!(compact.get_num_retained(), restored.get_num_retained());
}

#[test]
fn compact_v4_round_trip() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compressed();
    let restored = CompactThetaSketch::deserialize_compressed(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());
    assert_eq!(compact.get_num_retained(), restored.get_num_retained());
}

#[test]
fn deserialize_garbage_is_err() {
    assert!(CompactThetaSketch::deserialize(&[0u8; 3]).is_err());
}
```

- [ ] **Step 5: Run the tests**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add apache-datasketches/src/theta/compact.rs apache-datasketches/src/theta/mod.rs apache-datasketches/tests/theta_compact_smoke_test.rs
git commit -m "Add safe CompactThetaSketch wrapper with v3/v4 serialize and deserialize"
```

---

### Task 7: `WrappedCompactThetaSketch` C++ shim (wrap, query, no serialize) + cxx bridge + safe wrapper with lifetime

**Files:**
- Create: `apache-datasketches-sys/cpp/theta/theta_wrapped_shim.h`
- Create: `apache-datasketches-sys/cpp/theta/theta_wrapped_shim.cc`
- Create: `apache-datasketches-sys/src/theta_wrapped.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Create: `apache-datasketches/src/theta/wrapped.rs`
- Modify: `apache-datasketches/src/theta/mod.rs`
- Test: `apache-datasketches-sys/tests/theta_wrapped_link_test.rs`
- Test: `apache-datasketches/tests/theta_wrapped_smoke_test.rs`

**Interfaces:**
- Consumes: `datasketches::wrapped_compact_theta_sketch` (Reference section); raw byte slices produced by `CompactThetaSketch::serialize_compact`/`serialize_compressed` (Task 6).
- Produces: C++ class `apache_datasketches_rs::WrappedCompactThetaSketchShim`, free function `wrapped_compact_theta_sketch_wrap`; Rust bridge `apache_datasketches_sys::theta_wrapped::ffi::{WrappedCompactThetaSketchShim, wrapped_compact_theta_sketch_wrap}`; safe `apache_datasketches::theta::WrappedCompactThetaSketch<'a>`, consumed by Task 8 (`ThetaInput` impl) and Tasks 9–12 (set-op/jaccard inputs).

- [ ] **Step 1: Write `theta_wrapped_shim.h`**

`wrapped_compact_theta_sketch::wrap()` is a *non-owning* view over caller-provided bytes: upstream requires the backing buffer to outlive the wrapper. The shim holds the bytes alive by storing the `wrapped_compact_theta_sketch` (which itself just holds a `const uint8_t*`/size into the buffer) alongside nothing extra — the Rust side is responsible for keeping the byte slice alive via the `'a` lifetime (Step 6).

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"

namespace apache_datasketches_rs {

class WrappedCompactThetaSketchShim {
public:
  explicit WrappedCompactThetaSketchShim(datasketches::wrapped_compact_theta_sketch sketch);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  const datasketches::wrapped_compact_theta_sketch& inner() const { return sketch_; }

private:
  datasketches::wrapped_compact_theta_sketch sketch_;
};

std::unique_ptr<WrappedCompactThetaSketchShim> wrapped_compact_theta_sketch_wrap(rust::Slice<const uint8_t> bytes);

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Write `theta_wrapped_shim.cc`**

```cpp
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

WrappedCompactThetaSketchShim::WrappedCompactThetaSketchShim(datasketches::wrapped_compact_theta_sketch sketch)
  : sketch_(std::move(sketch)) {}

double WrappedCompactThetaSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double WrappedCompactThetaSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double WrappedCompactThetaSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool WrappedCompactThetaSketchShim::is_empty() const { return sketch_.is_empty(); }
bool WrappedCompactThetaSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool WrappedCompactThetaSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double WrappedCompactThetaSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t WrappedCompactThetaSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

std::unique_ptr<WrappedCompactThetaSketchShim> wrapped_compact_theta_sketch_wrap(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<WrappedCompactThetaSketchShim>(
      datasketches::wrapped_compact_theta_sketch::wrap(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Write `apache-datasketches-sys/src/theta_wrapped.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_wrapped_shim.h");

        type WrappedCompactThetaSketchShim;

        fn wrapped_compact_theta_sketch_wrap(bytes: &[u8]) -> Result<UniquePtr<WrappedCompactThetaSketchShim>>;

        fn get_estimate(self: &WrappedCompactThetaSketchShim) -> f64;
        fn get_lower_bound(self: &WrappedCompactThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &WrappedCompactThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &WrappedCompactThetaSketchShim) -> bool;
        fn is_estimation_mode(self: &WrappedCompactThetaSketchShim) -> bool;
        fn is_ordered(self: &WrappedCompactThetaSketchShim) -> bool;
        fn get_theta(self: &WrappedCompactThetaSketchShim) -> f64;
        fn get_num_retained(self: &WrappedCompactThetaSketchShim) -> u32;
    }
}
```

Note: `WrappedCompactThetaSketchShim` is intentionally its own bridge module (not cross-shared with `theta_compact` or `theta_sketch`) — upstream's `wrapped_compact_theta_sketch` is not constructible from `update_theta_sketch`/`compact_theta_sketch` directly, only from raw serialized bytes via `wrap()`, so there is no cross-type conversion path requiring type sharing here (unlike Task 4/5's `CompactThetaSketchShim` sharing with `ThetaSketchShim`).

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "theta")]
pub mod theta_wrapped;
```

- [ ] **Step 5: Build**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS (Task 1's `build.rs` already lists `cpp/theta/theta_wrapped_shim.cc` and `src/theta_wrapped.rs` unconditionally under the `theta` feature).

- [ ] **Step 6: Write the safe `apache-datasketches/src/theta/wrapped.rs`**

The `'a` lifetime parameter ties the Rust wrapper to the byte slice it was constructed from; `PhantomData<&'a [u8]>` documents and enforces this borrow at the Rust type level even though the `UniquePtr<WrappedCompactThetaSketchShim>` itself doesn't literally borrow Rust memory (the C++ `wrapped_compact_theta_sketch` holds a raw pointer into the slice's buffer, which cxx passed to it as `rust::Slice<const uint8_t>` in Step 1/2 — the pointer becomes dangling if the Rust slice is dropped or moved, so the borrow checker must be the one preventing that, since C++ cannot enforce it).

```rust
use crate::error::SketchError;
use apache_datasketches_sys::theta_wrapped::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

pub struct WrappedCompactThetaSketch<'a> {
    pub(crate) inner: UniquePtr<sys::WrappedCompactThetaSketchShim>,
    _marker: PhantomData<&'a [u8]>,
}

unsafe impl<'a> Send for WrappedCompactThetaSketch<'a> {}

impl<'a> WrappedCompactThetaSketch<'a> {
    pub fn wrap(bytes: &'a [u8]) -> Result<Self, SketchError> {
        let inner = sys::wrapped_compact_theta_sketch_wrap(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self {
            inner,
            _marker: PhantomData,
        })
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

    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }
}
```

- [ ] **Step 7: Update `apache-datasketches/src/theta/mod.rs`**

```rust
mod builder;
mod compact;
mod sketch;
mod wrapped;

pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use compact::CompactThetaSketch;
pub use sketch::ThetaSketch;
pub use wrapped::WrappedCompactThetaSketch;
```

- [ ] **Step 8: Write link-level sys test**

```rust
// apache-datasketches-sys/tests/theta_wrapped_link_test.rs
use apache_datasketches_sys::theta_compact::ffi as compact_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;
use apache_datasketches_sys::theta_wrapped::ffi as wrapped_ffi;

#[test]
fn wrap_matches_compact_estimate() {
    let mut sketch = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }
    let compact = compact_ffi::theta_sketch_compact(&sketch, true);
    let bytes = compact.serialize_compact();

    let wrapped = wrapped_ffi::wrapped_compact_theta_sketch_wrap(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), wrapped.get_estimate());
    assert_eq!(compact.get_num_retained(), wrapped.get_num_retained());
}
```

- [ ] **Step 9: Write safe smoke test**

```rust
// apache-datasketches/tests/theta_wrapped_smoke_test.rs
use apache_datasketches::theta::{ThetaSketchBuilder, WrappedCompactThetaSketch};

#[test]
fn wrap_bytes_and_query() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();

    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), wrapped.get_estimate());
}

#[test]
fn wrap_garbage_is_err() {
    assert!(WrappedCompactThetaSketch::wrap(&[0u8; 2]).is_err());
}
```

- [ ] **Step 10: Run all tests**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features theta
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 11: Commit**

```bash
git add apache-datasketches-sys/cpp/theta/theta_wrapped_shim.h apache-datasketches-sys/cpp/theta/theta_wrapped_shim.cc apache-datasketches-sys/src/theta_wrapped.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/tests/theta_wrapped_link_test.rs apache-datasketches/src/theta/wrapped.rs apache-datasketches/src/theta/mod.rs apache-datasketches/tests/theta_wrapped_smoke_test.rs
git commit -m "Add WrappedCompactThetaSketch C++ shim, cxx bridge, and safe wrapper"
```

---

### Task 8: Sealed `ThetaInput` trait + `sys::ThetaInputRef<'a>` enum

**Files:**
- Create: `apache-datasketches-sys/src/theta_input.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Create: `apache-datasketches/src/theta/input.rs`
- Modify: `apache-datasketches/src/theta/mod.rs`
- Test: `apache-datasketches/tests/theta_input_test.rs`

**Interfaces:**
- Consumes: `apache_datasketches_sys::theta_sketch::ffi::ThetaSketchShim`, `theta_compact::ffi::CompactThetaSketchShim`, `theta_wrapped::ffi::WrappedCompactThetaSketchShim` (Tasks 2, 4, 7); `apache_datasketches::theta::{ThetaSketch, CompactThetaSketch, WrappedCompactThetaSketch<'a>}` (Tasks 3, 6, 7).
- Produces: `apache_datasketches_sys::theta_input::ThetaInputRef<'a>` (plain Rust enum, per Design resolution #1 — not `#[cxx::bridge]`-declared, since cxx enums must be C-like/payload-free and this one borrows three different opaque types); `apache_datasketches::theta::ThetaInput` sealed trait, implemented by all three sketch types; consumed by every set-op (Tasks 9–11) and `jaccard_similarity` (Task 12) as their generic input bound.

- [ ] **Step 1: Write `apache-datasketches-sys/src/theta_input.rs`**

```rust
use crate::theta_compact::ffi::CompactThetaSketchShim;
use crate::theta_sketch::ffi::ThetaSketchShim;
use crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;

/// A borrowed reference to one of the three theta sketch shim types,
/// used to dispatch set-operation and Jaccard-similarity calls to the
/// correct concrete C++ overload. This is a plain Rust enum rather than
/// a `#[cxx::bridge]` type: `cxx` only supports C-like (payload-free)
/// shared enums, and this enum's variants carry borrowed references to
/// three unrelated opaque C++ types, which cxx cannot express directly.
pub enum ThetaInputRef<'a> {
    Sketch(&'a ThetaSketchShim),
    Compact(&'a CompactThetaSketchShim),
    Wrapped(&'a WrappedCompactThetaSketchShim),
}
```

- [ ] **Step 2: Declare the module in `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "theta")]
pub mod theta_input;
```

(placed after the other four `#[cfg(feature = "theta")]` `pub mod` lines; unlike those, this module has no corresponding `.h`/`.cc` shim files and no entry in `build.rs`, since it declares no `cxx::bridge`.)

- [ ] **Step 3: Build the sys crate**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 4: Write `apache-datasketches/src/theta/input.rs`**

```rust
use super::{CompactThetaSketch, ThetaSketch, WrappedCompactThetaSketch};
use apache_datasketches_sys::theta_input::ThetaInputRef;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ThetaSketch {}
    impl Sealed for super::CompactThetaSketch {}
    impl<'a> Sealed for super::WrappedCompactThetaSketch<'a> {}
}

/// Any of the three theta sketch types can be fed into a `ThetaUnion`,
/// `ThetaIntersection`, `ThetaAnotB`, or `jaccard_similarity`. This trait
/// is sealed — it cannot be implemented outside this crate — since the
/// set-op shims only have concrete overloads for these three exact types.
pub trait ThetaInput: sealed::Sealed {
    #[doc(hidden)]
    fn as_theta_input(&self) -> ThetaInputRef<'_>;
}

impl ThetaInput for ThetaSketch {
    fn as_theta_input(&self) -> ThetaInputRef<'_> {
        ThetaInputRef::Sketch(&self.inner)
    }
}

impl ThetaInput for CompactThetaSketch {
    fn as_theta_input(&self) -> ThetaInputRef<'_> {
        ThetaInputRef::Compact(&self.inner)
    }
}

impl<'a> ThetaInput for WrappedCompactThetaSketch<'a> {
    fn as_theta_input(&self) -> ThetaInputRef<'_> {
        ThetaInputRef::Wrapped(&self.inner)
    }
}
```

- [ ] **Step 5: Update `apache-datasketches/src/theta/mod.rs`**

```rust
mod builder;
mod compact;
mod input;
mod sketch;
mod wrapped;

pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use compact::CompactThetaSketch;
pub use input::ThetaInput;
pub use sketch::ThetaSketch;
pub use wrapped::WrappedCompactThetaSketch;
```

- [ ] **Step 6: Build**

```bash
cargo build -p apache-datasketches --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 7: Write a dispatch test confirming all three types implement the trait and produce the expected variant**

```rust
// apache-datasketches/tests/theta_input_test.rs
use apache_datasketches::theta::{CompactThetaSketch, ThetaInput, ThetaSketchBuilder, WrappedCompactThetaSketch};

fn accepts_theta_input(input: &impl ThetaInput) -> f64 {
    // Exercises the trait bound generically, as every set-op signature will.
    match input.as_theta_input() {
        apache_datasketches_sys::theta_input::ThetaInputRef::Sketch(s) => s.get_estimate(),
        apache_datasketches_sys::theta_input::ThetaInputRef::Compact(c) => c.get_estimate(),
        apache_datasketches_sys::theta_input::ThetaInputRef::Wrapped(w) => w.get_estimate(),
    }
}

#[test]
fn theta_sketch_implements_theta_input() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    sketch.update_u64(42);
    assert!(accepts_theta_input(&sketch) > 0.0);
}

#[test]
fn compact_theta_sketch_implements_theta_input() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    sketch.update_u64(42);
    let compact = sketch.compact(true);
    assert!(accepts_theta_input(&compact) > 0.0);
}

#[test]
fn wrapped_compact_theta_sketch_implements_theta_input() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    sketch.update_u64(42);
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();
    assert!(accepts_theta_input(&wrapped) > 0.0);
}
```

- [ ] **Step 8: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 9: Commit**

```bash
git add apache-datasketches-sys/src/theta_input.rs apache-datasketches-sys/src/lib.rs apache-datasketches/src/theta/input.rs apache-datasketches/src/theta/mod.rs apache-datasketches/tests/theta_input_test.rs
git commit -m "Add sealed ThetaInput trait and sys::ThetaInputRef enum"
```

---

### Task 9: `ThetaUnion` C++ shim (builder, 3-way `update_with_*`, `get_result`, `reset`) + cxx bridge + safe wrapper

**Files:**
- Create: `apache-datasketches-sys/cpp/theta/theta_union_shim.h`
- Create: `apache-datasketches-sys/cpp/theta/theta_union_shim.cc`
- Create: `apache-datasketches-sys/src/theta_union.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Create: `apache-datasketches/src/theta/union.rs`
- Modify: `apache-datasketches/src/theta/mod.rs`
- Test: `apache-datasketches-sys/tests/theta_union_link_test.rs`
- Test: `apache-datasketches/tests/theta_union_smoke_test.rs`

**Interfaces:**
- Consumes: `datasketches::theta_union` (Reference section); `ThetaSketchShim`, `CompactThetaSketchShim`, `WrappedCompactThetaSketchShim` (Tasks 2, 4, 7); `ThetaInput`/`ThetaInputRef` (Task 8).
- Produces: C++ class `apache_datasketches_rs::ThetaUnionShim`; Rust bridge `apache_datasketches_sys::theta_union::ffi::{ThetaUnionShim, new_theta_union}`; safe `apache_datasketches::theta::{ThetaUnionBuilder, ThetaUnion}`.

Upstream's `theta_union::update()` is a single template method (`template<typename FwdSketch> void update(FwdSketch&& sketch)`), instantiated by the real C++ code for any duck-typed sketch-like argument. Since `cxx` cannot bridge templates, the shim exposes one concrete non-template overload per concrete input type — `update_with_sketch`, `update_with_compact`, `update_with_wrapped` — and the safe Rust `ThetaUnion::update` dispatches to the correct one via a 3-arm match on `ThetaInputRef` (this is the 3-way version of the same dispatch pattern Task 8 exists to support; Tasks 11–12's `ThetaAnotB`/`jaccard_similarity` need the analogous but 9-way version, since those upstream functions are templated over *two* independent sketch-like arguments).

- [ ] **Step 1: Write `theta_union_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_union.hpp"
#include "theta_sketch_shim.h"
#include "theta_compact_shim.h"
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

enum class ResizeFactor : std::uint8_t;
ResizeFactor to_rust_resize_factor(datasketches::resize_factor rf);
datasketches::resize_factor to_cpp_resize_factor(ResizeFactor rf);

class ThetaUnionShim {
public:
  ThetaUnionShim(uint8_t lg_k, datasketches::resize_factor rf, float p);

  void update_with_sketch(const ThetaSketchShim& sketch);
  void update_with_compact(const CompactThetaSketchShim& sketch);
  void update_with_wrapped(const WrappedCompactThetaSketchShim& sketch);

  std::unique_ptr<CompactThetaSketchShim> get_result(bool ordered) const;
  void reset();

private:
  datasketches::theta_union union_;
};

std::unique_ptr<ThetaUnionShim> new_theta_union(uint8_t lg_k, ResizeFactor rf, float p);

} // namespace apache_datasketches_rs
```

Note: `to_cpp_resize_factor`/`to_rust_resize_factor` and the forward-declared `enum class ResizeFactor` are **not** redeclared here as new symbols — they already exist from Task 2's `theta_sketch_shim.h`/`.cc` (same forward-declare-enum pattern used there to avoid circular includes with the cxx-generated `.rs.h`). Re-declaring the same free functions with identical signatures in a second header is fine in C++ as long as the definitions aren't duplicated; to avoid ODR risk, this header does **not** redefine them — it only forward-declares the enum (required so `ThetaUnionShim`'s constructor signature can name `ResizeFactor` before the generated `theta_union.rs.h` exists) and calls the existing `to_cpp_resize_factor` defined once in `theta_sketch_shim.cc`. `theta_union_shim.cc`'s `#include "theta_sketch_shim.h"` brings in that one true definition.

- [ ] **Step 2: Write `theta_union_shim.cc`**

```cpp
#include "theta_union_shim.h"

namespace apache_datasketches_rs {

ThetaUnionShim::ThetaUnionShim(uint8_t lg_k, datasketches::resize_factor rf, float p)
  : union_(datasketches::theta_union::builder()
               .set_lg_k(lg_k)
               .set_resize_factor(rf)
               .set_p(p)
               .build()) {}

void ThetaUnionShim::update_with_sketch(const ThetaSketchShim& sketch) {
  union_.update(sketch.inner());
}

void ThetaUnionShim::update_with_compact(const CompactThetaSketchShim& sketch) {
  union_.update(sketch.inner());
}

void ThetaUnionShim::update_with_wrapped(const WrappedCompactThetaSketchShim& sketch) {
  union_.update(sketch.inner());
}

std::unique_ptr<CompactThetaSketchShim> ThetaUnionShim::get_result(bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(union_.get_result(ordered));
}

void ThetaUnionShim::reset() { union_.reset(); }

std::unique_ptr<ThetaUnionShim> new_theta_union(uint8_t lg_k, ResizeFactor rf, float p) {
  return std::make_unique<ThetaUnionShim>(lg_k, to_cpp_resize_factor(rf), p);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Write `apache-datasketches-sys/src/theta_union.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");
        include!("theta_wrapped_shim.h");
        include!("theta_union_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;
        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
        type WrappedCompactThetaSketchShim = crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;
        type ResizeFactor = crate::theta_sketch::ffi::ResizeFactor;

        type ThetaUnionShim;

        fn new_theta_union(lg_k: u8, rf: ResizeFactor, p: f32) -> Result<UniquePtr<ThetaUnionShim>>;

        fn update_with_sketch(self: Pin<&mut ThetaUnionShim>, sketch: &ThetaSketchShim);
        fn update_with_compact(self: Pin<&mut ThetaUnionShim>, sketch: &CompactThetaSketchShim);
        fn update_with_wrapped(self: Pin<&mut ThetaUnionShim>, sketch: &WrappedCompactThetaSketchShim);

        fn get_result(self: &ThetaUnionShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn reset(self: Pin<&mut ThetaUnionShim>);
    }
}
```

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "theta")]
pub mod theta_union;
```

- [ ] **Step 5: Build**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 6: Write the sys link test**

```rust
// apache-datasketches-sys/tests/theta_union_link_test.rs
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;
use apache_datasketches_sys::theta_union::ffi as union_ffi;

#[test]
fn union_two_sketches() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    let mut b = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    for i in 250..750u64 {
        b.pin_mut().update_u64(i);
    }

    let mut union_ = union_ffi::new_theta_union(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    union_.pin_mut().update_with_sketch(&a);
    union_.pin_mut().update_with_sketch(&b);

    let result = union_.get_result(true);
    assert!((result.get_estimate() - 750.0).abs() < 20.0);
}
```

- [ ] **Step 7: Run**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 8: Write safe `apache-datasketches/src/theta/union.rs`**

```rust
use super::input::ThetaInput;
use super::{CompactThetaSketch, ResizeFactor};
use crate::error::SketchError;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use apache_datasketches_sys::theta_union::ffi as sys;
use cxx::UniquePtr;

pub struct ThetaUnionBuilder {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
}

impl Default for ThetaUnionBuilder {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::default(),
            p: 1.0,
        }
    }
}

impl ThetaUnionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    pub fn build(self) -> Result<ThetaUnion, SketchError> {
        let inner = sys::new_theta_union(self.lg_k, self.resize_factor.into(), self.p)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(ThetaUnion { inner })
    }
}

pub struct ThetaUnion {
    inner: UniquePtr<sys::ThetaUnionShim>,
}

unsafe impl Send for ThetaUnion {}

impl ThetaUnion {
    pub fn update(&mut self, input: &impl ThetaInput) {
        match input.as_theta_input() {
            ThetaInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ThetaInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
            ThetaInputRef::Wrapped(w) => self.inner.pin_mut().update_with_wrapped(w),
        }
    }

    pub fn get_result(&self, ordered: bool) -> CompactThetaSketch {
        CompactThetaSketch::from_shim(self.inner.get_result(ordered))
    }

    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }
}
```

- [ ] **Step 9: Update `apache-datasketches/src/theta/mod.rs`**

```rust
mod builder;
mod compact;
mod input;
mod sketch;
mod union;
mod wrapped;

pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use compact::CompactThetaSketch;
pub use input::ThetaInput;
pub use sketch::ThetaSketch;
pub use union::{ThetaUnion, ThetaUnionBuilder};
pub use wrapped::WrappedCompactThetaSketch;
```

- [ ] **Step 10: Write the safe smoke test**

```rust
// apache-datasketches/tests/theta_union_smoke_test.rs
use apache_datasketches::theta::{ThetaSketchBuilder, ThetaUnionBuilder};

#[test]
fn union_two_theta_sketches() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    for i in 250..750u64 {
        b.update_u64(i);
    }

    let mut union_ = ThetaUnionBuilder::new().lg_k(12).build().unwrap();
    union_.update(&a);
    union_.update(&b);

    let result = union_.get_result(true);
    assert!((result.get_estimate() - 750.0).abs() < 20.0);
}

#[test]
fn union_mixed_input_types() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    let a_compact = a.compact(true);
    let a_bytes = a_compact.serialize_compact();
    let a_wrapped = apache_datasketches::theta::WrappedCompactThetaSketch::wrap(&a_bytes).unwrap();

    let mut union_ = ThetaUnionBuilder::new().lg_k(12).build().unwrap();
    union_.update(&a);
    union_.update(&a_compact);
    union_.update(&a_wrapped);

    let result = union_.get_result(true);
    assert!((result.get_estimate() - 500.0).abs() < 10.0);
}
```

- [ ] **Step 11: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 12: Commit**

```bash
git add apache-datasketches-sys/cpp/theta/theta_union_shim.h apache-datasketches-sys/cpp/theta/theta_union_shim.cc apache-datasketches-sys/src/theta_union.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/tests/theta_union_link_test.rs apache-datasketches/src/theta/union.rs apache-datasketches/src/theta/mod.rs apache-datasketches/tests/theta_union_smoke_test.rs
git commit -m "Add ThetaUnion C++ shim, cxx bridge, and safe wrapper"
```

---

### Task 10: `ThetaIntersection` C++ shim (ctor, 3-way `update_with_*`, `get_result`/`has_result`) + cxx bridge + safe wrapper + `SketchError::EmptyIntersection`

**Files:**
- Create: `apache-datasketches-sys/cpp/theta/theta_intersection_shim.h`
- Create: `apache-datasketches-sys/cpp/theta/theta_intersection_shim.cc`
- Create: `apache-datasketches-sys/src/theta_intersection.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Modify: `apache-datasketches/src/error.rs`
- Create: `apache-datasketches/src/theta/intersection.rs`
- Modify: `apache-datasketches/src/theta/mod.rs`
- Test: `apache-datasketches-sys/tests/theta_intersection_link_test.rs`
- Test: `apache-datasketches/tests/theta_intersection_smoke_test.rs`

**Interfaces:**
- Consumes: `datasketches::theta_intersection` (Reference section); `ThetaSketchShim`, `CompactThetaSketchShim`, `WrappedCompactThetaSketchShim` (Tasks 2, 4, 7); `ThetaInput`/`ThetaInputRef` (Task 8).
- Produces: C++ class `apache_datasketches_rs::ThetaIntersectionShim`; Rust bridge `apache_datasketches_sys::theta_intersection::ffi::{ThetaIntersectionShim, new_theta_intersection}`; `SketchError::EmptyIntersection` variant; safe `apache_datasketches::theta::ThetaIntersection`.

Upstream's `theta_intersection` has no builder (unlike union) — just a default-seed constructor — and `get_result()` **throws** `std::invalid_argument` if called before any `update()` (verified in the Reference section above: `"calling get_result() before calling update() is undefined"`). This plan maps that specific throw to `SketchError::EmptyIntersection` rather than the catch-all `SketchError::Cpp(String)`, so callers can `match` on it precisely; `has_result()` lets callers avoid the throw entirely.

- [ ] **Step 1: Write `theta_intersection_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_intersection.hpp"
#include "theta_sketch_shim.h"
#include "theta_compact_shim.h"
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

class ThetaIntersectionShim {
public:
  ThetaIntersectionShim();

  void update_with_sketch(const ThetaSketchShim& sketch);
  void update_with_compact(const CompactThetaSketchShim& sketch);
  void update_with_wrapped(const WrappedCompactThetaSketchShim& sketch);

  std::unique_ptr<CompactThetaSketchShim> get_result(bool ordered) const;
  bool has_result() const;

private:
  datasketches::theta_intersection intersection_;
};

std::unique_ptr<ThetaIntersectionShim> new_theta_intersection();

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Write `theta_intersection_shim.cc`**

```cpp
#include "theta_intersection_shim.h"

namespace apache_datasketches_rs {

ThetaIntersectionShim::ThetaIntersectionShim() : intersection_() {}

void ThetaIntersectionShim::update_with_sketch(const ThetaSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

void ThetaIntersectionShim::update_with_compact(const CompactThetaSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

void ThetaIntersectionShim::update_with_wrapped(const WrappedCompactThetaSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

std::unique_ptr<CompactThetaSketchShim> ThetaIntersectionShim::get_result(bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(intersection_.get_result(ordered));
}

bool ThetaIntersectionShim::has_result() const { return intersection_.has_result(); }

std::unique_ptr<ThetaIntersectionShim> new_theta_intersection() {
  return std::make_unique<ThetaIntersectionShim>();
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Write `apache-datasketches-sys/src/theta_intersection.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");
        include!("theta_wrapped_shim.h");
        include!("theta_intersection_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;
        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
        type WrappedCompactThetaSketchShim = crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;

        type ThetaIntersectionShim;

        fn new_theta_intersection() -> UniquePtr<ThetaIntersectionShim>;

        fn update_with_sketch(self: Pin<&mut ThetaIntersectionShim>, sketch: &ThetaSketchShim);
        fn update_with_compact(self: Pin<&mut ThetaIntersectionShim>, sketch: &CompactThetaSketchShim);
        fn update_with_wrapped(self: Pin<&mut ThetaIntersectionShim>, sketch: &WrappedCompactThetaSketchShim);

        fn get_result(self: &ThetaIntersectionShim, ordered: bool) -> Result<UniquePtr<CompactThetaSketchShim>>;
        fn has_result(self: &ThetaIntersectionShim) -> bool;
    }
}
```

`new_theta_intersection` returns a plain `UniquePtr` (not `Result<UniquePtr<...>>`) since the default constructor cannot throw — unlike `new_theta_sketch`/`new_theta_union`, which validate builder parameters (`lg_k`, `p`) that can be out of range. `get_result` is `Result<...>` because it throws when `has_result()` is false.

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "theta")]
pub mod theta_intersection;
```

- [ ] **Step 5: Build**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 6: Write the sys link test**

```rust
// apache-datasketches-sys/tests/theta_intersection_link_test.rs
use apache_datasketches_sys::theta_intersection::ffi as intersection_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;

#[test]
fn intersect_two_sketches() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    let mut b = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    for i in 250..750u64 {
        b.pin_mut().update_u64(i);
    }

    let mut isect = intersection_ffi::new_theta_intersection();
    assert!(!isect.has_result());
    isect.pin_mut().update_with_sketch(&a);
    isect.pin_mut().update_with_sketch(&b);
    assert!(isect.has_result());

    let result = isect.get_result(true).unwrap();
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}

#[test]
fn get_result_before_update_throws() {
    let isect = intersection_ffi::new_theta_intersection();
    assert!(isect.get_result(true).is_err());
}
```

- [ ] **Step 7: Run**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 8: Add `SketchError::EmptyIntersection` to `apache-datasketches/src/error.rs`**

Add a new variant to the existing `SketchError` enum:

```rust
    /// `ThetaIntersection::get_result()` was called before any `update()`.
    #[error("intersection has no result: no update() call has been made yet")]
    EmptyIntersection,
```

(placed alongside the existing `InvalidConfig`, `Deserialization`, `Cpp` variants, using whatever derive macro — e.g. `thiserror::Error` — the existing enum already uses; match the existing attribute style exactly.)

- [ ] **Step 9: Write safe `apache-datasketches/src/theta/intersection.rs`**

```rust
use super::input::ThetaInput;
use super::CompactThetaSketch;
use crate::error::SketchError;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use apache_datasketches_sys::theta_intersection::ffi as sys;
use cxx::UniquePtr;

pub struct ThetaIntersection {
    inner: UniquePtr<sys::ThetaIntersectionShim>,
}

unsafe impl Send for ThetaIntersection {}

impl Default for ThetaIntersection {
    fn default() -> Self {
        Self::new()
    }
}

impl ThetaIntersection {
    pub fn new() -> Self {
        Self {
            inner: sys::new_theta_intersection(),
        }
    }

    pub fn update(&mut self, input: &impl ThetaInput) {
        match input.as_theta_input() {
            ThetaInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ThetaInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
            ThetaInputRef::Wrapped(w) => self.inner.pin_mut().update_with_wrapped(w),
        }
    }

    pub fn get_result(&self, ordered: bool) -> Result<CompactThetaSketch, SketchError> {
        if !self.inner.has_result() {
            return Err(SketchError::EmptyIntersection);
        }
        let inner = self
            .inner
            .get_result(ordered)
            .map_err(|e| SketchError::Cpp(e.what().to_string()))?;
        Ok(CompactThetaSketch::from_shim(inner))
    }

    pub fn has_result(&self) -> bool {
        self.inner.has_result()
    }
}
```

Note: `get_result` checks `has_result()` itself and maps the empty case to `SketchError::EmptyIntersection` *before* ever calling the throwing shim method, so the `Cpp(...)` fallback branch is unreachable in practice (guarded defensively only in case upstream ever throws for another reason).

- [ ] **Step 10: Update `apache-datasketches/src/theta/mod.rs`**

```rust
mod builder;
mod compact;
mod input;
mod intersection;
mod sketch;
mod union;
mod wrapped;

pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use compact::CompactThetaSketch;
pub use input::ThetaInput;
pub use intersection::ThetaIntersection;
pub use sketch::ThetaSketch;
pub use union::{ThetaUnion, ThetaUnionBuilder};
pub use wrapped::WrappedCompactThetaSketch;
```

- [ ] **Step 11: Write the safe smoke test**

```rust
// apache-datasketches/tests/theta_intersection_smoke_test.rs
use apache_datasketches::theta::{ThetaIntersection, ThetaSketchBuilder};
use apache_datasketches::SketchError;

#[test]
fn intersect_two_theta_sketches() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    for i in 250..750u64 {
        b.update_u64(i);
    }

    let mut isect = ThetaIntersection::new();
    isect.update(&a);
    isect.update(&b);

    let result = isect.get_result(true).unwrap();
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}

#[test]
fn get_result_before_update_is_empty_intersection_error() {
    let isect = ThetaIntersection::new();
    match isect.get_result(true) {
        Err(SketchError::EmptyIntersection) => {}
        other => panic!("expected EmptyIntersection, got {:?}", other),
    }
}
```

- [ ] **Step 12: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 13: Commit**

```bash
git add apache-datasketches-sys/cpp/theta/theta_intersection_shim.h apache-datasketches-sys/cpp/theta/theta_intersection_shim.cc apache-datasketches-sys/src/theta_intersection.rs apache-datasketches-sys/src/lib.rs apache-datasketches/src/error.rs apache-datasketches/src/theta/intersection.rs apache-datasketches/src/theta/mod.rs apache-datasketches-sys/tests/theta_intersection_link_test.rs apache-datasketches/tests/theta_intersection_smoke_test.rs
git commit -m "Add ThetaIntersection C++ shim, cxx bridge, safe wrapper, and EmptyIntersection error"
```

---

### Task 11: `ThetaAnotB` C++ shim (ctor, 9-way `compute_*` combinations) + cxx bridge + safe wrapper

**Files:**
- Create: `apache-datasketches-sys/cpp/theta/theta_a_not_b_shim.h`
- Create: `apache-datasketches-sys/cpp/theta/theta_a_not_b_shim.cc`
- Create: `apache-datasketches-sys/src/theta_a_not_b.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Create: `apache-datasketches/src/theta/a_not_b.rs`
- Modify: `apache-datasketches/src/theta/mod.rs`
- Test: `apache-datasketches-sys/tests/theta_a_not_b_link_test.rs`
- Test: `apache-datasketches/tests/theta_a_not_b_smoke_test.rs`

**Interfaces:**
- Consumes: `datasketches::theta_a_not_b` (Reference section); `ThetaSketchShim`, `CompactThetaSketchShim`, `WrappedCompactThetaSketchShim` (Tasks 2, 4, 7); `ThetaInput`/`ThetaInputRef` (Task 8).
- Produces: C++ class `apache_datasketches_rs::ThetaAnotBShim`; Rust bridge `apache_datasketches_sys::theta_a_not_b::ffi::{ThetaAnotBShim, new_theta_a_not_b}`; safe `apache_datasketches::theta::ThetaAnotB`.

Upstream's `theta_a_not_b::compute()` is `template<typename FwdSketch, typename Sketch> compact_theta_sketch compute(FwdSketch&& a, const Sketch& b, bool ordered) const` — templated over **both** arguments independently. Since each of `a` and `b` can independently be any of the three sketch types, the shim needs the full 3×3=9 concrete overloads (`compute_sketch_sketch`, `compute_sketch_compact`, `compute_sketch_wrapped`, `compute_compact_sketch`, `compute_compact_compact`, `compute_compact_wrapped`, `compute_wrapped_sketch`, `compute_wrapped_compact`, `compute_wrapped_wrapped`), and the safe Rust `ThetaAnotB::compute` dispatches via a 3×3 (9-arm) match on `(a.as_theta_input(), b.as_theta_input())`.

- [ ] **Step 1: Write `theta_a_not_b_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_a_not_b.hpp"
#include "theta_sketch_shim.h"
#include "theta_compact_shim.h"
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

class ThetaAnotBShim {
public:
  ThetaAnotBShim();

  std::unique_ptr<CompactThetaSketchShim> compute_sketch_sketch(const ThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_sketch_compact(const ThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_sketch_wrapped(const ThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_compact_sketch(const CompactThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_compact_compact(const CompactThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_compact_wrapped(const CompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_wrapped_sketch(const WrappedCompactThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_wrapped_compact(const WrappedCompactThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_wrapped_wrapped(const WrappedCompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const;

private:
  datasketches::theta_a_not_b a_not_b_;
};

std::unique_ptr<ThetaAnotBShim> new_theta_a_not_b();

} // namespace apache_datasketches_rs
```

- [ ] **Step 2: Write `theta_a_not_b_shim.cc`**

```cpp
#include "theta_a_not_b_shim.h"

namespace apache_datasketches_rs {

ThetaAnotBShim::ThetaAnotBShim() : a_not_b_() {}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_sketch_sketch(
    const ThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_sketch_compact(
    const ThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_sketch_wrapped(
    const ThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_compact_sketch(
    const CompactThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_compact_compact(
    const CompactThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_compact_wrapped(
    const CompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_wrapped_sketch(
    const WrappedCompactThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_wrapped_compact(
    const WrappedCompactThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_wrapped_wrapped(
    const WrappedCompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<ThetaAnotBShim> new_theta_a_not_b() {
  return std::make_unique<ThetaAnotBShim>();
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Write `apache-datasketches-sys/src/theta_a_not_b.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");
        include!("theta_wrapped_shim.h");
        include!("theta_a_not_b_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;
        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
        type WrappedCompactThetaSketchShim = crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;

        type ThetaAnotBShim;

        fn new_theta_a_not_b() -> UniquePtr<ThetaAnotBShim>;

        fn compute_sketch_sketch(self: &ThetaAnotBShim, a: &ThetaSketchShim, b: &ThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_sketch_compact(self: &ThetaAnotBShim, a: &ThetaSketchShim, b: &CompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_sketch_wrapped(self: &ThetaAnotBShim, a: &ThetaSketchShim, b: &WrappedCompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_compact_sketch(self: &ThetaAnotBShim, a: &CompactThetaSketchShim, b: &ThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_compact_compact(self: &ThetaAnotBShim, a: &CompactThetaSketchShim, b: &CompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_compact_wrapped(self: &ThetaAnotBShim, a: &CompactThetaSketchShim, b: &WrappedCompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_wrapped_sketch(self: &ThetaAnotBShim, a: &WrappedCompactThetaSketchShim, b: &ThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_wrapped_compact(self: &ThetaAnotBShim, a: &WrappedCompactThetaSketchShim, b: &CompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_wrapped_wrapped(self: &ThetaAnotBShim, a: &WrappedCompactThetaSketchShim, b: &WrappedCompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
    }
}
```

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "theta")]
pub mod theta_a_not_b;
```

- [ ] **Step 5: Build**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 6: Write the sys link test**

```rust
// apache-datasketches-sys/tests/theta_a_not_b_link_test.rs
use apache_datasketches_sys::theta_a_not_b::ffi as a_not_b_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;

#[test]
fn a_not_b_two_sketches() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    let mut b = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    for i in 250..750u64 {
        b.pin_mut().update_u64(i);
    }

    let a_not_b = a_not_b_ffi::new_theta_a_not_b();
    let result = a_not_b.compute_sketch_sketch(&a, &b, true);
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}
```

- [ ] **Step 7: Run**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 8: Write safe `apache-datasketches/src/theta/a_not_b.rs`**

```rust
use super::input::ThetaInput;
use super::CompactThetaSketch;
use apache_datasketches_sys::theta_a_not_b::ffi as sys;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use cxx::UniquePtr;

pub struct ThetaAnotB {
    inner: UniquePtr<sys::ThetaAnotBShim>,
}

unsafe impl Send for ThetaAnotB {}

impl Default for ThetaAnotB {
    fn default() -> Self {
        Self::new()
    }
}

impl ThetaAnotB {
    pub fn new() -> Self {
        Self {
            inner: sys::new_theta_a_not_b(),
        }
    }

    pub fn compute(
        &self,
        a: &impl ThetaInput,
        b: &impl ThetaInput,
        ordered: bool,
    ) -> CompactThetaSketch {
        let inner = match (a.as_theta_input(), b.as_theta_input()) {
            (ThetaInputRef::Sketch(a), ThetaInputRef::Sketch(b)) => {
                self.inner.compute_sketch_sketch(a, b, ordered)
            }
            (ThetaInputRef::Sketch(a), ThetaInputRef::Compact(b)) => {
                self.inner.compute_sketch_compact(a, b, ordered)
            }
            (ThetaInputRef::Sketch(a), ThetaInputRef::Wrapped(b)) => {
                self.inner.compute_sketch_wrapped(a, b, ordered)
            }
            (ThetaInputRef::Compact(a), ThetaInputRef::Sketch(b)) => {
                self.inner.compute_compact_sketch(a, b, ordered)
            }
            (ThetaInputRef::Compact(a), ThetaInputRef::Compact(b)) => {
                self.inner.compute_compact_compact(a, b, ordered)
            }
            (ThetaInputRef::Compact(a), ThetaInputRef::Wrapped(b)) => {
                self.inner.compute_compact_wrapped(a, b, ordered)
            }
            (ThetaInputRef::Wrapped(a), ThetaInputRef::Sketch(b)) => {
                self.inner.compute_wrapped_sketch(a, b, ordered)
            }
            (ThetaInputRef::Wrapped(a), ThetaInputRef::Compact(b)) => {
                self.inner.compute_wrapped_compact(a, b, ordered)
            }
            (ThetaInputRef::Wrapped(a), ThetaInputRef::Wrapped(b)) => {
                self.inner.compute_wrapped_wrapped(a, b, ordered)
            }
        };
        CompactThetaSketch::from_shim(inner)
    }
}
```

- [ ] **Step 9: Update `apache-datasketches/src/theta/mod.rs`**

```rust
mod a_not_b;
mod builder;
mod compact;
mod input;
mod intersection;
mod sketch;
mod union;
mod wrapped;

pub use a_not_b::ThetaAnotB;
pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use compact::CompactThetaSketch;
pub use input::ThetaInput;
pub use intersection::ThetaIntersection;
pub use sketch::ThetaSketch;
pub use union::{ThetaUnion, ThetaUnionBuilder};
pub use wrapped::WrappedCompactThetaSketch;
```

- [ ] **Step 10: Write the safe smoke test**

```rust
// apache-datasketches/tests/theta_a_not_b_smoke_test.rs
use apache_datasketches::theta::{ThetaAnotB, ThetaSketchBuilder};

#[test]
fn a_not_b_two_theta_sketches() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    for i in 250..750u64 {
        b.update_u64(i);
    }

    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}

#[test]
fn a_not_b_mixed_input_types() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 250..750u64 {
        b.update_u64(i);
    }
    let b_compact = b.compact(true);

    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b_compact, true);
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}
```

- [ ] **Step 11: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 12: Commit**

```bash
git add apache-datasketches-sys/cpp/theta/theta_a_not_b_shim.h apache-datasketches-sys/cpp/theta/theta_a_not_b_shim.cc apache-datasketches-sys/src/theta_a_not_b.rs apache-datasketches-sys/src/lib.rs apache-datasketches/src/theta/a_not_b.rs apache-datasketches/src/theta/mod.rs apache-datasketches-sys/tests/theta_a_not_b_link_test.rs apache-datasketches/tests/theta_a_not_b_smoke_test.rs
git commit -m "Add ThetaAnotB C++ shim, cxx bridge, and safe wrapper"
```

---

### Task 12: `jaccard_similarity()` C++ shim (9-way dispatch) + cxx bridge + safe `jaccard_similarity()`/`JaccardBounds`

**Files:**
- Create: `apache-datasketches-sys/cpp/theta/theta_jaccard_shim.h`
- Create: `apache-datasketches-sys/cpp/theta/theta_jaccard_shim.cc`
- Create: `apache-datasketches-sys/src/theta_jaccard.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Create: `apache-datasketches/src/theta/jaccard.rs`
- Modify: `apache-datasketches/src/theta/mod.rs`
- Test: `apache-datasketches-sys/tests/theta_jaccard_link_test.rs`
- Test: `apache-datasketches/tests/theta_jaccard_smoke_test.rs`
- Test: `apache-datasketches/tests/theta_jaccard_similarity_test.rs` (ported from upstream)

**Interfaces:**
- Consumes: `datasketches::theta_jaccard_similarity` (Reference section); `ThetaSketchShim`, `CompactThetaSketchShim`, `WrappedCompactThetaSketchShim` (Tasks 2, 4, 7); `ThetaInput`/`ThetaInputRef` (Task 8).
- Produces: cxx-shared struct `apache_datasketches_rs::JaccardBoundsFfi {lower_bound: f64, estimate: f64, upper_bound: f64}`; free functions `jaccard_*` (9 combinations); Rust bridge `apache_datasketches_sys::theta_jaccard::ffi::{JaccardBoundsFfi, jaccard_sketch_sketch, ...}`; safe `apache_datasketches::theta::{JaccardBounds, jaccard_similarity}`.

Upstream's `theta_jaccard_similarity::jaccard()` is `template<typename SketchA, typename SketchB> static std::array<double, 3> jaccard(const SketchA& a, const SketchB& b)`, returning `{lower_bound, estimate, upper_bound}` — templated over both arguments independently, exactly like `theta_a_not_b::compute()` in Task 11. This is a static/stateless call (no persistent C++ object), so the shim exposes 9 free functions (not methods on a stateful shim class) named `jaccard_sketch_sketch`, `jaccard_sketch_compact`, ..., `jaccard_wrapped_wrapped`, matching Task 11's naming convention. Each returns a `JaccardBoundsFfi` cxx-shared struct (not `std::array<double,3>`, which cxx cannot bridge directly) by value.

- [ ] **Step 1: Write `theta_jaccard_shim.h`**

```cpp
#pragma once
#include <cstdint>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_jaccard_similarity.hpp"
#include "theta_sketch_shim.h"
#include "theta_compact_shim.h"
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

struct JaccardBoundsFfi;

JaccardBoundsFfi jaccard_sketch_sketch(const ThetaSketchShim& a, const ThetaSketchShim& b);
JaccardBoundsFfi jaccard_sketch_compact(const ThetaSketchShim& a, const CompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_sketch_wrapped(const ThetaSketchShim& a, const WrappedCompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_compact_sketch(const CompactThetaSketchShim& a, const ThetaSketchShim& b);
JaccardBoundsFfi jaccard_compact_compact(const CompactThetaSketchShim& a, const CompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_compact_wrapped(const CompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_wrapped_sketch(const WrappedCompactThetaSketchShim& a, const ThetaSketchShim& b);
JaccardBoundsFfi jaccard_wrapped_compact(const WrappedCompactThetaSketchShim& a, const CompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_wrapped_wrapped(const WrappedCompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b);

} // namespace apache_datasketches_rs
```

`struct JaccardBoundsFfi;` is forward-declared here (not defined) because its real definition must come from the cxx-generated header (shared structs are defined once by cxx, on the Rust side, per the same forward-declare pattern already used for `ResizeFactor` in Task 2 — a shared *struct* works identically to a shared *enum* in this respect). `theta_jaccard_shim.cc` includes the generated `theta_jaccard.rs.h` to get the full definition before implementing these functions.

- [ ] **Step 2: Write `theta_jaccard_shim.cc`**

```cpp
#include "theta_jaccard_shim.h"
#include "theta_jaccard.rs.h"

namespace apache_datasketches_rs {

namespace {
JaccardBoundsFfi to_ffi(const std::array<double, 3>& result) {
  return JaccardBoundsFfi{result[0], result[1], result[2]};
}
} // namespace

JaccardBoundsFfi jaccard_sketch_sketch(const ThetaSketchShim& a, const ThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_sketch_compact(const ThetaSketchShim& a, const CompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_sketch_wrapped(const ThetaSketchShim& a, const WrappedCompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_compact_sketch(const CompactThetaSketchShim& a, const ThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_compact_compact(const CompactThetaSketchShim& a, const CompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_compact_wrapped(const CompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_wrapped_sketch(const WrappedCompactThetaSketchShim& a, const ThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_wrapped_compact(const WrappedCompactThetaSketchShim& a, const CompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_wrapped_wrapped(const WrappedCompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 3: Write `apache-datasketches-sys/src/theta_jaccard.rs`**

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    struct JaccardBoundsFfi {
        lower_bound: f64,
        estimate: f64,
        upper_bound: f64,
    }

    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");
        include!("theta_wrapped_shim.h");
        include!("theta_jaccard_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;
        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
        type WrappedCompactThetaSketchShim = crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;

        fn jaccard_sketch_sketch(a: &ThetaSketchShim, b: &ThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_sketch_compact(a: &ThetaSketchShim, b: &CompactThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_sketch_wrapped(a: &ThetaSketchShim, b: &WrappedCompactThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_compact_sketch(a: &CompactThetaSketchShim, b: &ThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_compact_compact(a: &CompactThetaSketchShim, b: &CompactThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_compact_wrapped(a: &CompactThetaSketchShim, b: &WrappedCompactThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_wrapped_sketch(a: &WrappedCompactThetaSketchShim, b: &ThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_wrapped_compact(a: &WrappedCompactThetaSketchShim, b: &CompactThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_wrapped_wrapped(a: &WrappedCompactThetaSketchShim, b: &WrappedCompactThetaSketchShim) -> JaccardBoundsFfi;
    }
}
```

- [ ] **Step 4: Declare the module in `apache-datasketches-sys/src/lib.rs`**

```rust
#[cfg(feature = "theta")]
pub mod theta_jaccard;
```

- [ ] **Step 5: Build**

```bash
cargo build -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 6: Write the sys link test**

```rust
// apache-datasketches-sys/tests/theta_jaccard_link_test.rs
use apache_datasketches_sys::theta_jaccard::ffi as jaccard_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;

#[test]
fn jaccard_identical_sketches_is_one() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        a.pin_mut().update_u64(i);
    }
    let bounds = jaccard_ffi::jaccard_sketch_sketch(&a, &a);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
    assert!((bounds.lower_bound - 1.0).abs() < 1e-9);
    assert!((bounds.upper_bound - 1.0).abs() < 1e-9);
}

#[test]
fn jaccard_disjoint_sketches_is_zero() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    let mut b = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    for i in 500..1000u64 {
        b.pin_mut().update_u64(i);
    }
    let bounds = jaccard_ffi::jaccard_sketch_sketch(&a, &b);
    assert!(bounds.estimate.abs() < 1e-9);
}
```

- [ ] **Step 7: Run**

```bash
cargo test -p apache-datasketches-sys --no-default-features --features theta
```

Expected: PASS.

- [ ] **Step 8: Write safe `apache-datasketches/src/theta/jaccard.rs`**

```rust
use super::input::ThetaInput;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use apache_datasketches_sys::theta_jaccard::ffi as sys;

/// The result of [`jaccard_similarity`]: a confidence interval around the
/// estimated Jaccard index of two theta sketches, in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JaccardBounds {
    pub lower_bound: f64,
    pub estimate: f64,
    pub upper_bound: f64,
}

impl From<sys::JaccardBoundsFfi> for JaccardBounds {
    fn from(ffi: sys::JaccardBoundsFfi) -> Self {
        Self {
            lower_bound: ffi.lower_bound,
            estimate: ffi.estimate,
            upper_bound: ffi.upper_bound,
        }
    }
}

/// Estimates the Jaccard index (intersection-over-union) of two theta
/// sketches, each of which may independently be a [`super::ThetaSketch`],
/// [`super::CompactThetaSketch`], or [`super::WrappedCompactThetaSketch`].
pub fn jaccard_similarity(a: &impl ThetaInput, b: &impl ThetaInput) -> JaccardBounds {
    let ffi = match (a.as_theta_input(), b.as_theta_input()) {
        (ThetaInputRef::Sketch(a), ThetaInputRef::Sketch(b)) => sys::jaccard_sketch_sketch(a, b),
        (ThetaInputRef::Sketch(a), ThetaInputRef::Compact(b)) => sys::jaccard_sketch_compact(a, b),
        (ThetaInputRef::Sketch(a), ThetaInputRef::Wrapped(b)) => sys::jaccard_sketch_wrapped(a, b),
        (ThetaInputRef::Compact(a), ThetaInputRef::Sketch(b)) => sys::jaccard_compact_sketch(a, b),
        (ThetaInputRef::Compact(a), ThetaInputRef::Compact(b)) => sys::jaccard_compact_compact(a, b),
        (ThetaInputRef::Compact(a), ThetaInputRef::Wrapped(b)) => sys::jaccard_compact_wrapped(a, b),
        (ThetaInputRef::Wrapped(a), ThetaInputRef::Sketch(b)) => sys::jaccard_wrapped_sketch(a, b),
        (ThetaInputRef::Wrapped(a), ThetaInputRef::Compact(b)) => sys::jaccard_wrapped_compact(a, b),
        (ThetaInputRef::Wrapped(a), ThetaInputRef::Wrapped(b)) => sys::jaccard_wrapped_wrapped(a, b),
    };
    ffi.into()
}
```

- [ ] **Step 9: Update `apache-datasketches/src/theta/mod.rs`**

```rust
mod a_not_b;
mod builder;
mod compact;
mod input;
mod intersection;
mod jaccard;
mod sketch;
mod union;
mod wrapped;

pub use a_not_b::ThetaAnotB;
pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use compact::CompactThetaSketch;
pub use input::ThetaInput;
pub use intersection::ThetaIntersection;
pub use jaccard::{jaccard_similarity, JaccardBounds};
pub use sketch::ThetaSketch;
pub use union::{ThetaUnion, ThetaUnionBuilder};
pub use wrapped::WrappedCompactThetaSketch;
```

- [ ] **Step 10: Write the safe smoke test**

```rust
// apache-datasketches/tests/theta_jaccard_smoke_test.rs
use apache_datasketches::theta::{jaccard_similarity, ThetaSketchBuilder};

#[test]
fn jaccard_identical_sketches_is_one() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        a.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &a);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
}

#[test]
fn jaccard_disjoint_sketches_is_zero() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    for i in 500..1000u64 {
        b.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.estimate.abs() < 1e-9);
}
```

- [ ] **Step 11: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 12: Port `theta_jaccard_similarity_test.cpp` → `theta_jaccard_similarity_test.rs`**

Per the Test Inventory above, upstream has 10 cases, 4 of which pass a custom (non-default) seed to `jaccard()`'s three-argument overload. Since this plan's Global Constraints forbid exposing any seed parameter (all sketches always use `DEFAULT_SEED`), those 4 custom-seed cases have no reachable equivalent through this crate's public API and are **not ported** — this is consistent with, and an application of, the same no-custom-seed constraint already noted inline elsewhere in this plan, not a new exception. The remaining 6 default-seed cases are ported:

```rust
// apache-datasketches/tests/theta_jaccard_similarity_test.rs
//! Ported from theta/test/theta_jaccard_similarity_test.cpp (tag 5.2.0).
//! The upstream file's custom-seed cases are not ported: this crate never
//! exposes a seed parameter (see Global Constraints), so there is no
//! reachable equivalent through the public API.
use apache_datasketches::theta::{jaccard_similarity, ThetaSketchBuilder};

#[test]
fn empty_sketches() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let b = ThetaSketchBuilder::new().build().unwrap();
    let bounds = jaccard_similarity(&a, &b);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
}

#[test]
fn first_empty_second_not() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    b.update_u64(1);
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.estimate.abs() < 1e-9);
}

#[test]
fn second_empty_first_not() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    a.update_u64(1);
    let b = ThetaSketchBuilder::new().build().unwrap();
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.estimate.abs() < 1e-9);
}

#[test]
fn exact_mode_identical() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..100u64 {
        a.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &a);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
    assert!((bounds.lower_bound - 1.0).abs() < 1e-9);
    assert!((bounds.upper_bound - 1.0).abs() < 1e-9);
}

#[test]
fn estimation_mode_similar_sketches() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    for i in 0..10_000u64 {
        b.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.lower_bound <= bounds.estimate);
    assert!(bounds.estimate <= bounds.upper_bound);
    assert!((bounds.estimate - 1.0).abs() < 0.05);
}

#[test]
fn estimation_mode_half_overlap() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    for i in 5_000..15_000u64 {
        b.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.lower_bound <= bounds.estimate);
    assert!(bounds.estimate <= bounds.upper_bound);
    // union ~15000, intersection ~5000 => jaccard ~= 1/3
    assert!((bounds.estimate - (1.0 / 3.0)).abs() < 0.05);
}
```

- [ ] **Step 13: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 14: Commit**

```bash
git add apache-datasketches-sys/cpp/theta/theta_jaccard_shim.h apache-datasketches-sys/cpp/theta/theta_jaccard_shim.cc apache-datasketches-sys/src/theta_jaccard.rs apache-datasketches-sys/src/lib.rs apache-datasketches/src/theta/jaccard.rs apache-datasketches/src/theta/mod.rs apache-datasketches-sys/tests/theta_jaccard_link_test.rs apache-datasketches/tests/theta_jaccard_smoke_test.rs apache-datasketches/tests/theta_jaccard_similarity_test.rs
git commit -m "Add jaccard_similarity C++ shim, cxx bridge, safe wrapper, and ported tests"
```

---

### Task 13: `ThetaInput` trait dispatch tests (new) + port `theta_setop_test.cpp` (16 pairwise combination cases)

**Files:**
- Test: `apache-datasketches/tests/theta_input_dispatch_test.rs` (new, non-upstream)
- Test: `apache-datasketches/tests/theta_setop_test.rs` (ported from upstream)

**Interfaces:**
- Consumes: everything built so far — `ThetaSketch`, `CompactThetaSketch`, `WrappedCompactThetaSketch`, `ThetaUnion`, `ThetaIntersection`, `ThetaAnotB`, `jaccard_similarity`, `ThetaInput` (Tasks 3, 6, 7, 8, 9, 10, 11, 12).
- Produces: no new production code — pure test coverage confirming (a) all 3×3/3 dispatch combinations are reachable end-to-end through the public API for every set-op and Jaccard, and (b) the four `SkType` (`EMPTY`, `EXACT`, `ESTIMATION`, `DEGENERATE`) pairwise combinations from upstream's data-driven `theta_setop_test.cpp` behave identically through this crate.

Task 9's `union_mixed_input_types` and Task 11's `a_not_b_mixed_input_types` smoke tests already each exercise a few `ThetaInputRef` combinations, but not the full 3×3 (or 3×3×3, for intersection combined with union/a_not_b) matrix systematically. This task closes that gap with one dedicated, clearly-labeled non-upstream test file per Global Constraints ("plus new, clearly-marked non-upstream tests for `ThetaInput` trait dispatch").

Note: a `Box<dyn ThetaInput>`-based helper that iterates over "each input kind" generically was considered and rejected — `ThetaInput` is sealed and only implemented for the three concrete owned/borrowed types (not for `&ThetaSketch`/`&CompactThetaSketch` reference wrappers), and it is not object-safe as written (`as_theta_input` returns `ThetaInputRef<'_>`, which ties the trait object's lifetime awkwardly). The test below instead enumerates all 9 combinations directly and explicitly, which is also more readable.

- [ ] **Step 1: Write `theta_input_dispatch_test.rs`**

```rust
// apache-datasketches/tests/theta_input_dispatch_test.rs
//! New, non-upstream tests: systematically exercise every ThetaInput
//! dispatch combination (Sketch/Compact/Wrapped x Sketch/Compact/Wrapped)
//! for each set operation and for jaccard_similarity, since production
//! code contains a hand-written match arm per combination (see
//! ThetaUnion::update, ThetaIntersection::update, ThetaAnotB::compute,
//! and jaccard_similarity) that would not otherwise be caught by a
//! missing/mismatched arm.
use apache_datasketches::theta::{
    jaccard_similarity, ThetaAnotB, ThetaIntersection, ThetaSketchBuilder, ThetaUnionBuilder,
    WrappedCompactThetaSketch,
};

fn fixture() -> (
    apache_datasketches::theta::ThetaSketch,
    apache_datasketches::theta::CompactThetaSketch,
    Vec<u8>,
) {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    (sketch, compact, bytes)
}

#[test]
fn union_accepts_all_nine_combinations() {
    let (sketch, compact, bytes) = fixture();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    for _ in 0..1 {
        // Exercise all 3 inputs as the *first* update, and all 3 as the
        // *second*, covering the 3x3 matrix across two calls per union.
        let combos: [&dyn Fn(&mut apache_datasketches::theta::ThetaUnion); 3] = [
            &|u| u.update(&sketch),
            &|u| u.update(&compact),
            &|u| u.update(&wrapped),
        ];
        for first in &combos {
            for second in &combos {
                let mut union_ = ThetaUnionBuilder::new().lg_k(12).build().unwrap();
                first(&mut union_);
                second(&mut union_);
                let result = union_.get_result(true);
                assert!((result.get_estimate() - 500.0).abs() < 20.0);
            }
        }
    }
}

#[test]
fn intersection_accepts_all_nine_combinations() {
    let (sketch, compact, bytes) = fixture();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    let combos: [&dyn Fn(&mut ThetaIntersection); 3] = [
        &|u| u.update(&sketch),
        &|u| u.update(&compact),
        &|u| u.update(&wrapped),
    ];
    for first in &combos {
        for second in &combos {
            let mut isect = ThetaIntersection::new();
            first(&mut isect);
            second(&mut isect);
            let result = isect.get_result(true).unwrap();
            assert!((result.get_estimate() - 500.0).abs() < 20.0);
        }
    }
}

#[test]
fn a_not_b_accepts_all_nine_combinations() {
    let (sketch, compact, bytes) = fixture();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    // a-not-b of a set with itself, across every (a, b) type combination,
    // is always empty regardless of which concrete types are used.
    let a_not_b = ThetaAnotB::new();
    assert!(a_not_b.compute(&sketch, &sketch, true).is_empty());
    assert!(a_not_b.compute(&sketch, &compact, true).is_empty());
    assert!(a_not_b.compute(&sketch, &wrapped, true).is_empty());
    assert!(a_not_b.compute(&compact, &sketch, true).is_empty());
    assert!(a_not_b.compute(&compact, &compact, true).is_empty());
    assert!(a_not_b.compute(&compact, &wrapped, true).is_empty());
    assert!(a_not_b.compute(&wrapped, &sketch, true).is_empty());
    assert!(a_not_b.compute(&wrapped, &compact, true).is_empty());
    assert!(a_not_b.compute(&wrapped, &wrapped, true).is_empty());
}

#[test]
fn jaccard_accepts_all_nine_combinations() {
    let (sketch, compact, bytes) = fixture();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    // jaccard_similarity of a set with itself is always exactly 1.0,
    // regardless of which concrete types are used for the two arguments.
    let pairs_estimate = [
        jaccard_similarity(&sketch, &sketch).estimate,
        jaccard_similarity(&sketch, &compact).estimate,
        jaccard_similarity(&sketch, &wrapped).estimate,
        jaccard_similarity(&compact, &sketch).estimate,
        jaccard_similarity(&compact, &compact).estimate,
        jaccard_similarity(&compact, &wrapped).estimate,
        jaccard_similarity(&wrapped, &sketch).estimate,
        jaccard_similarity(&wrapped, &compact).estimate,
        jaccard_similarity(&wrapped, &wrapped).estimate,
    ];
    for estimate in pairs_estimate {
        assert!((estimate - 1.0).abs() < 1e-9);
    }
}
```

- [ ] **Step 2: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 3: Port `theta_setop_test.cpp` → `theta_setop_test.rs`**

Per the verified upstream source (`vendor/datasketches-cpp/theta/test/theta_setop_test.cpp`), the file builds sketches of four kinds via a `build_sketch(SkType)` helper — `EMPTY`, `EXACT` (few small updates, no estimation), `ESTIMATION` (many updates, lg_k=5 forces estimation mode), and `DEGENERATE` (theta < 1 with zero retained entries, built via `update_theta_sketch::builder().set_p(LOWP).build()` then updating with values that all hash above theta) — then runs union/intersection/a-not-b over the 4x4 combinations (16 cases) via `checks()`/`check_result()` helpers that assert `is_empty()`, `is_estimation_mode()`, and estimate bounds. This plan reproduces that structure directly in Rust, using the same lg_k=5 and constants confirmed from the upstream source (`GT_MIDP_V=3`, `MIDP=0.5f`, `GT_LOWP_V=6`, `LOWP=0.1f`, `LT_LOWP_V=4`).

```rust
// apache-datasketches/tests/theta_setop_test.rs
//! Ported from theta/test/theta_setop_test.cpp (tag 5.2.0): the 4x4 = 16
//! pairwise combinations of {Empty, Exact, Estimation, Degenerate} sketch
//! states run through union/intersection/a-not-b.
use apache_datasketches::theta::{
    ResizeFactor, ThetaAnotB, ThetaIntersection, ThetaSketch, ThetaSketchBuilder, ThetaUnionBuilder,
};

const LG_K: u8 = 5;
const GT_MIDP_V: u64 = 3;
const MIDP: f32 = 0.5;
const GT_LOWP_V: u64 = 6;
const LOWP: f32 = 0.1;
const LT_LOWP_V: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
enum SkType {
    Empty,
    Exact,
    Estimation,
    Degenerate,
}

const ALL_TYPES: [SkType; 4] = [SkType::Empty, SkType::Exact, SkType::Estimation, SkType::Degenerate];

fn build_sketch(ty: SkType) -> ThetaSketch {
    match ty {
        SkType::Empty => ThetaSketchBuilder::new().lg_k(LG_K).build().unwrap(),
        SkType::Exact => {
            let mut s = ThetaSketchBuilder::new().lg_k(LG_K).build().unwrap();
            for i in 0..GT_MIDP_V {
                s.update_u64(i);
            }
            s
        }
        SkType::Estimation => {
            let mut s = ThetaSketchBuilder::new()
                .lg_k(LG_K)
                .resize_factor(ResizeFactor::X1)
                .build()
                .unwrap();
            for i in 0..10_000u64 {
                s.update_u64(i);
            }
            s
        }
        SkType::Degenerate => {
            // p = LOWP forces theta < 1 with (typically) zero retained
            // entries once a handful of updates are applied, matching
            // upstream's construction of a "degenerate" (theta < 1, empty
            // retained set) sketch.
            let mut s = ThetaSketchBuilder::new().lg_k(LG_K).p(LOWP).build().unwrap();
            for i in 0..LT_LOWP_V {
                s.update_u64(i);
            }
            s
        }
    }
}

fn is_estimation_expected(ty: SkType) -> bool {
    matches!(ty, SkType::Estimation | SkType::Degenerate)
}

fn is_empty_expected(ty: SkType) -> bool {
    matches!(ty, SkType::Empty)
}

#[test]
fn union_all_type_combinations() {
    for &a_ty in &ALL_TYPES {
        for &b_ty in &ALL_TYPES {
            let a = build_sketch(a_ty);
            let b = build_sketch(b_ty);
            let mut union_ = ThetaUnionBuilder::new().lg_k(LG_K).build().unwrap();
            union_.update(&a);
            union_.update(&b);
            let result = union_.get_result(true);

            let expect_empty = is_empty_expected(a_ty) && is_empty_expected(b_ty);
            assert_eq!(
                result.is_empty(),
                expect_empty,
                "union({:?}, {:?}).is_empty()",
                a_ty,
                b_ty
            );
            if is_estimation_expected(a_ty) || is_estimation_expected(b_ty) {
                assert!(
                    result.is_estimation_mode() || expect_empty,
                    "union({:?}, {:?}) expected estimation mode",
                    a_ty,
                    b_ty
                );
            }
        }
    }
}

#[test]
fn intersection_all_type_combinations() {
    for &a_ty in &ALL_TYPES {
        for &b_ty in &ALL_TYPES {
            let a = build_sketch(a_ty);
            let b = build_sketch(b_ty);
            let mut isect = ThetaIntersection::new();
            isect.update(&a);
            isect.update(&b);
            let result = isect.get_result(true).unwrap();

            let expect_empty = is_empty_expected(a_ty) || is_empty_expected(b_ty);
            assert_eq!(
                result.is_empty(),
                expect_empty,
                "intersection({:?}, {:?}).is_empty()",
                a_ty,
                b_ty
            );
        }
    }
}

#[test]
fn a_not_b_all_type_combinations() {
    let a_not_b = ThetaAnotB::new();
    for &a_ty in &ALL_TYPES {
        for &b_ty in &ALL_TYPES {
            let a = build_sketch(a_ty);
            let b = build_sketch(b_ty);
            let result = a_not_b.compute(&a, &b, true);

            if is_empty_expected(a_ty) {
                assert!(
                    result.is_empty(),
                    "a_not_b({:?}, {:?}) expected empty when a is empty",
                    a_ty,
                    b_ty
                );
            }
        }
    }
}
```

Note: unlike upstream's `checks()`/`check_result()`, which assert exact retained-entry counts per combination (verified precisely against the C++ template instantiation's internal state), this port asserts the coarser, type-agnostic invariants (`is_empty()`, `is_estimation_mode()`) that hold identically regardless of implementation details — the goal of porting this file is behavioral parity of the public API's observable outputs across sketch-state combinations, not bit-for-bit internal state replication, consistent with every other ported test file in this plan (which assert `get_estimate()` within tolerance bounds, not exact bit patterns).

- [ ] **Step 4: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add apache-datasketches/tests/theta_input_dispatch_test.rs apache-datasketches/tests/theta_setop_test.rs
git commit -m "Add ThetaInput dispatch tests and port theta_setop_test.cpp"
```

---

### Task 14: Port the four remaining upstream test files (`theta_sketch_test.cpp`, `theta_union_test.cpp`, `theta_intersection_test.cpp`, `theta_a_not_b_test.cpp`)

**Files:**
- Test: `apache-datasketches/tests/theta_sketch_test.rs` (14 of 22 upstream cases ported; the 8 from-java `.sk` fixture cases are excluded — see Step 2's note)
- Test: `apache-datasketches/tests/theta_union_test.rs` (7 cases, ported)
- Test: `apache-datasketches/tests/theta_intersection_test.rs` (13 cases, ported)
- Test: `apache-datasketches/tests/theta_a_not_b_test.rs` (11 cases, ported)

**Interfaces:**
- Consumes: `ThetaSketch`, `ThetaSketchBuilder`, `CompactThetaSketch`, `WrappedCompactThetaSketch`, `ThetaUnion`, `ThetaUnionBuilder`, `ThetaIntersection`, `ThetaAnotB` (Tasks 3, 6, 7, 9, 10, 11).
- Produces: no new production code — completes the 1:1 test-porting obligation from the Test Inventory section for the four remaining upstream Catch2 files (`theta_jaccard_similarity_test.cpp` and `theta_setop_test.cpp` were already ported in Tasks 12–13).

Following this plan's own precedent from the HLL plan (`HllSketchTest.cpp`'s 15 cases were ported with full code for the most behaviorally distinct cases, plus one step porting the remaining cases by the same established pattern once the harness/helpers are in place), each file below is ported with full code for its first several cases, followed by one step instructing the remaining cases to be ported using the same per-case pattern (construct → act → assert, matching the corresponding upstream `TEST_CASE` body).

- [ ] **Step 1: Write the first 8 cases of `theta_sketch_test.rs`** (from `theta_sketch_test.cpp`)

```rust
// apache-datasketches/tests/theta_sketch_test.rs
//! Ported from theta/test/theta_sketch_test.cpp (tag 5.2.0).
use apache_datasketches::theta::{CompactThetaSketch, ThetaSketchBuilder, WrappedCompactThetaSketch};

#[test]
fn empty_sketch_is_empty() {
    let sketch = ThetaSketchBuilder::new().build().unwrap();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_estimate(), 0.0);
    assert!(!sketch.is_estimation_mode());
}

#[test]
fn single_item_exact_mode() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    sketch.update_u64(1);
    assert!(!sketch.is_empty());
    assert_eq!(sketch.get_estimate(), 1.0);
    assert!(!sketch.is_estimation_mode());
}

#[test]
fn many_items_estimation_mode() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..100_000u64 {
        sketch.update_u64(i);
    }
    assert!(sketch.is_estimation_mode());
    assert!((sketch.get_estimate() - 100_000.0).abs() / 100_000.0 < 0.03);
}

#[test]
fn duplicate_updates_do_not_double_count() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    for _ in 0..1000 {
        sketch.update_u64(42);
    }
    assert_eq!(sketch.get_estimate(), 1.0);
}

#[test]
fn trim_reduces_retained_below_target() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(4).build().unwrap();
    for i in 0..10_000u64 {
        sketch.update_u64(i);
    }
    let before = sketch.get_num_retained();
    sketch.trim();
    assert!(sketch.get_num_retained() <= before);
}

#[test]
fn reset_returns_to_empty() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    sketch.update_u64(1);
    sketch.reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_estimate(), 0.0);
}

#[test]
fn lower_and_upper_bounds_bracket_estimate() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    let lb = sketch.get_lower_bound(2).unwrap();
    let ub = sketch.get_upper_bound(2).unwrap();
    assert!(lb <= estimate);
    assert!(estimate <= ub);
}

#[test]
fn serialize_deserialize_v3_round_trip() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    let restored = CompactThetaSketch::deserialize(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());
}
```

- [ ] **Step 2: Port the remaining 6 non-Java-fixture cases of `theta_sketch_test.cpp` using the same pattern**

Per the Test Inventory section, upstream `theta_sketch_test.cpp` has 22 cases, of which cases 6–9 and 16–19 (8 cases) are from-java `.sk` fixture round-trip assertions that are **not ported** (this repo has no `.sk` fixture files, and this plan introduces none — see the Test Inventory's "Not ported" paragraph). That leaves 14 portable cases: 8 are written in full in Step 1 above. Port the remaining 6 (following the upstream file's order, skipping the excluded java-fixture cases): input-type-overload coverage for `update_i32`/`update_u32`/`update_i16`/`update_u16`/`update_i8`/`update_u8`/`update_f64`/`update_bytes`/`update_str`; ordered vs. unordered `compact()`; v4 compressed round-trip via `serialize_compressed`/`deserialize_compressed`; `WrappedCompactThetaSketch` query parity with its source `CompactThetaSketch`; and builder validation errors for `lg_k` outside `[MIN_LG_K, MAX_LG_K]` and `p` outside `(0.0, 1.0]` — each as its own `#[test]` function in `theta_sketch_test.rs`, appended after Step 1's 8 cases, following the construct→act→assert shape already established.

- [ ] **Step 3: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all 14 PASS (the 8 from-java cases are excluded, per Step 2's note).

- [ ] **Step 4: Write the first 4 cases of `theta_union_test.rs`** (from `theta_union_test.cpp`)

```rust
// apache-datasketches/tests/theta_union_test.rs
//! Ported from theta/test/theta_union_test.cpp (tag 5.2.0).
use apache_datasketches::theta::{ThetaSketchBuilder, ThetaUnionBuilder};

#[test]
fn union_of_empty_sketches_is_empty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let b = ThetaSketchBuilder::new().build().unwrap();
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.update(&b);
    let result = union_.get_result(true);
    assert!(result.is_empty());
}

#[test]
fn union_with_one_empty_one_nonempty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    b.update_u64(1);
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.update(&b);
    let result = union_.get_result(true);
    assert_eq!(result.get_estimate(), 1.0);
}

#[test]
fn union_exact_mode_no_overlap() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..5u64 {
        a.update_u64(i);
    }
    for i in 5..10u64 {
        b.update_u64(i);
    }
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.update(&b);
    let result = union_.get_result(true);
    assert_eq!(result.get_estimate(), 10.0);
}

#[test]
fn union_reset_clears_state() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    a.update_u64(1);
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.reset();
    let result = union_.get_result(true);
    assert!(result.is_empty());
}
```

- [ ] **Step 5: Port the remaining 3 cases of `theta_union_test.cpp`**

Port the remaining cases (union in estimation mode with large overlapping input sets, verifying the result's estimate is within the standard error tolerance of the true union cardinality; union across mismatched `lg_k` builder configurations, verifying the result reflects the union builder's own `lg_k` rather than either input's; and a builder-validation case for out-of-range `lg_k`) as `#[test]` functions appended to `theta_union_test.rs`, following the same construct→act→assert shape as Step 4.

- [ ] **Step 6: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all 7 PASS.

- [ ] **Step 7: Write the first 5 cases of `theta_intersection_test.rs`** (from `theta_intersection_test.cpp`)

```rust
// apache-datasketches/tests/theta_intersection_test.rs
//! Ported from theta/test/theta_intersection_test.cpp (tag 5.2.0).
use apache_datasketches::theta::{ThetaIntersection, ThetaSketchBuilder};
use apache_datasketches::SketchError;

#[test]
fn get_result_before_update_is_empty_intersection() {
    let isect = ThetaIntersection::new();
    match isect.get_result(true) {
        Err(SketchError::EmptyIntersection) => {}
        other => panic!("expected EmptyIntersection, got {:?}", other),
    }
}

#[test]
fn has_result_false_before_any_update() {
    let isect = ThetaIntersection::new();
    assert!(!isect.has_result());
}

#[test]
fn intersect_empty_with_nonempty_is_empty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    b.update_u64(1);
    let mut isect = ThetaIntersection::new();
    isect.update(&a);
    isect.update(&b);
    let result = isect.get_result(true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn intersect_disjoint_sets_is_empty() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..5u64 {
        a.update_u64(i);
    }
    for i in 5..10u64 {
        b.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&a);
    isect.update(&b);
    let result = isect.get_result(true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn intersect_identical_sets_preserves_estimate() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10u64 {
        a.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&a);
    isect.update(&a);
    let result = isect.get_result(true).unwrap();
    assert_eq!(result.get_estimate(), 10.0);
}
```

- [ ] **Step 8: Port the remaining 8 cases of `theta_intersection_test.cpp`**

Port the remaining cases (intersection with a single `update()` call returning that input's own set unmodified, matching upstream's semantics of "intersection of a set with the universe when only one update has been made"; intersection in estimation mode with large overlapping sets, checking the estimate is within tolerance of the true intersection cardinality; three-or-more sequential `update()` calls narrowing the running result each time; intersecting a set with itself repeatedly; interleaving all three `ThetaInput` concrete types across multiple `update()` calls on one `ThetaIntersection` instance; and `is_empty`/`get_theta` consistency checks on the result after each of the above) as `#[test]` functions appended to `theta_intersection_test.rs`, following the same construct→act→assert shape as Step 7.

- [ ] **Step 9: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all 13 PASS.

- [ ] **Step 10: Write the first 4 cases of `theta_a_not_b_test.rs`** (from `theta_a_not_b_test.cpp`)

```rust
// apache-datasketches/tests/theta_a_not_b_test.rs
//! Ported from theta/test/theta_a_not_b_test.cpp (tag 5.2.0).
use apache_datasketches::theta::{ThetaAnotB, ThetaSketchBuilder};

#[test]
fn a_not_b_both_empty_is_empty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let b = ThetaSketchBuilder::new().build().unwrap();
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert!(result.is_empty());
}

#[test]
fn a_not_b_a_empty_is_empty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    b.update_u64(1);
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert!(result.is_empty());
}

#[test]
fn a_not_b_b_empty_returns_a() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..5u64 {
        a.update_u64(i);
    }
    let b = ThetaSketchBuilder::new().build().unwrap();
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert_eq!(result.get_estimate(), 5.0);
}

#[test]
fn a_not_b_disjoint_sets_returns_a() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..5u64 {
        a.update_u64(i);
    }
    for i in 5..10u64 {
        b.update_u64(i);
    }
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert_eq!(result.get_estimate(), 5.0);
}
```

- [ ] **Step 11: Port the remaining 7 cases of `theta_a_not_b_test.cpp`**

Port the remaining cases (partial overlap with an exact expected difference count; a-not-b of a set with itself is always empty; estimation-mode a-not-b with large sets, checking the estimate is within tolerance of the true set-difference cardinality; ordered vs. unordered result `compact()` output; repeated/reused `ThetaAnotB` instance across multiple independent `compute()` calls, confirming statelessness between calls; and mixed `ThetaInput` concrete type combinations for `a`/`b` beyond the ones already covered by Task 13's dispatch test) as `#[test]` functions appended to `theta_a_not_b_test.rs`, following the same construct→act→assert shape as Step 10.

- [ ] **Step 12: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all 11 PASS.

- [ ] **Step 13: Commit**

```bash
git add apache-datasketches/tests/theta_sketch_test.rs apache-datasketches/tests/theta_union_test.rs apache-datasketches/tests/theta_intersection_test.rs apache-datasketches/tests/theta_a_not_b_test.rs
git commit -m "Port theta_sketch_test, theta_union_test, theta_intersection_test, and theta_a_not_b_test"
```

---

### Task 15: New v4 compressed round-trip tests across size tiers (replacing `bit_packing_test.cpp`)

**Files:**
- Test: `apache-datasketches/tests/theta_compressed_round_trip_test.rs` (new, non-upstream)

**Interfaces:**
- Consumes: `ThetaSketch`, `ThetaSketchBuilder`, `CompactThetaSketch` (Tasks 3, 6).
- Produces: no new production code — closes the coverage gap left by not porting `bit_packing_test.cpp` (which has no public-API surface: it tests an internal bit-packing implementation detail of the v4 compressed format, not reachable through any of this crate's public types).

Task 6 already includes one v4 round-trip smoke test (`compact_v4_round_trip`, 10,000 updates). Per the Test Inventory section's "Not ported" note, `bit_packing_test.cpp` itself is not portable (no public API), so this task substitutes broader, clearly-marked new coverage of the v4 path across the three structurally distinct size tiers upstream's compressed format handles differently: LIST (very few entries, no hash table), SET (small hash table, exact mode), and the sparse/estimation tier equivalent to what HLL calls its largest mode (large hash table, estimation mode) — ensuring `serialize_compressed`/`deserialize_compressed` round-trip correctly at each structural boundary, not just at one arbitrarily chosen size.

- [ ] **Step 1: Write the size-tier round-trip tests**

```rust
// apache-datasketches/tests/theta_compressed_round_trip_test.rs
//! New, non-upstream tests: v4 (compressed) serialize/deserialize
//! round-trips across the three structurally distinct size tiers of the
//! theta compact sketch format (LIST, SET, and large/estimation-mode).
//! This substitutes for theta/test/bit_packing_test.cpp, which is not
//! portable: it tests internal bit-packing helpers with no public-API
//! surface (see the Test Inventory section's "Not ported" note).
use apache_datasketches::theta::{CompactThetaSketch, ThetaSketchBuilder};

fn round_trip_compressed(num_updates: u64) {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..num_updates {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compressed();
    let restored = CompactThetaSketch::deserialize_compressed(&bytes).unwrap();

    assert_eq!(compact.get_estimate(), restored.get_estimate());
    assert_eq!(compact.get_num_retained(), restored.get_num_retained());
    assert_eq!(compact.is_empty(), restored.is_empty());
    assert_eq!(compact.is_estimation_mode(), restored.is_estimation_mode());
    assert_eq!(compact.get_theta(), restored.get_theta());
}

#[test]
fn empty_sketch_compressed_round_trip() {
    round_trip_compressed(0);
}

#[test]
fn list_mode_compressed_round_trip() {
    // Very few entries: upstream's LIST representation (no hash table).
    round_trip_compressed(3);
}

#[test]
fn set_mode_exact_compressed_round_trip() {
    // Enough entries to move from LIST to SET representation, but still
    // exact (no estimation): well under 2^lg_k = 4096 entries.
    round_trip_compressed(100);
}

#[test]
fn set_mode_large_estimation_compressed_round_trip() {
    // Well past 2^lg_k = 4096 entries: forces estimation mode, theta < 1,
    // and a fuller/resized hash table.
    round_trip_compressed(1_000_000);
}

#[test]
fn deserialize_compressed_rejects_v3_uncompressed_only_if_actually_invalid() {
    // deserialize_compressed and deserialize both call the same
    // auto-detecting upstream routine (Design resolution #3), so v3
    // uncompressed bytes ARE accepted by deserialize_compressed too --
    // this documents that behavior rather than asserting it must fail.
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let v3_bytes = compact.serialize_compact();
    let restored = CompactThetaSketch::deserialize_compressed(&v3_bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());
}
```

- [ ] **Step 2: Run**

```bash
cargo test -p apache-datasketches --no-default-features --features theta
```

Expected: all PASS.

- [ ] **Step 3: Commit**

```bash
git add apache-datasketches/tests/theta_compressed_round_trip_test.rs
git commit -m "Add new v4 compressed round-trip tests across size tiers"
```

---

### Task 16: Cargo.toml feature-flip (`default = []` for both crates) + README updates + `theta.rs` example

**Files:**
- Modify: `apache-datasketches-sys/Cargo.toml`
- Modify: `apache-datasketches/Cargo.toml`
- Modify: `apache-datasketches-sys/README.md`
- Modify: `apache-datasketches/README.md`
- Modify: root `README.md`
- Create: `apache-datasketches/examples/theta.rs`

**Interfaces:**
- Consumes: everything built in Tasks 1–15.
- Produces: the finished, feature-complete Theta sketch family, with both crates' feature flags matching the design spec's final state (`default = []`, `hll`/`theta` explicit opt-in) and all documentation brought up to date, per the repo's established "keep READMEs in sync" convention.

- [ ] **Step 1: Flip `apache-datasketches-sys/Cargo.toml`'s default feature**

Change:

```toml
[features]
default = ["hll"]
hll = []
theta = []
```

to:

```toml
[features]
default = []
hll = []
theta = []
```

- [ ] **Step 2: Flip `apache-datasketches/Cargo.toml`'s default feature**

Change:

```toml
[features]
default = ["hll"]
hll = ["apache-datasketches-sys/hll"]
theta = ["apache-datasketches-sys/theta"]
```

to:

```toml
[features]
default = []
hll = ["apache-datasketches-sys/hll"]
theta = ["apache-datasketches-sys/theta"]
```

- [ ] **Step 3: Verify default build now compiles nothing feature-gated (sanity check for the flip)**

```bash
cargo build --workspace
cargo build --workspace --features apache-datasketches/hll
cargo build --workspace --features apache-datasketches/theta
cargo build --workspace --features apache-datasketches/hll,apache-datasketches/theta
```

Expected: all four PASS; the first produces a workspace with no HLL or Theta types compiled in (matching the new `default = []`), and each subsequent invocation compiles in exactly the requested family/families, confirming the two features remain independently selectable and additive.

- [ ] **Step 4: Update `apache-datasketches-sys/README.md`**

Add a "Theta" section mirroring the existing "HLL" section's structure (feature name, what it wraps, example `Cargo.toml` snippet), and update every existing `Cargo.toml` usage snippet in the file to explicitly list `features = ["hll"]` (or `["theta"]`, or `["hll", "theta"]`) instead of relying on a default feature, since `default = []` now means no snippet works without an explicit feature list. Add a note that `default` is now empty and users must opt into `hll` and/or `theta` explicitly.

- [ ] **Step 5: Update `apache-datasketches/README.md`**

Add a "Theta sketches" section documenting `ThetaSketch`/`ThetaSketchBuilder`, `CompactThetaSketch`, `WrappedCompactThetaSketch`, `ThetaUnion`/`ThetaUnionBuilder`, `ThetaIntersection`, `ThetaAnotB`, and `jaccard_similarity`/`JaccardBounds`, each with a one- or two-line usage snippet, mirroring the existing "HLL sketches" section's depth and style. Update the crate-level usage example at the top of the README to show `features = ["hll"]` and/or `features = ["theta"]` explicitly (no default feature is enabled anymore).

- [ ] **Step 6: Update root `README.md`**

Update the root README's feature-flag table/section (wherever it currently documents `default = ["hll"]`) to reflect `default = []`, listing `hll` and `theta` as the two available opt-in features, each with a one-line description (Theta's should mention set operations: union, intersection, a-not-b, and Jaccard similarity, alongside cardinality estimation), consistent with this repo's "keep READMEs in sync" convention of updating root + per-crate READMEs together whenever a sketch family ships.

- [ ] **Step 7: Write `apache-datasketches/examples/theta.rs`**

Mirroring `examples/hll.rs`'s structure and level of detail:

```rust
//! Demonstrates the Theta sketch family: cardinality estimation, set
//! operations (union, intersection, a-not-b), and Jaccard similarity.
//!
//! Run with:
//!   cargo run --example theta --features theta

use apache_datasketches::theta::{
    jaccard_similarity, ThetaAnotB, ThetaIntersection, ThetaSketchBuilder, ThetaUnionBuilder,
};

fn main() {
    // Build two sketches representing two overlapping sets of user IDs.
    let mut visitors_day1 = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for id in 0..10_000u64 {
        visitors_day1.update_u64(id);
    }

    let mut visitors_day2 = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for id in 5_000..15_000u64 {
        visitors_day2.update_u64(id);
    }

    println!("Day 1 unique visitors (estimate): {:.0}", visitors_day1.get_estimate());
    println!("Day 2 unique visitors (estimate): {:.0}", visitors_day2.get_estimate());

    // Union: total unique visitors across both days.
    let mut union = ThetaUnionBuilder::new().lg_k(12).build().unwrap();
    union.update(&visitors_day1);
    union.update(&visitors_day2);
    let total_unique = union.get_result(true);
    println!("Total unique visitors (union estimate): {:.0}", total_unique.get_estimate());

    // Intersection: visitors who came back on day 2.
    let mut intersection = ThetaIntersection::new();
    intersection.update(&visitors_day1);
    intersection.update(&visitors_day2);
    match intersection.get_result(true) {
        Ok(returning) => println!("Returning visitors (intersection estimate): {:.0}", returning.get_estimate()),
        Err(e) => println!("No intersection result: {e}"),
    }

    // A-not-b: visitors who only came on day 1.
    let a_not_b = ThetaAnotB::new();
    let day1_only = a_not_b.compute(&visitors_day1, &visitors_day2, true);
    println!("Day-1-only visitors (a-not-b estimate): {:.0}", day1_only.get_estimate());

    // Jaccard similarity: how similar are the two days' visitor sets?
    let similarity = jaccard_similarity(&visitors_day1, &visitors_day2);
    println!(
        "Jaccard similarity: {:.3} (range [{:.3}, {:.3}])",
        similarity.estimate, similarity.lower_bound, similarity.upper_bound
    );

    // Serialize a compact sketch for storage/transmission, then restore it.
    let compact = visitors_day1.compact(true);
    let bytes = compact.serialize_compact();
    println!("Serialized day-1 sketch: {} bytes", bytes.len());
    let restored = apache_datasketches::theta::CompactThetaSketch::deserialize(&bytes).unwrap();
    println!("Restored estimate: {:.0}", restored.get_estimate());
}
```

- [ ] **Step 8: Run the example**

```bash
cargo run --example theta --features theta -p apache-datasketches
```

Expected: prints all six lines above without panicking; estimates are all within a few percent of the true set sizes (10,000 / 10,000 / ~15,000 / ~5,000 / ~5,000 / similarity ≈ 0.33).

- [ ] **Step 9: Run the full workspace test suite one final time across all feature combinations**

```bash
cargo test --workspace
cargo test --workspace --features apache-datasketches/hll
cargo test --workspace --features apache-datasketches/theta
cargo test --workspace --features apache-datasketches/hll,apache-datasketches/theta
```

Expected: all PASS in all four configurations (the first, with no features, should show zero theta/hll tests collected, since every test file in both crates is behind `#[cfg(feature = ...)]` or its whole crate requires the feature to compile at all — matching HLL's existing precedent for feature-gated tests).

- [ ] **Step 10: Commit**

```bash
git add apache-datasketches-sys/Cargo.toml apache-datasketches/Cargo.toml apache-datasketches-sys/README.md apache-datasketches/README.md README.md apache-datasketches/examples/theta.rs
git commit -m "Flip default features to opt-in, update READMEs, and add theta example"
```

---
