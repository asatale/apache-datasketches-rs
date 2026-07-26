# Apache DataSketches Theta Rust Bindings — Design

## Purpose

Add the Theta sketch family to the existing `apache-datasketches`/
`apache-datasketches-sys` crates, following the pattern established by
[the HLL design](2026-07-26-hll-rust-bindings-design.md): a `cxx` shim over
the vendored `datasketches-cpp` submodule, a safe idiomatic wrapper, and a
1:1-ported C++ test suite. This design covers what's different or new about
Theta; anything not mentioned here (FFI tool choice, sys/safe crate split,
`SketchError`, submodule versioning, licensing, no CI) carries over unchanged
from the HLL design.

**Scope of this design:** the Theta sketch family — `update_theta_sketch`,
`compact_theta_sketch`, `wrapped_compact_theta_sketch`, `theta_union`,
`theta_intersection`, `theta_a_not_b`, and Jaccard similarity.

## Why Theta needs a different type shape than HLL

HLL uses one C++ class (`hll_sketch`) for both building and serializing.
Theta's upstream API is structurally different: `update_theta_sketch` (has
`update()`/`trim()`/`reset()`/`compact()`, no `serialize()`) and
`compact_theta_sketch` (has `serialize()`/`is_ordered()`, no `update()`) are
genuinely distinct classes, both deriving from a common `theta_sketch` base
used polymorphically wherever set operations accept "any theta sketch."
Following HLL's single-type precedent here would mean re-adding, in Rust, a
runtime state check upstream's own type system already gives away for free.
So this design uses **three distinct Rust types** instead:

- **`ThetaSketch`** — mutable, update-only. Built via `ThetaSketchBuilder`
  (`lg_k`, `resize_factor`, `p`, then `.build()` — no public constructor,
  matching upstream). Methods: `update_u64`/`update_i64`/`update_f64`/
  `update_str`/`update_bytes` (parity with HLL) plus `update_i8`/`update_u8`/
  `update_i16`/`update_u16`/`update_i32`/`update_u32` (theta-specific,
  present upstream for byte-compatible hashing with the Java library's
  narrower integer types). Also `trim()`, `reset()`, and
  `compact(ordered: bool) -> CompactThetaSketch`.
- **`CompactThetaSketch`** — immutable, serializable. Produced by
  `ThetaSketch::compact()`, by any set operation's result, or by
  deserializing. Has `get_estimate()`/`get_lower_bound()`/`get_upper_bound()`/
  `is_ordered()` (shared query surface with `ThetaSketch`, exposed on both
  types rather than through a shared Rust trait — see below) plus
  `serialize_compact(ordered: bool) -> Vec<u8>` / `serialize_compressed() ->
  Vec<u8>` and `CompactThetaSketch::deserialize(&[u8])` /
  `deserialize_compressed(&[u8])`.
- **`WrappedCompactThetaSketch<'a>`** — a zero-copy view over an
  already-serialized `&'a [u8]` buffer (upstream's
  `wrapped_compact_theta_sketch`), usable directly as set-operation input
  without a full deserialize. Query-only (no `serialize()` of its own — reuse
  the original bytes if you need them again).

Shared query methods (`get_estimate`, `get_lower_bound`, `get_upper_bound`,
`is_empty`, `is_estimation_mode`, `get_theta`, `get_num_retained`) are
implemented individually on each of the three types rather than factored into
a shared trait, since (a) each delegates to a different concrete shim type
underneath and (b) HLL's precedent doesn't use a query trait either — no
existing convention to break. Revisit if a fourth sketch family makes the
duplication painful.

## Feature flags: everything becomes opt-in

Both crates currently ship `default = ["hll"]`. This design changes that to
`default = []` for both crates, with `hll = []`/`theta = []` as explicit
opt-in features users must request. This is a breaking change to the
published `apache-datasketches`/`apache-datasketches-sys` `0.1.1`, ships as
`0.2.0` alongside Theta, and is acceptable now specifically because neither
crate has real external users yet to break. Going the other direction later
(removing something from `default`) would be a breaking change requiring
another major bump; starting opt-in and later promoting to `default` is a
free, non-breaking move — so opt-in is the low-regret choice for both
existing and new families. The root `README.md` and both crates' READMEs get
updated to show the explicit `features = [...]` a caller now needs.

## Seed

