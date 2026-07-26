pub mod error;

#[cfg(feature = "hll")]
pub mod hll;

#[cfg(feature = "theta")]
pub mod theta;

pub use error::SketchError;
