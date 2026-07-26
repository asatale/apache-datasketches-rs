use super::input::ThetaInput;
use super::{CompactThetaSketch, ResizeFactor};
use crate::error::SketchError;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use apache_datasketches_sys::theta_union::ffi as sys;
use cxx::UniquePtr;

/// Builder for [`ThetaUnion`], mirroring upstream's `theta_union::builder`.
/// `lg_k` defaults to `12`, `resize_factor` to [`ResizeFactor::X8`], `p` to
/// `1.0` (no sampling). As with [`super::ThetaSketchBuilder`], the seed is
/// never exposed.
pub struct ThetaUnionBuilder {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
}

impl Default for ThetaUnionBuilder {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::default(),
            p: 1.0,
        }
    }
}

impl ThetaUnionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    pub fn build(self) -> Result<ThetaUnion, SketchError> {
        let inner = sys::new_theta_union(self.lg_k, self.resize_factor.into(), self.p)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(ThetaUnion { inner })
    }
}

/// A streaming union accumulator over theta sketches. Accepts any of
/// [`super::ThetaSketch`], [`CompactThetaSketch`], or
/// [`super::WrappedCompactThetaSketch`] via the sealed [`ThetaInput`] trait.
pub struct ThetaUnion {
    inner: UniquePtr<sys::ThetaUnionShim>,
}

unsafe impl Send for ThetaUnion {}

impl ThetaUnion {
    pub fn update(&mut self, input: &impl ThetaInput) {
        match input.as_theta_input() {
            ThetaInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ThetaInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
            ThetaInputRef::Wrapped(w) => self.inner.pin_mut().update_with_wrapped(w),
        }
    }

    pub fn get_result(&self, ordered: bool) -> CompactThetaSketch {
        CompactThetaSketch::from_shim(self.inner.get_result(ordered))
    }

    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }
}
