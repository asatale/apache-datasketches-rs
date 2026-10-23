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
//! - [`KllSortedView`] / [`SortedViewEntry`] — an immutable sorted snapshot
//!   of a sketch, from [`KllSketch::sorted_view`]; `Sync`, so it is how to
//!   share query access across threads.
//! - [`kolmogorov_smirnov_test`] (plus [`kolmogorov_smirnov_delta`] and
//!   [`kolmogorov_smirnov_threshold`]) — whether two sketches look like
//!   samples of the same distribution.

mod checks;
mod item;
mod kolmogorov_smirnov;
mod sketch;
mod sorted_view;

pub use item::KllItem;
pub use kolmogorov_smirnov::{
    kolmogorov_smirnov_delta, kolmogorov_smirnov_test, kolmogorov_smirnov_threshold,
};
pub use sketch::KllSketch;
pub use sorted_view::{KllSortedView, SortedViewEntry};
