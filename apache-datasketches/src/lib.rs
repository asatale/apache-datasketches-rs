pub mod error;

#[cfg(feature = "hll")]
pub mod hll;

pub use error::SketchError;
