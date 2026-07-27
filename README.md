# apache-datasketches-rs

Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp),
built via the `cxx` crate over a pinned git submodule.

`default = []` for both crates — no sketch family is compiled in unless
you opt in explicitly via Cargo features:

```toml
[dependencies]
apache-datasketches = { version = "0.1", features = ["hll", "theta"] }
```

## Crates

- `apache-datasketches-sys` — raw `cxx` bridge (do not use directly).
- `apache-datasketches` — safe, idiomatic Rust API.

## Building

This repo vendors `datasketches-cpp` as a git submodule and compiles it via
the `cxx` crate, so you need:

- A C++17 compiler on `PATH` (used by `cc`/`cxx-build`).
- The submodule checked out:

  ```bash
  git submodule update --init --recursive
  ```

Then build and test as usual:

```bash
cargo build --workspace
cargo test --workspace
```

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

## Examples

Standalone runnable demos live under `apache-datasketches/examples/`:

```bash
cargo run -p apache-datasketches --example hll --features hll
cargo run -p apache-datasketches --example theta --features theta
```

## Sketch families

`default = []` for both crates; enable one or both of the following
opt-in features:

- [x] `hll` (HyperLogLog) — cardinality estimation (sketch + union).
- [x] `theta` — cardinality estimation plus set operations: union,
  intersection, a-not-b, and Jaccard similarity.

## Vendored C++ version

`vendor/datasketches-cpp` is pinned to tag `5.2.0`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
