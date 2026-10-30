//! The single error type shared across every sketch family in this crate.

use thiserror::Error;

/// Error type shared across all sketch families (HLL, Theta, CPC, Tuple,
/// KLL, REQ, Bloom) in this crate — there is no per-family error type.
///
/// Marked `#[non_exhaustive]` so a new family can add a variant without a
/// breaking release: a `match` on it needs a wildcard arm.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SketchError {
    /// A builder or constructor was given an out-of-range configuration
    /// value (e.g. `lg_k`/`lg_config_k` outside its valid bounds, or a
    /// `num_std_dev` outside `1..=3`). The underlying C++ layer rejected the
    /// value; the string is its exception message. Also returned when a set
    /// operation is given incompatible operands (e.g. Bloom `union_with` or
    /// `intersect` of filters with different sizes, hash counts or seeds).
    #[error("invalid sketch configuration: {0}")]
    InvalidConfig(String),

    /// `deserialize`/`deserialize_compressed`/`wrap` was given bytes that
    /// don't parse as a valid serialized sketch (e.g. truncated, corrupt,
    /// or produced by an incompatible seed, or a Bloom `wrap` of misaligned
    /// bytes).
    #[error("failed to deserialize sketch: {0}")]
    Deserialization(String),

    /// A catch-all for any other C++ exception that crossed the FFI
    /// boundary, carrying its `what()` message. Prefer matching on a more
    /// specific variant above when one applies.
    #[error("datasketches C++ error: {0}")]
    Cpp(String),

    /// `ThetaIntersection::get_result()` was called before any `update()`.
    #[error("intersection has no result: no update() call has been made yet")]
    EmptyIntersection,

    /// A query that needs data (a quantile, rank, PMF/CDF, or a
    /// Kolmogorov–Smirnov test) was made against an empty sketch.
    #[error("operation is undefined for an empty sketch")]
    EmptySketch,
}

impl From<cxx::Exception> for SketchError {
    fn from(e: cxx::Exception) -> Self {
        SketchError::Cpp(e.what().to_string())
    }
}
