use super::{summary::TupleSummary, TupleSketch};
use crate::error::SketchError;
use crate::tuple::ResizeFactor;
use std::marker::PhantomData;

/// Builder for [`TupleSketch`], mirroring upstream's
/// `update_tuple_sketch::builder`. `lg_k` defaults to `12`, `resize_factor`
/// to [`ResizeFactor::X8`], and `p` to `1.0` (no sampling). The seed is never
/// exposed.
#[derive(Debug, Clone, Copy)]
pub struct TupleSketchBuilder<S: TupleSummary> {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
    _marker: PhantomData<fn() -> S>,
}

impl<S: TupleSummary> Default for TupleSketchBuilder<S> {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::X8,
            p: 1.0,
            _marker: PhantomData,
        }
    }
}

impl<S: TupleSummary> TupleSketchBuilder<S> {
    /// Creates a builder with default settings (`lg_k = 12`,
    /// `resize_factor = X8`, `p = 1.0`).
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

    /// Sets the sampling probability. `1.0` (the default) disables sampling.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Builds the sketch. Returns [`SketchError::InvalidConfig`] if `lg_k` is
    /// out of range or `p` is outside `(0, 1]`.
    pub fn build(self) -> Result<TupleSketch<S>, SketchError> {
        TupleSketch::from_parts(self.lg_k, self.resize_factor, self.p)
    }
}

/// Converts the safe enum to the literal multiplier this bridge passes as a
/// `u8`. See the task note on why this bridge does not share a cxx enum.
pub(crate) fn resize_factor_multiplier(rf: ResizeFactor) -> u8 {
    match rf {
        ResizeFactor::X1 => 1,
        ResizeFactor::X2 => 2,
        ResizeFactor::X4 => 4,
        ResizeFactor::X8 => 8,
    }
}
