# Apache DataSketches ArrayOfDoubles (Tuple) Rust Bindings — Design

## Purpose

Add the concrete `array_of_doubles_sketch` instantiation of the Tuple
sketch family to the existing `apache-datasketches`/`apache-datasketches-sys`
crates, following the pattern established by
[the HLL design](2026-07-26-hll-rust-bindings-design.md),
[the Theta design](2026-07-26-theta-rust-bindings-design.md), and
[the CPC design](2026-07-27-cpc-rust-bindings-design.md): a `cxx` shim over
the vendored `datasketches-cpp` submodule, a safe idiomatic wrapper, and a
1:1-ported C++ test suite. This design covers what's different or new;
anything not mentioned here (FFI tool choice, sys/safe crate split,
`SketchError`, submodule versioning, licensing, no CI) carries over
unchanged from the prior designs.

**Scope of this design:** `array_of_doubles_sketch` only — upstream's
pre-instantiated, non-generic Tuple sketch where each retained entry's
summary is a fixed-size array of `f64`s, combined by summing on collision.
This is the equivalent of Java's `ArrayOfDoublesSketch`, the Tuple sketch
family's most commonly used concrete form.

**Explicitly out of scope, by design:** the fully generic
`tuple_sketch<Summary, Update, Policy, Allocator>`, where a C++ caller
supplies their own `Summary`/`Policy` types as compile-time template
parameters. This can't be exposed through Rust FFI the way this project's
existing bindings work (all of them bind one concrete instantiation fixed
at `std::allocator<uint8_t>`, never a caller-supplied C++ type). Supporting
arbitrary user-defined summaries would require a type-erased
`Box<dyn TupleSummary>`-based callback framework where C++ calls back into
Rust-authored combine/serialize logic on every hash-table operation — a
substantial, higher-risk undertaking with no code-level overlap with this
design (see "Future generic framework" below). That is intentionally a
**separate, later design and plan**, not part of this one.

## Why ArrayOfDoublesSketch needs a different type shape than HLL/CPC, but the same shape as Theta

Upstream's `update_array_tuple_sketch<array<double>>` (the update/mutable
type) and `compact_array_tuple_sketch<array<double>>` (the immutable,
serializable type) are genuinely distinct C++ classes — the same situation
that drove Theta's three-type design. This design uses **two** Rust types
(no third "wrapped" type — upstream provides no zero-copy wrapped variant
for array-tuple sketches, unlike Theta):

- **`ArrayOfDoublesSketch`** — mutable, update-only. Built via
  `ArrayOfDoublesSketchBuilder` (`lg_k`, `resize_factor`, `p` — inherited
  from the same `theta_base_builder` Theta's own builder uses, plus
  `num_values`, new to this family: the fixed number of `f64`s each
  retained entry carries, set once at construction and shared by every
  sketch that will ever be unioned/intersected with this one. Defaults to
  `1`, matching upstream's `default_array_tuple_update_policy`'s own
  default).
- **`CompactArrayOfDoublesSketch`** — immutable, serializable. Produced by
  `ArrayOfDoublesSketch::compact()`, by any set operation's result, or by
  deserializing.

Both types support `get_estimate()`/`get_lower_bound()`/`get_upper_bound()`/
`is_empty()`/`is_estimation_mode()`/`is_ordered()`/`get_theta()`/
`get_num_retained()`, matching Theta's shared query surface (implemented
individually on each type, not via a shared trait, consistent with every
prior family's no-query-trait precedent), plus the two capabilities unique
to this family:

- **`get_num_values() -> u8`** — the fixed array width this sketch was
  built with.
- **`entries() -> impl Iterator<Item = (u64, &[f64])>`** — per-entry
  access to each retained item's hash and associated values. This is
  genuinely new: HLL, Theta, and CPC only ever expose aggregate statistics,
  because presence/absence is all they track. Tuple sketches exist
  specifically to carry a value *per retained item*, so read access to
  those values is core functionality, not an afterthought. Backed by a
  shim method that materializes parallel `Vec<u64>` (hashes) and `Vec<f64>`
  (values, `num_values` per entry, concatenated) — cxx has no mechanism to
  hand back a live C++ iterator, so the entries are copied out in one call
  and the safe wrapper zips/chunks them into the iterator above.

## The update contract: a real safety requirement, not just an API nicety

