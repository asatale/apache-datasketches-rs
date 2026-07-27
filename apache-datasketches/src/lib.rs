pub mod error;

#[cfg(feature = "hll")]
pub mod hll;

#[cfg(feature = "theta")]
pub mod theta;

#[cfg(feature = "cpc")]
pub mod cpc;

pub use error::SketchError;