Every theta constructor, builder, and (de)serialize call upstream takes a
numeric seed (default `DEFAULT_SEED`); sketches built with different seeds
cannot be combined in set operations. This design always uses
`DEFAULT_SEED` internally and does **not** expose a seed parameter on any
type in v1 — covers the overwhelming majority of usage, and a seed parameter
can be added later as a non-breaking builder addition (an optional
`set_seed()` builder method, defaulting to today's implicit behavior).

## Set operations and the `ThetaInput` trait

`ThetaUnion`, `ThetaIntersection`, and `ThetaAnotB` all need to accept any of
`ThetaSketch`, `CompactThetaSketch`, or `WrappedCompactThetaSketch` as input,
mirroring upstream's polymorphism over the common `theta_sketch` base class.
Rust has no inheritance, so this design introduces a small sealed trait:

```rust
mod sealed {
    pub trait Sealed {}
}

pub trait ThetaInput: sealed::Sealed {
    #[doc(hidden)]
    fn as_theta_input(&self) -> sys::ThetaInputRef;
}
```

implemented (with the `Sealed` marker, so it can't be implemented outside
this crate) for all three sketch types. Each impl dispatches to the matching
concrete shim overload (`update_with_sketch`/`update_with_compact`/
`update_with_wrapped` on the C++ side — cxx has no generics either, so the
shim itself still needs one concrete method per input type; the trait only
hides that fan-out from callers). This gives callers a single method name —
`union.update(&some_sketch)`, `intersection.update(&some_sketch)`,
`a_not_b.compute(&a, &b, ordered)` — regardless of which concrete sketch type
they're holding, instead of three differently-named methods per operation.

- **`ThetaUnion`** — built via `ThetaUnionBuilder` (same `lg_k`/
  `resize_factor`/`p` shape as `ThetaSketchBuilder`). `update(&impl
  ThetaInput)`, `get_result(ordered: bool) -> CompactThetaSketch`, `reset()`.
- **`ThetaIntersection`** — plain constructor (`ThetaIntersection::new()`,
  no builder, matching upstream). `update(&impl ThetaInput)`,
  `get_result(ordered: bool) -> Result<CompactThetaSketch, SketchError>`
  (upstream throws if `update()` was never called — an "undefined universe"
  state — mapped to a new `SketchError::EmptyIntersection` variant rather
  than the existing catch-all, since it's a distinguishable, expected-to-occur
  condition worth matching on), `has_result() -> bool`.
- **`ThetaAnotB`** — plain constructor. Single stateless method:
  `compute(&impl ThetaInput, &impl ThetaInput, ordered: bool) ->
  CompactThetaSketch` ("A" minus "B", unlike union/intersection there's no
  accumulation across repeated calls).

## Jaccard similarity

A free function, not a type:

```rust
pub struct JaccardBounds {
    pub lower_bound: f64,
    pub estimate: f64,
    pub upper_bound: f64,
}

pub fn jaccard_similarity(a: &impl ThetaInput, b: &impl ThetaInput) -> JaccardBounds;
```

mirroring upstream's `theta_jaccard_similarity` utility, which computes a
similarity estimate (with bounds) between two theta sketches without
requiring either to be mutated or consumed.

## Serialization: both v3 and v4

Theta has two on-wire formats: v3 (uncompressed, structurally analogous to
HLL's compact/updatable split) and v4 (compressed, using the
`bit_packing.hpp` codec). Both are in scope for v1. The compression codec
itself is already-implemented, header-only C++ that gets vendored and
compiled like the rest of theta's headers — the shim only needs to call the
existing `serialize_compressed()`/matching `deserialize()` methods on
`compact_theta_sketch`; no codec logic is reimplemented in the shim or in
Rust.

`CompactThetaSketch` exposes:
- `serialize_compact(ordered: bool) -> Vec<u8>` (v3)
- `serialize_compressed() -> Vec<u8>` (v4)
- `CompactThetaSketch::deserialize(bytes: &[u8]) -> Result<Self, SketchError>`
  (auto-detects v1/v2/v3 on read, per upstream)
- `CompactThetaSketch::deserialize_compressed(bytes: &[u8]) ->
  Result<Self, SketchError>` (v4)

## Repo layout additions

```
apache-datasketches-sys/
├── vendor/datasketches-cpp/theta/include/   copied from the root submodule
├── cpp/theta/
│   ├── theta_sketch_shim.h/.cc              ThetaSketchShim + builder
│   ├── theta_compact_shim.h/.cc             CompactThetaSketchShim (incl. v3/v4 (de)serialize)
│   ├── theta_wrapped_shim.h/.cc             WrappedCompactThetaSketchShim
│   ├── theta_union_shim.h/.cc               ThetaUnionShim + builder
│   ├── theta_intersection_shim.h/.cc        ThetaIntersectionShim
│   ├── theta_a_not_b_shim.h/.cc             ThetaAnotBShim
│   └── theta_jaccard_shim.h/.cc             free jaccard_similarity() shim function
└── src/
    ├── theta_sketch.rs                      one #[cxx::bridge] module per shim pair
    ├── theta_compact.rs
    ├── theta_wrapped.rs
    ├── theta_union.rs
    ├── theta_intersection.rs
    ├── theta_a_not_b.rs
    └── theta_jaccard.rs

apache-datasketches/src/theta/
├── mod.rs
├── builder.rs        ThetaSketchBuilder, ThetaUnionBuilder, ResizeFactor
├── sketch.rs          ThetaSketch
├── compact.rs         CompactThetaSketch
├── wrapped.rs          WrappedCompactThetaSketch<'a>
├── union.rs            ThetaUnion
├── intersection.rs     ThetaIntersection
├── a_not_b.rs          ThetaAnotB
└── jaccard.rs          jaccard_similarity(), JaccardBounds

apache-datasketches/examples/theta.rs
```

One `#[cxx::bridge]` module per shim pair (seven files, vs. HLL's one file
for both sketch+union) — theta's surface is roughly 2.5x HLL's, and splitting
by class keeps each bridge/shim file focused and independently reviewable,
consistent with this project's existing preference for small, single-purpose
files.

`build.rs` gains `.include(vendor_dir.join("theta/include"))` and, under
`cfg!(feature = "theta")`, compiles all seven `cpp/theta/*_shim.cc` files.
`vendor/README.md`'s manual sync script gains a `theta/include` copy step
alongside the existing `common`/`hll` ones.

## `ResizeFactor`

A `cxx`-shared enum (same pattern as HLL's `TargetHllType`), mapping to
upstream's resize-factor values (`X1`, `X2`, `X4`, `X8`), used by both
`ThetaSketchBuilder` and `ThetaUnionBuilder`. Default (when unset on the
builder) matches upstream's own default (`X8`).

## Error handling

Reuses the existing single `SketchError` enum (no new error type for Theta).
One new variant, `SketchError::EmptyIntersection`, covers
`ThetaIntersection::get_result()` being called before any `update()` — a
distinguishable, expected-to-occur condition (not a C++-exception-message
catch-all) that callers may want to match on specifically (e.g. to
distinguish "no data yet" from a genuine error).

## Testing

1:1-ported (same header-comment-linking-to-upstream convention as HLL's
`hll_sketch_test.rs`/`hll_union_test.rs`):
- `theta_sketch_test.rs` ← `theta_sketch_test.cpp`
- `theta_union_test.rs` ← `theta_union_test.cpp`
- `theta_intersection_test.rs` ← `theta_intersection_test.cpp`
- `theta_a_not_b_test.rs` ← `theta_a_not_b_test.cpp`
- `theta_setop_test.rs` ← `theta_setop_test.cpp`
- `theta_jaccard_similarity_test.rs` ← `theta_jaccard_similarity_test.cpp`

Not ported:
- `bit_packing_test.cpp` — unit-tests internal C++ template functions in
  `bit_packing.hpp` that are never called directly through any public sketch
  API (the shim only ever calls `serialize_compressed()`/`deserialize()` at
  the sketch level), so there is nothing on the Rust side to call 1:1.
  Coverage is provided instead by new (clearly-marked, non-upstream)
  sketch-level round-trip tests exercising `serialize_compressed`/
  `deserialize_compressed` across the same LIST/SET/HLL-equivalent size
  tiers the ported tests already use. The upstream C++ test itself is left
  untouched and still runs as part of datasketches-cpp's own test suite.
- `theta_sketch_deserialize_from_java_test.cpp` /
  `theta_sketch_serialize_for_java.cpp` — byte-for-byte interop fixture
  tests against the Java library's own test data; no Java-side counterpart
  exists in this project.

Additional Rust-specific tests (beyond the HLL precedent's `Send`
verification, `SketchError` conversion, feature-flag compilation, all of
which apply unchanged to Theta): `ThetaInput` trait dispatch across all
three concrete input types for each of union/intersection/a-not-b/jaccard,
and v4 compressed round-trip coverage in place of `bit_packing_test.cpp`
(see above).

## Out of scope for this design

- Exposing a custom seed parameter (see "Seed" above) — default-seed-only
  for v1.
- Sketch families other than HLL and Theta (KLL, CPC, Tuple, REQ, Frequent
  Items, VarOpt/Sampling, etc.) — each remains its own follow-up
  design/implementation cycle.
- CI/CD, prebuilt binaries — unchanged from the HLL design.