`ArrayOfDoublesSketch::update(key: u64, values: &[f64])` requires
`values.len() == num_values`. Upstream's `default_array_tuple_update_policy`
loops `for i in 0..num_values_` indexing into whatever's passed, **with no
bounds check of its own** — passing a shorter slice through the C++ layer
unchecked would be an out-of-bounds read, not a graceful failure. The safe
wrapper validates `values.len() == num_values` in Rust and returns
`SketchError::InvalidConfig` before the slice ever crosses the FFI
boundary. This validation cannot be delegated to the existing
C++-exception-to-`Result` pattern the way `lg_k`/`num_std_dev` validation
is elsewhere in this project, since the C++ side has no validation to
delegate to.

## Set operations

Same shape as Theta's, restricted to same-family operands only — no
cross-family generic dispatch (a `ThetaInput`-style sealed trait isn't
needed here, since this design has exactly one input type,
`ArrayOfDoublesSketch`/`CompactArrayOfDoublesSketch`, not three):

- **`ArrayOfDoublesUnion`** — built via `ArrayOfDoublesUnionBuilder`
  (`lg_k`/`resize_factor`/`p`/`num_values`, same shape as the sketch
  builder). Sums values on collision — upstream's only default union
  policy (`default_array_tuple_union_policy`).
- **`ArrayOfDoublesIntersection`** — plain constructor (`num_values` plus
  the fixed seed), no builder, matching `ThetaIntersection`'s precedent.
  Uses a **sum** combine-on-collision policy. Upstream itself ships no
  default policy for array-of-doubles intersection ("no default policy
  since it is not clear in general," per its own header comment) — v1
  picks sum, mirroring union's behavior, as the single supported policy.
  A small fixed enum of additional policies (e.g. min/max) can be added
  later as a non-breaking extension without touching this design's public
  API shape.
- **`ArrayOfDoublesAnotB`** — stateless, single `compute(a, b, ordered)`,
  matching `ThetaAnotB`'s precedent.
- **`array_of_doubles_jaccard_similarity`** — upstream provides no
  ready-made alias for this family (unlike Theta, where
  `theta_jaccard_similarity` is built-in). This design assembles it from
  upstream's existing generic `jaccard_similarity_base<Union, Intersection,
  ExtractKey>` template, instantiated with the union/intersection types
  already in scope above — no new upstream functionality is written, only
  a template instantiation gluing existing pieces together (comparable
  integration effort to Theta's own jaccard task). Returns the same
  `JaccardBounds { lower_bound, estimate, upper_bound }` shape as Theta's.

**Cross-operand invariant:** every sketch fed into a union, intersection,
or a-not-b must share the same `num_values`. Upstream does not validate
this itself — mismatched array widths merged together would silently
misbehave (reading/writing past the shorter array's bounds) rather than
error. This design validates `num_values` equality in Rust before crossing
the FFI boundary for every set-operation `update`/`compute` call, returning
`SketchError::InvalidConfig` on mismatch.

## Error handling

Reuses the existing single `SketchError` enum — no new variant. The two
family-specific validation cases above (`values.len() != num_values` on
update; `num_values` mismatch across set-operation operands) both map to
the existing `SketchError::InvalidConfig`, consistent with how out-of-range
`lg_k`/`num_std_dev` are already reported elsewhere in this project.

## Concurrency

`ArrayOfDoublesSketch`, `CompactArrayOfDoublesSketch`,
`ArrayOfDoublesUnion`, `ArrayOfDoublesIntersection`, and
`ArrayOfDoublesAnotB` are all `unsafe impl Send`, not `Sync` — unchanged
convention from every prior family. No CPC-style global-init concurrency
hazard exists for this family.

## Repo layout additions

```
apache-datasketches-sys/
├── vendor/datasketches-cpp/tuple/include/   copied from the root submodule
│                                             (array_tuple_sketch.hpp,
│                                              array_tuple_union.hpp,
│                                              array_tuple_intersection.hpp,
│                                              array_tuple_a_not_b.hpp,
│                                              array_of_doubles_sketch.hpp,
│                                              tuple_sketch.hpp,
│                                              tuple_union.hpp,
│                                              tuple_intersection.hpp,
│                                              tuple_a_not_b.hpp,
│                                              tuple_jaccard_similarity.hpp,
│                                              and their *_impl.hpp files)
├── cpp/tuple/
│   ├── array_of_doubles_sketch_shim.h/.cc     ArrayOfDoublesSketchShim + builder
│   ├── array_of_doubles_compact_shim.h/.cc    CompactArrayOfDoublesSketchShim
│   ├── array_of_doubles_union_shim.h/.cc      ArrayOfDoublesUnionShim + builder
│   ├── array_of_doubles_intersection_shim.h/.cc
│   ├── array_of_doubles_a_not_b_shim.h/.cc
│   └── array_of_doubles_jaccard_shim.h/.cc    free jaccard function, assembled
│                                               from jaccard_similarity_base
└── src/
    ├── array_of_doubles_sketch.rs
    ├── array_of_doubles_compact.rs
    ├── array_of_doubles_union.rs
    ├── array_of_doubles_intersection.rs
    ├── array_of_doubles_a_not_b.rs
    └── array_of_doubles_jaccard.rs

apache-datasketches/src/tuple/
├── mod.rs
├── builder.rs        ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder
├── sketch.rs         ArrayOfDoublesSketch
├── compact.rs        CompactArrayOfDoublesSketch
├── union.rs          ArrayOfDoublesUnion
├── intersection.rs   ArrayOfDoublesIntersection
├── a_not_b.rs         ArrayOfDoublesAnotB
└── jaccard.rs         array_of_doubles_jaccard_similarity(), JaccardBounds

apache-datasketches/examples/tuple.rs
```

New Cargo feature: `tuple`, additive with `hll`/`theta`/`cpc`, still
`default = []`. This feature name is chosen (not e.g. `array-of-doubles`)
because a future generic-summary framework, if built, is intended to
**share this same feature** rather than introduce a separate one — both
are the same upstream "Tuple sketch" family, and the callback machinery
would be a fixed, one-time compilation cost rather than something worth
gating separately per summary type.

## Testing

1:1-ported (same header-comment-linking-to-upstream convention as every
prior family's test files):
- `array_of_doubles_sketch_test.rs` ← `array_of_doubles_sketch_test.cpp`
  (the single upstream test file for this family — sketch, union,
  intersection, and a-not-b are all exercised from this one file upstream,
  unlike Theta's separate per-class files).

Additional Rust-specific tests (beyond the `Send` verification,
`SketchError` conversion, and feature-flag compilation checks already
established as standard for every family): explicit tests for the two
family-specific validation requirements this design introduces —
`update()` with a mismatched-length `values` slice returns
`InvalidConfig`, and union/intersection/a-not-b with mismatched
`num_values` operands returns `InvalidConfig` — since neither has an
upstream C++ exception to delegate to and both are safety-critical (not
just ergonomic).

## Future generic framework (explicitly out of scope here)

A later, separate design will cover a `TupleSummary` trait allowing
arbitrary Rust-defined per-entry summary types, backed by a type-erased
`DynSummary` C++ wrapper whose `create`/`update`/`combine`/serialize
methods forward to a boxed Rust trait object via `extern "C"` trampolines
(the standard `cxx`-documented pattern for trait-object-like behavior,
since `cxx` does not support `dyn Trait` as an opaque type directly — the
concrete wrapper struct holding a `Box<dyn TupleSummary>` internally must
cross the boundary instead). That design will additionally need to resolve
where the `TupleSummary` trait itself lives, given `cxx` requires
`extern "Rust"` opaque types to be defined in the same crate as the bridge
declaring them: a minimal `RawSummaryOps` trait in
`apache-datasketches-sys` (satisfying that constraint) with the ergonomic
`TupleSummary` trait and a blanket adapter living in `apache-datasketches`,
keeping the sys crate's public surface as minimal/unstable as established
convention. `ArrayOfDoublesSketch` and any future generic
`TupleSketch<S: TupleSummary>` are intended to coexist permanently as
separate, purpose-built types (not unified into one type with hidden
internal dispatch) — `ArrayOfDoublesSketch` remains the zero-callback-
overhead fast path for the common numeric-array case, while
`TupleSketch<S>` handles arbitrary custom summaries at the cost of
per-operation FFI callback overhead. None of this design's code is
expected to be reused by that future work; what it does provide is a
fully de-risked understanding of the family's builder/seed/set-operation
plumbing.

## Out of scope for this design

- The fully generic `tuple_sketch<Summary, Update, Policy, Allocator>` and
  any callback/type-erasure framework — see above.
- Additional intersection combine policies beyond sum (min/max, etc.) —
  deferred as a future non-breaking addition.
- Cross-family a-not-b/jaccard (e.g. an `ArrayOfDoublesSketch` against a
  plain `ThetaSketch`) — upstream's generic `compute()`/similarity
  functions technically allow mixed operand types, but this design keeps
  every family self-contained, consistent with how HLL/Theta/CPC don't
  interoperate with each other either.
- Exposing a custom seed parameter — default-seed-only, same as every
  prior family.
- Sketch families other than HLL, Theta, CPC, and this one (KLL, REQ,
  t-Digest, Frequent Items, Count-Min, Sampling, Bloom Filters, Density) —
  each remains its own follow-up design/implementation cycle.
- CI/CD, prebuilt binaries — unchanged from prior designs.
