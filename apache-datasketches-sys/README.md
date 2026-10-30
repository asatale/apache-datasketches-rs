# apache-datasketches-sys

Raw `cxx` bridge to the [Apache DataSketches](https://github.com/apache/datasketches-cpp)
C++ library. **Do not use this crate directly** — its API is an unstable,
low-level FFI surface that can change without a semver bump to the safe
layer's public API.

Use [`apache-datasketches`](https://crates.io/crates/apache-datasketches)
instead, which wraps this crate in a safe, idiomatic Rust API.

## Building

Requires a C++17 compiler on `PATH` (used by `cc`/`cxx-build`). The
`datasketches-cpp` headers this crate compiles against are vendored in
`vendor/datasketches-cpp` — see `vendor/README.md` for how that copy is
kept in sync with the upstream project.

## Features

`default = []` — no sketch family is compiled in unless you opt in
explicitly. Enable one or more of:

- `hll` — raw bridge to the HLL (HyperLogLog) sketch and union C++ types.
- `theta` — raw bridge to the Theta sketch, union, intersection, a-not-b,
  and Jaccard similarity C++ types.
- `cpc` — raw bridge to the CPC (Compressed Probabilistic Counting) sketch
  and union C++ types.
- `tuple` — raw bridge to the Tuple (ArrayOfDoubles) sketch, union,
  intersection, a-not-b, and Jaccard similarity C++ types.
- `kll` — raw bridge to the KLL quantiles sketch and its sorted view C++
  types, for `double` and `float` items, plus the Kolmogorov–Smirnov test.
- `req` — raw bridge to the REQ quantiles sketch and its sorted view C++
  types, for `double` and `float` items.
- `bloom` — raw bridge to the Bloom filter and its zero-copy wrapped view C++
  types.

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["hll"] }
```

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["theta"] }
```

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["cpc"] }
```

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["tuple"] }
```

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["kll"] }
```

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["req"] }
```

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["bloom"] }
```

```toml
[dependencies]
apache-datasketches-sys = { version = "0.2", features = ["hll", "theta", "cpc", "tuple", "kll", "req", "bloom"] }
```
