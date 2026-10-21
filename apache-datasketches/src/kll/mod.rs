//! KLL quantiles sketch family: approximate ranks, quantiles, PMF and CDF
//! over a stream of `f64` or `f32` values, in bounded memory.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::kll::KllSketch;
//!
//! let mut latencies = KllSketch::<f64>::new(200)?;
//! for ms in 1..=1000 {
//!     latencies.update(f64::from(ms));
//! }
//! let p99 = latencies.get_quantile(0.99, true)?;
//! println!("p99 latency: {p99} ms");
//! # Ok(())
//! # }
//! ```
//!
//! - [`KllSketch`] — the sketch, generic over [`KllItem`] (`f64` or `f32`).
//!   Merge sketches with [`KllSketch::merge`].

mod checks;
mod item;
mod sketch;

pub use item::KllItem;
pub use sketch::KllSketch;
