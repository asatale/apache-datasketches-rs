# Apache DataSketches CPC Rust Bindings — Design

## Purpose

Add the CPC (Compressed Probabilistic Counting) sketch family to the
existing `apache-datasketches`/`apache-datasketches-sys` crates, following
the pattern established by
[the HLL design](2026-07-26-hll-rust-bindings-design.md) and
[the Theta design](2026-07-26-theta-rust-bindings-design.md): a `cxx` shim
over the vendored `datasketches-cpp` submodule, a safe idiomatic wrapper,
and a 1:1-ported C++ test suite. This design covers what's different or new
about CPC; anything not mentioned here (FFI tool choice, sys/safe crate
split, `SketchError`, submodule versioning, licensing, no CI) carries over
unchanged from the HLL/Theta designs.

**Scope of this design:** the CPC sketch family — `cpc_sketch` and
`cpc_union` only. Unlike Theta, CPC has no intersection/a-not-b/Jaccard —
those set operations don't exist upstream for CPC.

## Why CPC is structurally simpler than Theta

Theta needed three Rust types (`ThetaSketch`/`CompactThetaSketch`/
`WrappedCompactThetaSketch`) because upstream's update and compact sketches
are genuinely distinct C++ classes, and because Theta has two wire formats
(v3 uncompressed, v4 compressed). CPC has neither complication:

- **One sketch type upstream, one sketch type here**: `cpc_sketch_alloc<A>`
  is simultaneously the mutable/update type and the serializable type — no
  separate compact/wrapped variant. This design uses a single
  **`CpcSketch`** Rust type.
