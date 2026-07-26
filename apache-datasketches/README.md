# apache-datasketches

Safe, idiomatic Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp),
built via the `cxx` crate over
[`apache-datasketches-sys`](https://crates.io/crates/apache-datasketches-sys).

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

## Sketch families

- [x] HLL (HyperLogLog) — `hll` feature, enabled by default (sketch + union).

## License

Dual-licensed under MIT or Apache-2.0, at your option.
