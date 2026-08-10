use super::{CompactTupleSketch, TupleInput, TupleSummary};
use apache_datasketches_sys::tuple_generic_a_not_b::ffi as sys;
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// Computes the set difference `a - b` over generic Tuple sketches.
///
/// Retained entries keep `a`'s summaries unchanged: unlike
/// [`TupleUnion`](super::TupleUnion) and
/// [`TupleIntersection`](super::TupleIntersection), a-not-b has no combine
/// policy at all, so neither [`TupleSummary::union_combine`] nor
/// [`TupleSummary::intersection_combine`] is ever invoked. Each surviving
/// summary is cloned out of `a`, which leaves `a` itself untouched and usable
/// afterwards.
///
/// Stateless between calls, and asymmetric: `compute(a, b)` and
/// `compute(b, a)` are different operations.
///
/// No builder: upstream's type has a plain constructor, matching
/// [`ArrayOfDoublesAnotB`](crate::tuple::ArrayOfDoublesAnotB).
pub struct TupleAnotB<S: TupleSummary> {
    inner: UniquePtr<sys::TupleGenericAnotBShim>,
    _marker: PhantomData<fn() -> S>,
}

// Sound because `S: Send` is a supertrait of `TupleSummary` and every summary
// that crosses through this calculator is a `Box<dyn RawSummaryOps + Send>`.
// Deliberately not `Sync`, matching `TupleSketch<S>`, `CompactTupleSketch<S>`,
// `TupleUnion<S>` and `TupleIntersection<S>`.
unsafe impl<S: TupleSummary> Send for TupleAnotB<S> {}

impl<S: TupleSummary> Default for TupleAnotB<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: TupleSummary> TupleAnotB<S> {
    /// Creates a reusable a-not-b calculator.
    pub fn new() -> Self {
        Self {
            inner: sys::new_tuple_generic_a_not_b(),
            _marker: PhantomData,
        }
    }

    /// Computes `a - b`: keys in `a` that are not in `b`, carrying `a`'s
    /// summaries. If `ordered` is `true`, the result's entries are sorted by
    /// hash value.
    ///
    /// Infallible: upstream throws only on a seed-hash mismatch, and this
    /// family never exposes a seed — every sketch here is built with the
    /// default one.
    pub fn compute(
        &self,
        a: &impl TupleInput<S>,
        b: &impl TupleInput<S>,
        ordered: bool,
    ) -> CompactTupleSketch<S> {
        let inner = match (a.as_input(), b.as_input()) {
            (TupleGenericInputRef::Sketch(a), TupleGenericInputRef::Sketch(b)) => {
                self.inner.compute_sketch_sketch(a, b, ordered)
            }
            (TupleGenericInputRef::Sketch(a), TupleGenericInputRef::Compact(b)) => {
                self.inner.compute_sketch_compact(a, b, ordered)
            }
            (TupleGenericInputRef::Compact(a), TupleGenericInputRef::Sketch(b)) => {
                self.inner.compute_compact_sketch(a, b, ordered)
            }
            (TupleGenericInputRef::Compact(a), TupleGenericInputRef::Compact(b)) => {
                self.inner.compute_compact_compact(a, b, ordered)
            }
        };
        CompactTupleSketch::from_shim(inner)
    }
}
