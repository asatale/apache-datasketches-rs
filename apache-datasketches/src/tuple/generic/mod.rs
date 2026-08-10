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
//! for key in 0..100u64 {
//!     sketch.update_u64(key, &());
//! }
//! // Updating a key that is already present combines the new summary into
//! // the retained one with `union_combine`, so key 42 ends up at 2.
//! sketch.update_u64(42, &());
//! println!("estimate: {}", sketch.get_estimate());
//!
//! // `compact` freezes the sketch into an immutable snapshot; `entries`
//! // hands back each retained `(hash, summary)` pair, with the summary
//! // cloned back out of C++ as an owned `Count`.
//! let compact = sketch.compact(true);
//! assert_eq!(compact.get_num_retained(), 100);
//! assert!(compact.is_ordered());
//!
//! let entries: Vec<(u64, Count)> = compact.entries().collect();
//! assert_eq!(entries.len(), 100);
//! // 99 keys were seen once and key 42 was seen twice.
//! assert_eq!(entries.iter().map(|(_, c)| c.0).sum::<u64>(), 101);
//! # Ok(())
//! # }
//! ```

mod a_not_b;
mod builder;
mod compact;
mod input;
mod intersection;
mod jaccard;
mod sketch;
mod summary;
mod union;

pub use a_not_b::TupleAnotB;
pub use builder::TupleSketchBuilder;
pub use compact::CompactTupleSketch;
pub use input::TupleInput;
pub use intersection::TupleIntersection;
pub use jaccard::tuple_jaccard_similarity;
pub use sketch::TupleSketch;
pub use summary::TupleSummary;
pub use union::{TupleUnion, TupleUnionBuilder};
