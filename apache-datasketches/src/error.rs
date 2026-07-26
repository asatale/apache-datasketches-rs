use thiserror::Error;

#[derive(Debug, Error)]
pub enum SketchError {
    #[error("invalid sketch configuration: {0}")]
    InvalidConfig(String),

    #[error("failed to deserialize sketch: {0}")]
    Deserialization(String),

    #[error("datasketches C++ error: {0}")]
    Cpp(String),
}

impl From<cxx::Exception> for SketchError {
    fn from(e: cxx::Exception) -> Self {
        SketchError::Cpp(e.what().to_string())
    }
}
