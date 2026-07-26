# apache-rust-sketch-wrapper

Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp),
built via the `cxx` crate over a pinned git submodule.

## Crates

- `apache-datasketches-sys` — raw `cxx` bridge (do not use directly).
- `apache-datasketches` — safe, idiomatic Rust API.

## Sketch families

- [x] HLL (HyperLogLog) — `hll` feature, enabled by default.

## Vendored C++ version

`vendor/datasketches-cpp` is pinned to tag `5.2.0`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
