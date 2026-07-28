//! The single error type shared across every sketch family in this crate.

use thiserror::Error;

/// Error type shared across all sketch families (HLL, Theta, CPC) in this
/// crate — there is no per-family error type.
#[derive(Debug, Error)]
pub enum SketchError {
    /// A builder or constructor was given an out-of-range configuration
    /// value (e.g. `lg_k`/`lg_config_k` outside its valid bounds, or a
    /// `num_std_dev` outside `1..=3`). The underlying C++ layer rejected the
    /// value; the string is its exception message.
    #[error("invalid sketch configuration: {0}")]
    InvalidConfig(String),

    /// `deserialize`/`deserialize_compressed`/`wrap` was given bytes that
    /// don't parse as a valid serialized sketch (e.g. truncated, corrupt,
    /// or produced by an incompatible seed).
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
}

impl From<cxx::Exception> for SketchError {
    fn from(e: cxx::Exception) -> Self {
        SketchError::Cpp(e.what().to_string())
    }
}
