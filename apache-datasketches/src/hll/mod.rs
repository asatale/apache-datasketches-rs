//! HyperLogLog (HLL) cardinality estimation.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::hll::{HllSketch, TargetHllType};
//!
//! let mut sketch = HllSketch::new(12, TargetHllType::Hll4)?;
//! sketch.update_str("some-key");
//! sketch.update_u64(42);
//! println!("estimate: {}", sketch.get_estimate());
//! # Ok(())
//! # }
//! ```
//!
//! [`HllUnion`] merges multiple sketches into one, e.g. combining
//! per-shard/per-day counts into a total distinct count.

mod sketch;
mod union;

pub use sketch::{HllSketch, TargetHllType};
pub use union::HllUnion;
