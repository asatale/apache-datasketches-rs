use super::{CompactTupleSketch, TupleInput, TupleSummary};
use crate::error::SketchError;
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;
use apache_datasketches_sys::tuple_generic_intersection::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// Computes the intersection of generic Tuple sketches fed via
/// [`Self::update`]. Summaries of keys present in every input are merged with
/// [`TupleSummary::intersection_combine`](super::TupleSummary::intersection_combine);
/// a key present in only some of the inputs is dropped and its summary never
/// reaches the callback.
///
/// No builder: upstream's type has a plain constructor, matching
/// [`ArrayOfDoublesIntersection`](crate::tuple::ArrayOfDoublesIntersection).
///
/// A fresh intersection is the infinite "universe", not the empty set — see
/// [`Self::get_result`].
pub struct TupleIntersection<S: TupleSummary> {
    inner: UniquePtr<sys::TupleGenericIntersectionShim>,
    _marker: PhantomData<fn() -> S>,
}

// Sound because `S: Send` is a supertrait of `TupleSummary` and every summary
// the intersection owns is a `Box<dyn RawSummaryOps + Send>`. Deliberately not
// `Sync`, matching `TupleSketch<S>`, `CompactTupleSketch<S>` and
// `TupleUnion<S>`.
unsafe impl<S: TupleSummary> Send for TupleIntersection<S> {}

impl<S: TupleSummary> Default for TupleIntersection<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: TupleSummary> TupleIntersection<S> {
    /// Creates an intersection with no result yet — call [`Self::update`] at
    /// least once before [`Self::get_result`].
    pub fn new() -> Self {
        Self {
            inner: sys::new_tuple_generic_intersection(),
            _marker: PhantomData,
        }
    }

    /// Narrows the running result to also require membership in `input`.
    ///
    /// Infallible: unlike ArrayOfDoubles there is no `num_values` to agree
    /// on — the type system already guarantees both operands carry `S`.
    pub fn update(&mut self, input: &impl TupleInput<S>) {
        match input.as_input() {
            TupleGenericInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            TupleGenericInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
        }
    }

    /// Returns the current result, or [`SketchError::EmptyIntersection`] if
    /// [`Self::update`] has never been called. If `ordered` is `true`,
    /// entries are sorted by hash value.
    ///
    /// The no-operand case is an error rather than an empty sketch because
    /// upstream defines it as the infinite "universe" and throws
    /// (`theta_intersection_base::get_result`). That is a genuinely different
    /// state from an intersection of disjoint operands, which succeeds and
    /// returns an empty sketch.
    pub fn get_result(&self, ordered: bool) -> Result<CompactTupleSketch<S>, SketchError> {
        if !self.inner.has_result() {
            return Err(SketchError::EmptyIntersection);
        }
        let inner = self
            .inner
            .get_result(ordered)
            .map_err(|e| SketchError::Cpp(e.what().to_string()))?;
        Ok(CompactTupleSketch::from_shim(inner))
    }

    /// Returns `true` if [`Self::update`] has been called at least once.
    pub fn has_result(&self) -> bool {
        self.inner.has_result()
    }
}
