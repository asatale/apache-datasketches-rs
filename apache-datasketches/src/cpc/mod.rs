//! CPC (Compressed Probabilistic Counting) sketch family: cardinality
//! estimation with a more compact serialized form than HLL or Theta.
//!
//! Unlike the `theta` module, CPC has no set operations beyond union — no
//! intersection, a-not-b, or Jaccard similarity.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::cpc::CpcSketchBuilder;
//!
//! let mut sketch = CpcSketchBuilder::new().lg_k(11).build()?;
//! sketch.update_u64(42);
//! println!("estimate: {}", sketch.get_estimate());
//! # Ok(())
//! # }
//! ```
//!
//! - [`CpcSketch`] / [`CpcSketchBuilder`] — the sketch; build with
//!   `CpcSketchBuilder::new().lg_k(..).build()`.
//! - [`CpcUnion`] / [`CpcUnionBuilder`] — merges multiple sketches.
//! - [`get_max_serialized_size_bytes`] — the estimated maximum compressed
//!   serialized size, in bytes, for a given `lg_k`.
//! - [`init`] — eagerly initializes CPC's global decompression tables, as
//!   a one-time latency optimization; see its own doc comment for details.

mod builder;
mod init;
mod sketch;
mod union;

pub use builder::{CpcSketchBuilder, CpcUnionBuilder};
pub use init::init;
pub use sketch::{get_max_serialized_size_bytes, CpcSketch};
pub use union::CpcUnion;
