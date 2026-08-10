# apache-datasketches

Safe, idiomatic Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp),
built via the `cxx` crate over
[`apache-datasketches-sys`](https://crates.io/crates/apache-datasketches-sys).

All four sketch families — `hll`, `theta`, `cpc` and `tuple` — are enabled by
default:

```toml
[dependencies]
apache-datasketches = "0.2"
```

To compile only what you need, disable default features and name the families:

```toml
[dependencies]
apache-datasketches = { version = "0.2", default-features = false, features = ["hll"] }
```

Unused families cost nothing at runtime — the linker drops what you do not
call — so opting out buys C++ compile time, not a smaller binary. Cold debug
builds of the FFI layer run about 4.5s with no families and about 20s with all
four; `theta` and `tuple` account for nearly all of the difference.

Disabling default features without naming at least one family is a compile
error rather than a crate that silently exposes nothing.

> **Feature defaults changed in 0.2.1:** 0.2.0 shipped `default = []`, which
> compiled cleanly and exposed nothing. 0.2.1 enables all four families by
> default. This only adds APIs, so upgrading cannot break existing code; if you
> were relying on the minimal build, add `default-features = false` and list the
> families you want.

## Usage

```rust
use apache_datasketches::hll::{HllSketch, TargetHllType};

let mut sketch = HllSketch::new(12, TargetHllType::Hll4)?;
sketch.update_str("some-key");
sketch.update_u64(42);
println!("estimate: {}", sketch.get_estimate());
```

Merging multiple sketches with `HllUnion`:

```rust
use apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType};

let mut sketch1 = HllSketch::new(12, TargetHllType::Hll4)?;
sketch1.update_u64(1);

let mut sketch2 = HllSketch::new(12, TargetHllType::Hll4)?;
sketch2.update_u64(2);

let mut union = HllUnion::new(12)?;
union.update_sketch(&sketch1);
union.update_sketch(&sketch2);

let result = union.get_result(TargetHllType::Hll4);
println!("union estimate: {}", result.get_estimate());
```

`HllSketch` supports `serialize_compact`/`serialize_updatable` and
`HllSketch::deserialize` for persisting sketches. `HllUnion` has no
serializable state of its own upstream (only its result sketch does),
so `HllUnion::serialize_compact`/`serialize_updatable` serialize
`get_result(tgt_type)` directly — to resume accumulating after
deserializing, feed the deserialized `HllSketch` back into a new
`HllUnion` via `update_sketch`.

See `examples/hll.rs` (`cargo run -p apache-datasketches --example hll`)
for a complete runnable demo.

## Theta sketches

The Theta sketch family (`theta` feature) supports cardinality estimation
like HLL, plus set operations — union, intersection, and a-not-b — and
Jaccard similarity between sketches.

```rust
use apache_datasketches::theta::ThetaSketchBuilder;

let mut sketch = ThetaSketchBuilder::new().lg_k(12).build()?;
sketch.update_u64(42);
println!("estimate: {}", sketch.get_estimate());
```

- `ThetaSketch` / `ThetaSketchBuilder` — the updatable sketch; build with
  `ThetaSketchBuilder::new().lg_k(..).resize_factor(..).p(..).build()`.
- `CompactThetaSketch` — an immutable, serializable snapshot produced by
  `ThetaSketch::compact`, `ThetaUnion::get_result`, or
  `ThetaIntersection::get_result`; supports `serialize_compact`/
  `serialize_compressed` and `CompactThetaSketch::deserialize`/
  `deserialize_compressed`.
- `WrappedCompactThetaSketch` — a zero-copy, read-only view over a
  serialized compact sketch's bytes, built with `WrappedCompactThetaSketch::wrap`.
- `ThetaUnion` / `ThetaUnionBuilder` — merges multiple sketches; build with
  `ThetaUnionBuilder::new().lg_k(..).build()`, feed sketches via `update`,
  and read the merged estimate via `get_result(ordered)`.
- `ThetaIntersection` — computes the intersection of sketches fed via
  `update`; `get_result(ordered)` returns `Err` if no sketch has been
  provided yet.
- `ThetaAnotB` — computes the set difference (items in `a` but not `b`)
  via `ThetaAnotB::new().compute(a, b, ordered)`.
- `jaccard_similarity`/`JaccardBounds` — estimates the Jaccard index
  (intersection-over-union) of two sketches, returning a
  `{ lower_bound, estimate, upper_bound }` confidence interval.

`ThetaSketch`, `CompactThetaSketch`, and `WrappedCompactThetaSketch` can
all be passed interchangeably to `ThetaUnion::update`,
`ThetaIntersection::update`, `ThetaAnotB::compute`, and
`jaccard_similarity`.

See `examples/theta.rs` (`cargo run -p apache-datasketches --example theta
--features theta`) for a complete runnable demo covering cardinality
estimation, union, intersection, a-not-b, Jaccard similarity, and
serialize/deserialize round-tripping.

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
  safe under concurrent access (C++11 magic-static guarantees), so this
  isn't a correctness fix; it's a latency optimization that moves the
  one-time table-building cost off the hot path and avoids threads
  stalling behind whichever one wins the lazy-init race. Single-threaded
  callers never need to call this.

See `examples/cpc.rs` (`cargo run -p apache-datasketches --example cpc
--features cpc`) for a complete runnable demo.

## Tuple sketches

The Tuple (ArrayOfDoubles) sketch family (`tuple` feature) supports
cardinality estimation like HLL/Theta/CPC, but each distinct key also
carries a fixed-width array of `f64` values, summed on collision — plus
set operations (union, intersection, a-not-b) and Jaccard similarity.

```rust
use apache_datasketches::tuple::ArrayOfDoublesSketchBuilder;

let mut sketch = ArrayOfDoublesSketchBuilder::new()
    .lg_k(12)
    .num_values(2)
    .build()?;
sketch.update_u64(42, &[1.0, 2.50])?;
println!("estimate: {}", sketch.get_estimate());
```

- `ArrayOfDoublesSketch` / `ArrayOfDoublesSketchBuilder` — the updatable
  sketch; build with `ArrayOfDoublesSketchBuilder::new().lg_k(..)
  .resize_factor(..).p(..).num_values(..).build()`. Supports the full
  upstream `update` overload set (`update_u64`/`update_i64`/`update_u32`/
  `update_i32`/`update_u16`/`update_i16`/`update_u8`/`update_i8`/
  `update_f64`/`update_str`/`update_bytes`), each taking a key plus the
  entry's values, plus `trim`/`reset`, `get_num_values`, `entries`, and
  `compact(ordered)`.
- `CompactArrayOfDoublesSketch` — an immutable, serializable snapshot
  produced by `ArrayOfDoublesSketch::compact` or by any set operation's
  result; supports `serialize`/`CompactArrayOfDoublesSketch::deserialize`.
  Unlike Theta there is one serialization format — upstream has no
  compressed variant for this family.
- `ArrayOfDoublesUnion` / `ArrayOfDoublesUnionBuilder` — merges multiple
  sketches, summing values per index on collision; build with
  `ArrayOfDoublesUnionBuilder::new().lg_k(..).num_values(..).build()`, feed
  sketches via `update`, and read the result via `get_result(ordered)`.
- `ArrayOfDoublesIntersection` — computes the intersection of sketches fed
  via `update`, summing values per index. Built with
  `ArrayOfDoublesIntersection::new(num_values)` rather than a builder;
  `get_result(ordered)` returns `Err` if no sketch has been provided yet.
- `ArrayOfDoublesAnotB` — computes the set difference (keys in `a` but not
  `b`, preserving `a`'s values) via
  `ArrayOfDoublesAnotB::new().compute(a, b, ordered)`.
- `array_of_doubles_jaccard_similarity`/`JaccardBounds` — estimates the
  Jaccard index (intersection-over-union) of two sketches, returning a
  `{ lower_bound, estimate, upper_bound }` confidence interval. Only the
  keys affect the result; per-entry values do not.

`ArrayOfDoublesSketch` and `CompactArrayOfDoublesSketch` can both be passed
interchangeably (via the sealed `ArrayOfDoublesInput` trait) to
`ArrayOfDoublesUnion::update`, `ArrayOfDoublesIntersection::update`,
`ArrayOfDoublesAnotB::compute`, and `array_of_doubles_jaccard_similarity`.

Two things differ from the other families. `update` returns `Result`
because `values.len()` must equal `num_values` — upstream indexes the
supplied array without a bounds check of its own, so this is validated in
Rust before crossing the FFI boundary. For the same reason, every set
operation requires its operands to agree on `num_values` and returns
`SketchError::InvalidConfig` if they don't.

Repeated updates to the same key sum their values per index, as does
merging a key present in more than one input. Read them back per entry via
`entries()`, scaling by `1.0 / get_theta()` to estimate population totals.

See `examples/tuple.rs` (`cargo run -p apache-datasketches --example tuple
--features tuple`) for a complete runnable demo covering cardinality
estimation, per-entry values, union, intersection, a-not-b, Jaccard
similarity, and serialize/deserialize round-tripping.

### Generic summaries

When a fixed array of `f64` is the wrong shape, implement `TupleSummary` on
your own type and use `tuple::generic::TupleSketch<S>`:

```rust
use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};

#[derive(Clone)]
struct Count(u64);

impl TupleSummary for Count {
    type Update = ();
    fn create(_: &()) -> Self { Count(1) }
    fn union_combine(&mut self, other: &Self) { self.0 += other.0; }
    fn intersection_combine(&mut self, other: &Self) { self.0 += other.0; }
}

let mut sketch: TupleSketch<Count> = TupleSketchBuilder::new().build()?;
sketch.update_u64(42, &());
```

`TupleUnion<S>`, `TupleIntersection<S>`, `TupleAnotB<S>`, and
`tuple_jaccard_similarity` mirror their ArrayOfDoubles counterparts, and
`CompactTupleSketch<S>::entries()` yields `(hash, S)` pairs.

C++ calls back into Rust to clone and combine summaries. A panic in
`union_combine`, `intersection_combine`, or `Clone::clone` aborts the process
— panics cannot cross the FFI boundary — so make those total. A panic in
`create` is an ordinary Rust panic, because it runs Rust-side before anything
crosses the boundary. The callbacks are not confined to the obvious places:
`tuple_jaccard_similarity` clones essentially every retained summary and runs
both combine callbacks on scratch copies, even though summary values do not
affect the ratio it returns. A-not-b is the one operation that invokes neither
combine callback — its C++ template takes no policy at all — but it still clones
each summary it copies out of `a`, and leaves `a` intact.
Serialization of generic sketches is not supported; use
`ArrayOfDoublesSketch` if you need to persist a sketch.

See `examples/tuple_generic.rs` (`cargo run -p apache-datasketches --example
tuple_generic --features tuple`) for a complete runnable demo.

## Sketch families

- [x] HLL (HyperLogLog) — `hll` feature (sketch + union).
- [x] Theta — `theta` feature (sketch, union, intersection, a-not-b,
  Jaccard similarity).
- [x] CPC (Compressed Probabilistic Counting) — `cpc` feature (sketch +
  union).
- [x] Tuple — `tuple` feature (per-key summaries: fixed-width `f64` arrays
  summed on collision (ArrayOfDoubles), or a summary type you define in Rust
  (`tuple::generic`); union, intersection, a-not-b, Jaccard similarity).

## License

Dual-licensed under MIT or Apache-2.0, at your option.
