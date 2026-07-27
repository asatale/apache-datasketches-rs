# apache-datasketches

Safe, idiomatic Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp),
built via the `cxx` crate over
[`apache-datasketches-sys`](https://crates.io/crates/apache-datasketches-sys).

`default = []` — no sketch family is enabled unless you opt in explicitly.
Add the `hll` and/or `theta` feature to your `Cargo.toml`:

```toml
[dependencies]
apache-datasketches = { version = "0.2", features = ["hll"] }
```

```toml
[dependencies]
apache-datasketches = { version = "0.2", features = ["theta"] }
```

```toml
[dependencies]
apache-datasketches = { version = "0.2", features = ["hll", "theta"] }
```

> **Breaking change in 0.2.0:** prior versions defaulted to `features = ["hll"]`.
> As of 0.2.0, `default = []` — existing consumers upgrading from 0.1.x must
> add `features = ["hll"]` explicitly to keep HLL support enabled.

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

## Sketch families

- [x] HLL (HyperLogLog) — `hll` feature (sketch + union).
- [x] Theta — `theta` feature (sketch, union, intersection, a-not-b,
  Jaccard similarity).

## License

Dual-licensed under MIT or Apache-2.0, at your option.
