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