- **One wire format**: CPC's on-disk representation is compressed by
  construction (that's the whole point of the algorithm), so there is only
  one `serialize()`/`deserialize()` pair — no v3/v4 split to mirror.
- **One set operation**: `cpc_union_alloc<A>` (→ **`CpcUnion`**), no
  intersection/a-not-b/Jaccard equivalents upstream.

## API surface (CPC, v1)

- **`CpcSketchBuilder`**: `.lg_k(u8)` (validated `4..=26`, default `11`,
  matching upstream's `cpc_constants::{MIN,MAX,DEFAULT}_LG_K`), then
  `.build() -> Result<CpcSketch, SketchError>`. No public constructor on
  `CpcSketch` itself, matching the `ThetaSketchBuilder` precedent.
- **`CpcSketch`** methods:
  - `update_u64`/`update_i64`/`update_u32`/`update_i32`/`update_u16`/
    `update_i16`/`update_u8`/`update_i8`/`update_f64`/`update_f32`/
    `update_str`/`update_bytes` — full parity with upstream's overload set
    (wider than HLL/Theta's u64/i64/f64/str/bytes subset, since upstream
    keeps the narrow-int/float overloads specifically for Java
    cross-compatibility and this design mirrors the C++ API exactly for
    CPC).
  - `is_empty() -> bool`, `get_estimate() -> f64`.
  - `get_lower_bound(num_std_dev: u8) -> Result<f64, SketchError>`,
    `get_upper_bound(num_std_dev: u8) -> Result<f64, SketchError>` —
    parameter named `num_std_dev` for consistency with the existing
    HLL/Theta public API, validated to `1..=3` before crossing FFI (maps to
    upstream's `kappa` parameter, which is restricted to the same range).
  - `to_string_summary() -> String` — mirrors HLL's method of the same
    name (upstream `to_string()`; Theta doesn't have an equivalent).
  - `serialize() -> Vec<u8>`, `CpcSketch::deserialize(bytes: &[u8]) ->
    Result<Self, SketchError>`. Upstream's optional `header_size_bytes`
    parameter on `serialize()` (used by the DataSketches PostgreSQL
    extension) is not exposed — always `0`.
  - Not exposed: `get_num_coupons()`, `validate()` — both marked
    `@private` upstream (internal/debugging use only).
- **`get_max_serialized_size_bytes(lg_k: u8) -> usize`**: free function
  (not a method — upstream's version is `static`, independent of any
  sketch instance), useful for callers pre-allocating buffers.
- **`CpcUnion`**: built via `CpcUnionBuilder` with `.lg_k(u8)` (same
  `4..=26`/default-`11` validation as `CpcSketchBuilder`; if unset, uses
  upstream's own default), mirroring `ThetaUnionBuilder`'s shape even
  though upstream's union constructor takes `lg_k` as a plain parameter —
  the builder wrapper is for API consistency across the union types in this
  crate, not because upstream needs one. `.build() -> Result<CpcUnion,
  SketchError>`, then `update(&CpcSketch)`, `get_result() -> CpcSketch`.
- No `seed` parameter anywhere in the public API — same fixed-
  `DEFAULT_SEED` convention as Theta.

## Global decompression-table initialization (`cpc::init()`)

Upstream's `cpc_init<A>()` eagerly allocates global decompression tables
used during serialization/deserialization. If never called explicitly, the
tables lazily self-initialize on first use — and upstream's own doc comment
states this lazy path **is not thread-safe**: concurrent first-use from two
threads racing to initialize the same global state is a real hazard, unlike
anything in HLL or Theta (which hold no global lazily-initialized state;
each sketch instance is independent).

This design adds a safe wrapper:

```rust
pub fn init();
```

in a new `apache_datasketches::cpc` module-level function, documented as:
"call once, single-threaded, before spawning worker threads that will
serialize or deserialize CPC sketches concurrently." It is not called
automatically by any other CPC function — callers doing single-threaded CPC
work never need it (upstream's lazy self-init is fine there); callers doing
concurrent CPC work must call it up front to avoid the race. A dedicated
test exercises calling `init()` before spawning threads that each build,
update, and serialize a `CpcSketch` concurrently.

## Repo layout additions

```
apache-datasketches-sys/
├── vendor/datasketches-cpp/cpc/include/   copied from the root submodule
├── cpp/cpc/
│   ├── cpc_sketch_shim.h/.cc              CpcSketchShim + builder + init()
│   └── cpc_union_shim.h/.cc               CpcUnionShim + builder
└── src/
    ├── cpc_sketch.rs                      #[cxx::bridge] module
    └── cpc_union.rs

apache-datasketches/src/cpc/
├── mod.rs
├── builder.rs        CpcSketchBuilder, CpcUnionBuilder
├── sketch.rs         CpcSketch
├── union.rs          CpcUnion
└── init.rs           cpc::init()

apache-datasketches/examples/cpc.rs
```

`build.rs` gains `.include(vendor_dir.join("cpc/include"))` and, under
`cfg!(feature = "cpc")`, compiles both `cpp/cpc/*_shim.cc` files.
`vendor/README.md`'s manual sync script gains a `cpc/include` copy step
alongside the existing `common`/`hll`/`theta` ones. Third Cargo feature:
`cpc`, additive with `hll`/`theta`, still `default = []`.

## Error handling

Reuses the existing single `SketchError` enum — no new variant needed for
CPC (no distinguishable "expected" error condition analogous to Theta's
`EmptyIntersection`; CPC's own failure modes are configuration validation,
already covered by `InvalidConfig`, and deserialization, already covered by
`Deserialization`).

## Concurrency

`CpcSketch` and `CpcUnion` are `unsafe impl Send`, not `Sync` — same
convention as HLL/Theta. The `cpc::init()` function addresses the one
CPC-specific concurrency hazard (see above) that doesn't exist for the
other two families.

## Testing

1:1-ported (same header-comment-linking-to-upstream convention as the
existing test files):
- `cpc_sketch_test.rs` ← `cpc_sketch_test.cpp`
- `cpc_union_test.rs` ← `cpc_union_test.cpp`

Not ported:
- `cpc_sketch_allocation_test.cpp` — exercises custom C++ allocators, not
  representable through this crate's public API (no allocator parameter is
  exposed, same rationale as every other allocator-focused upstream test
  skipped so far).
- `compression_test.cpp` — unit-tests the internal compression codec
  directly (`cpc_compressor.hpp`), never called except through
  `serialize()`/`deserialize()` at the sketch level; same "not portable, no
  public API surface" rationale as Theta's excluded `bit_packing_test.cpp`.
- `cpc_sketch_deserialize_from_java_test.cpp` /
  `cpc_sketch_serialize_for_java.cpp` — byte-for-byte interop fixture tests
  against the Java library's own test data; no Java-side counterpart exists
  in this project (same rationale as Theta's excluded java-fixture cases).

Additional Rust-specific tests (beyond the HLL/Theta precedent's `Send`
verification, `SketchError` conversion, feature-flag compilation, all of
which apply unchanged to CPC): the `cpc::init()` concurrent-use test
described above, and builder validation for `lg_k` outside `4..=26` and
`num_std_dev` outside `1..=3`.

## Examples

`apache-datasketches/examples/cpc.rs`, mirroring `examples/hll.rs` and
`examples/theta.rs`'s structure and depth: build two overlapping `CpcSketch`
instances, print cardinality estimates and bounds, union them, serialize
and restore one via `serialize()`/`deserialize()`, and print the restored
estimate. Runnable via `cargo run --example cpc --features cpc`.

## Out of scope for this design

- Exposing a custom seed parameter — default-seed-only for v1, same as
  Theta.
- Exposing `header_size_bytes` on `serialize()` — PostgreSQL-extension-
  specific, no use case in this crate.
- Sketch families other than HLL, Theta, and CPC (KLL, Tuple, REQ, Frequent
  Items, VarOpt/Sampling, Count-Min, Bloom filters, T-Digest, Density) —
  each remains its own follow-up design/implementation cycle.
- CI/CD, prebuilt binaries — unchanged from the HLL/Theta designs.
