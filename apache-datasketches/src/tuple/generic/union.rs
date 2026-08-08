use super::builder::resize_factor_multiplier;
use super::{CompactTupleSketch, TupleInput, TupleSummary};
use crate::error::SketchError;
use crate::tuple::ResizeFactor;
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;
use apache_datasketches_sys::tuple_generic_union::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// Builder for [`TupleUnion`]. `lg_k` defaults to `12`, `resize_factor` to
/// [`ResizeFactor::X8`], `p` to `1.0`.
pub struct TupleUnionBuilder<S: TupleSummary> {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
    _marker: PhantomData<fn() -> S>,
}

// Hand-written rather than `#[derive(..)]`, for the same reason as
// `TupleSketchBuilder` (see the note in builder.rs): a derive would add an
// `S: Debug`/`S: Clone`/`S: Copy` bound to each impl even though every field
// here is unconditionally `Debug + Clone + Copy`. `TupleSummary` requires
// neither `Debug` nor `Copy`, so deriving would make this builder silently
// non-`Debug` and non-`Copy` for most summaries -- and inconsistent with its
// sketch-side counterpart.
impl<S: TupleSummary> std::fmt::Debug for TupleUnionBuilder<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TupleUnionBuilder")
            .field("lg_k", &self.lg_k)
            .field("resize_factor", &self.resize_factor)
            .field("p", &self.p)
            .finish()
    }
}

impl<S: TupleSummary> Clone for TupleUnionBuilder<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: TupleSummary> Copy for TupleUnionBuilder<S> {}

impl<S: TupleSummary> Default for TupleUnionBuilder<S> {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::X8,
            p: 1.0,
            _marker: PhantomData,
        }
    }
}

impl<S: TupleSummary> TupleUnionBuilder<S> {
    /// Creates a builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the target number of retained entries.
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Sets the hash table's growth [`ResizeFactor`].
    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    /// Sets the sampling probability.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Builds the union. Returns [`SketchError::InvalidConfig`] if `lg_k` is
    /// out of range or `p` is outside `(0, 1]`.
    pub fn build(self) -> Result<TupleUnion<S>, SketchError> {
        let inner = sys::new_tuple_generic_union(
            self.lg_k,
            resize_factor_multiplier(self.resize_factor),
            self.p,
        )
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(TupleUnion {
            inner,
            _marker: PhantomData,
        })
    }
}

/// A streaming union over generic Tuple sketches. Summaries for a key present
/// in more than one input are merged with
/// [`TupleSummary::union_combine`](super::TupleSummary::union_combine).
pub struct TupleUnion<S: TupleSummary> {
    inner: UniquePtr<sys::TupleGenericUnionShim>,
    _marker: PhantomData<fn() -> S>,
}

// Sound because `S: Send` is a supertrait of `TupleSummary` and every summary
// the union owns is a `Box<dyn RawSummaryOps + Send>`. Deliberately not
// `Sync`, matching `TupleSketch<S>` and `CompactTupleSketch<S>`.
unsafe impl<S: TupleSummary> Send for TupleUnion<S> {}

impl<S: TupleSummary> TupleUnion<S> {
    /// Merges the given sketch into the running result.
    ///
    /// Infallible: unlike ArrayOfDoubles there is no `num_values` to agree
    /// on — the type system already guarantees both operands carry `S`.
    pub fn update(&mut self, input: &impl TupleInput<S>) {
        match input.as_input() {
            TupleGenericInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            TupleGenericInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
        }
    }

    /// Returns the union's current result. If `ordered` is `true`, entries
    /// are sorted by hash value.
    pub fn get_result(&self, ordered: bool) -> CompactTupleSketch<S> {
        CompactTupleSketch::from_shim(self.inner.get_result(ordered))
    }

    /// Resets this union to its initial, empty state.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }
}
