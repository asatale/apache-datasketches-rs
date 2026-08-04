//! Safe, idiomatic Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp),
//! built via the `cxx` crate over the raw
//! [`apache-datasketches-sys`](https://docs.rs/apache-datasketches-sys) bridge.
//!
//! `default = []` — no sketch family is compiled in unless you opt into its
//! Cargo feature explicitly:
//!
//! - `hll` (feature `hll`) — HyperLogLog cardinality estimation (sketch +
//!   union).
//! - `theta` (feature `theta`) — cardinality estimation plus set
//!   operations: union, intersection, a-not-b, and Jaccard similarity.
//! - `cpc` (feature `cpc`) — Compressed Probabilistic Counting
//!   cardinality estimation with a more compact serialized form (sketch +
//!   union only; no set operations beyond union).
//! - `tuple` (feature `tuple`) — ArrayOfDoubles Tuple sketches: cardinality
//!   estimation where each distinct key also carries a fixed-width array of
//!   `f64` values (summed on collision), plus the same set operations and
//!   Jaccard similarity as `theta`.
//!
//! (Module-level docs for each feature are only linked above when built
//! with that feature enabled — see `hll`/`theta`/`cpc`/`tuple` in the
//! sidebar, or build with `--all-features` to see all four at once.)
//!
//! See each module's documentation for usage examples, or the crate's
//! `examples/` directory for complete runnable demos.

#![warn(missing_docs)]

pub mod error;

#[cfg(feature = "hll")]
pub mod hll;

#[cfg(feature = "theta")]
pub mod theta;

#[cfg(feature = "cpc")]
pub mod cpc;

#[cfg(feature = "tuple")]
pub mod tuple;

pub use error::SketchError;
