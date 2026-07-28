use super::input::ThetaInput;
use super::CompactThetaSketch;
use crate::error::SketchError;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use apache_datasketches_sys::theta_intersection::ffi as sys;
use cxx::UniquePtr;

/// Computes the intersection of theta sketches fed via [`Self::update`].
/// Unlike [`super::ThetaUnion`]/[`super::ThetaAnotB`], intersection has no
/// builder — the intersecting universe is defined entirely by the sketches
/// passed to `update`, matching upstream's plain-constructor
/// `theta_intersection`.
pub struct ThetaIntersection {
    inner: UniquePtr<sys::ThetaIntersectionShim>,
}

unsafe impl Send for ThetaIntersection {}

impl Default for ThetaIntersection {
    fn default() -> Self {
        Self::new()
    }
}

impl ThetaIntersection {
    /// Creates a new intersection accumulator with no result yet — call
    /// [`Self::update`] at least once before [`Self::get_result`].
    pub fn new() -> Self {
        Self {
            inner: sys::new_theta_intersection(),
        }
    }

    /// Narrows this intersection's running result to also require
    /// membership in the given sketch. The first call establishes the
    /// initial universe; each subsequent call intersects further.
    pub fn update(&mut self, input: &impl ThetaInput) {
        match input.as_theta_input() {
            ThetaInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ThetaInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
            ThetaInputRef::Wrapped(w) => self.inner.pin_mut().update_with_wrapped(w),
        }
    }

    /// Returns the current intersection result as a [`CompactThetaSketch`],
    /// or [`SketchError::EmptyIntersection`] if [`Self::update`] has never
    /// been called. If `ordered` is `true`, the result's entries are
    /// sorted by hash value.
    pub fn get_result(&self, ordered: bool) -> Result<CompactThetaSketch, SketchError> {
        if !self.inner.has_result() {
            return Err(SketchError::EmptyIntersection);
        }
        let inner = self
            .inner
            .get_result(ordered)
            .map_err(|e| SketchError::Cpp(e.what().to_string()))?;
        Ok(CompactThetaSketch::from_shim(inner))
    }

    /// Returns `true` if [`Self::update`] has been called at least once.
    pub fn has_result(&self) -> bool {
        self.inner.has_result()
    }
}
