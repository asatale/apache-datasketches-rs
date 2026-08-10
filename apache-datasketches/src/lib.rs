//! Safe, idiomatic Rust bindings for [Apache DataSketches](https://github.com/apache/datasketches-cpp),
//! built via the `cxx` crate over the raw
//! [`apache-datasketches-sys`](https://docs.rs/apache-datasketches-sys) bridge.
//!
//! All four sketch families are enabled by default. To compile only the ones
//! you need, disable default features and name them:
//!
//! ```toml
//! apache-datasketches = { version = "0.2", default-features = false, features = ["hll"] }
//! ```
//!
//! Unused families cost nothing at runtime — the linker drops what you do not
//! call — so opting out buys C++ compile time, not a smaller binary.
//!
//! The families:
//!
//! - `hll` (feature `hll`) — HyperLogLog cardinality estimation (sketch +
//!   union).
//! - `theta` (feature `theta`) — cardinality estimation plus set
//!   operations: union, intersection, a-not-b, and Jaccard similarity.
//! - `cpc` (feature `cpc`) — Compressed Probabilistic Counting
//!   cardinality estimation with a more compact serialized form (sketch +
//!   union only; no set operations beyond union).
//! - `tuple` (feature `tuple`) — Tuple sketches, in two shapes. The
//!   ArrayOfDoubles form carries a fixed-width array of `f64` per distinct
//!   key (summed on collision); the generic form in `tuple::generic` carries
//!   a summary type you define in Rust. Both support union, intersection,
//!   a-not-b, and Jaccard similarity.
//!
//! (Module-level docs for each feature are only linked above when built
//! with that feature enabled — see `hll`/`theta`/`cpc`/`tuple` in the
//! sidebar.)
//!
//! See each module's documentation for usage examples, or the crate's
//! `examples/` directory for complete runnable demos.

#![warn(missing_docs)]

// Disabling default features without naming a family leaves nothing behind
// but `SketchError`. That used to compile silently; say so instead.
#[cfg(not(any(feature = "hll", feature = "theta", feature = "cpc", feature = "tuple")))]
compile_error!(
    "apache-datasketches: no sketch family is enabled, so this crate exposes nothing. \
     Enable at least one of the `hll`, `theta`, `cpc`, or `tuple` features, or drop \
     `default-features = false`."
);

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
