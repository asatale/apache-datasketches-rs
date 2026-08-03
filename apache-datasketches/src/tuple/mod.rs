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

mod builder;
mod compact;
mod sketch;

pub use builder::{ArrayOfDoublesSketchBuilder, ResizeFactor};
pub use compact::CompactArrayOfDoublesSketch;
pub use sketch::ArrayOfDoublesSketch;
