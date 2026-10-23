//! REQ (Relative Error Quantiles) sketch family: approximate ranks,
//! quantiles, PMF and CDF over a stream of `f64` or `f32` values, with error
//! relative to the rank — accurate at the high ranks by default.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::req::{RankAccuracy, ReqSketch};
//!
//! let mut latencies = ReqSketch::<f64>::new(12, RankAccuracy::High)?;
//! for ms in 1..=100_000 {
//!     latencies.update(f64::from(ms));
//! }
//! let p999 = latencies.get_quantile(0.999, true)?;
//! println!("p99.9 latency: {p999} ms");
//! # Ok(())
//! # }
//! ```
//!
//! - [`ReqSketch`] — the sketch, generic over [`ReqItem`] (`f64` or `f32`).
//!   Merge sketches with [`ReqSketch::merge`], which fails if their
//!   [`RankAccuracy`] differs.
//! - [`ReqSortedView`] / [`ReqSortedViewEntry`] — an immutable sorted
//!   snapshot of a sketch, from [`ReqSketch::sorted_view`]; `Sync`, so it is
//!   how to share query access across threads.

mod checks;
mod item;
mod sketch;
mod sorted_view;

pub use item::ReqItem;
pub use sketch::{RankAccuracy, ReqSketch};
pub use sorted_view::{ReqSortedView, ReqSortedViewEntry};
