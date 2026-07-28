//! Theta sketch family: cardinality estimation plus set operations (union,
//! intersection, a-not-b) and Jaccard similarity.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::theta::ThetaSketchBuilder;
//!
//! let mut sketch = ThetaSketchBuilder::new().lg_k(12).build()?;
//! sketch.update_u64(42);
//! println!("estimate: {}", sketch.get_estimate());
//! # Ok(())
//! # }
//! ```
//!
//! - [`ThetaSketch`] / [`ThetaSketchBuilder`] — the updatable sketch; build
//!   with `ThetaSketchBuilder::new().lg_k(..).resize_factor(..).p(..).build()`.
//! - [`CompactThetaSketch`] — an immutable, serializable snapshot produced
//!   by `ThetaSketch::compact`, `ThetaUnion::get_result`, or
//!   `ThetaIntersection::get_result`.
//! - [`WrappedCompactThetaSketch`] — a zero-copy, read-only view over a
//!   serialized compact sketch's bytes, built with
//!   `WrappedCompactThetaSketch::wrap`.
//! - [`ThetaUnion`] / [`ThetaUnionBuilder`] — merges multiple sketches.
//! - [`ThetaIntersection`] — computes the intersection of sketches fed via
//!   `update`.
//! - [`ThetaAnotB`] — computes the set difference (items in `a` but not
//!   `b`).
//! - [`jaccard_similarity`] / [`JaccardBounds`] — estimates the Jaccard
//!   index (intersection-over-union) of two sketches.
//!
//! [`ThetaSketch`], [`CompactThetaSketch`], and [`WrappedCompactThetaSketch`]
//! can all be passed interchangeably (via the sealed [`ThetaInput`] trait)
//! to `ThetaUnion::update`, `ThetaIntersection::update`, `ThetaAnotB::compute`,
//! and [`jaccard_similarity`].

mod a_not_b;
mod builder;
mod compact;
mod input;
mod intersection;
mod jaccard;
mod sketch;
mod union;
mod wrapped;

pub use a_not_b::ThetaAnotB;
pub use builder::{ResizeFactor, ThetaSketchBuilder};
pub use compact::CompactThetaSketch;
pub use input::ThetaInput;
pub use intersection::ThetaIntersection;
pub use jaccard::{jaccard_similarity, JaccardBounds};
pub use sketch::ThetaSketch;
pub use union::{ThetaUnion, ThetaUnionBuilder};
pub use wrapped::WrappedCompactThetaSketch;
