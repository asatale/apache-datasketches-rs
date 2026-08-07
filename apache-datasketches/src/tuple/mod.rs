//! ArrayOfDoubles Tuple sketch family: cardinality estimation where each
//! retained key also carries a fixed-width array of `f64` values, summed on
//! collision.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::tuple::ArrayOfDoublesSketchBuilder;
//!
//! let mut sketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build()?;
//! sketch.update_u64(42, &[1.0, 2.5])?;
//! println!("estimate: {}", sketch.get_estimate());
//! # Ok(())
//! # }
//! ```
//!
//! - [`ArrayOfDoublesSketch`] / [`ArrayOfDoublesSketchBuilder`] — the
//!   updatable sketch.
//! - [`CompactArrayOfDoublesSketch`] — an immutable, serializable snapshot
//!   produced by `ArrayOfDoublesSketch::compact` or by a set operation's
//!   result.
//! - [`ArrayOfDoublesUnion`] / [`ArrayOfDoublesUnionBuilder`] — merges
//!   multiple sketches, summing values per index on collision.
//! - [`ArrayOfDoublesIntersection`] — computes the intersection of sketches
//!   fed via `update`, summing values per index.
//! - [`ArrayOfDoublesAnotB`] — computes the set difference (keys in `a` but
//!   not `b`), preserving `a`'s values.
//! - [`array_of_doubles_jaccard_similarity`] / [`JaccardBounds`] — estimates
//!   the Jaccard index (intersection-over-union) of two sketches.
//! - [`generic`] — Tuple sketches over a summary type you define yourself,
//!   for cases the fixed `f64`-array shape above does not cover.
//!
//! [`ArrayOfDoublesSketch`] and [`CompactArrayOfDoublesSketch`] can both be
//! passed interchangeably (via the sealed [`ArrayOfDoublesInput`] trait) to
//! every set operation in this module.

mod a_not_b;
mod builder;
mod compact;
pub mod generic;
mod input;
mod intersection;
mod jaccard;
mod sketch;
mod union;

pub use a_not_b::ArrayOfDoublesAnotB;
pub use builder::{ArrayOfDoublesSketchBuilder, ResizeFactor};
pub use compact::CompactArrayOfDoublesSketch;
pub use input::ArrayOfDoublesInput;
pub use intersection::ArrayOfDoublesIntersection;
pub use jaccard::{array_of_doubles_jaccard_similarity, JaccardBounds};
pub use sketch::ArrayOfDoublesSketch;
pub use union::{ArrayOfDoublesUnion, ArrayOfDoublesUnionBuilder};
