use thiserror::Error;

#[derive(Debug, Error)]
pub enum SketchError {
    #[error("invalid sketch configuration: {0}")]
    InvalidConfig(String),

    #[error("failed to deserialize sketch: {0}")]
    Deserialization(String),

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
