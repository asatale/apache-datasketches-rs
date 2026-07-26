use super::input::ThetaInput;
use super::CompactThetaSketch;
use crate::error::SketchError;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use apache_datasketches_sys::theta_intersection::ffi as sys;
use cxx::UniquePtr;

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
    pub fn new() -> Self {
        Self {
            inner: sys::new_theta_intersection(),
        }
    }

    pub fn update(&mut self, input: &impl ThetaInput) {
        match input.as_theta_input() {
            ThetaInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ThetaInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
            ThetaInputRef::Wrapped(w) => self.inner.pin_mut().update_with_wrapped(w),
        }
    }

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

    pub fn has_result(&self) -> bool {
        self.inner.has_result()
    }
}
