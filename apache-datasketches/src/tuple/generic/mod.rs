//! Generic Tuple sketches: cardinality estimation where each distinct key
//! carries a summary of a type you define.
//!
//! Implement [`TupleSummary`] on your own type and it becomes usable as a
//! sketch summary. C++ calls back into Rust to clone and combine summaries;
//! see [`TupleSummary`]'s documentation for which methods must not panic.
//!
//! For the common case of a fixed-width array of `f64` per key, prefer
//! [`ArrayOfDoublesSketch`](crate::tuple::ArrayOfDoublesSketch) — it binds a
//! concrete C++ instantiation with no callback overhead.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};
//!
//! #[derive(Clone)]
//! struct Count(u64);
//!
//! impl TupleSummary for Count {
//!     type Update = ();
//!     fn create(_: &()) -> Self { Count(1) }
//!     fn union_combine(&mut self, other: &Self) { self.0 += other.0; }
//!     fn intersection_combine(&mut self, other: &Self) { self.0 += other.0; }
//! }
//!
//! let mut sketch: TupleSketch<Count> = TupleSketchBuilder::new().build()?;
//! sketch.update_u64(42, &());
//! println!("estimate: {}", sketch.get_estimate());
//! # Ok(())
//! # }
//! ```

mod builder;
mod compact;
mod sketch;
mod summary;

pub use builder::TupleSketchBuilder;
pub use compact::CompactTupleSketch;
pub use sketch::TupleSketch;
pub use summary::TupleSummary;
