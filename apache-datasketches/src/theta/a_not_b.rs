use super::input::ThetaInput;
use super::CompactThetaSketch;
use apache_datasketches_sys::theta_a_not_b::ffi as sys;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use cxx::UniquePtr;

/// Computes the set difference ("A not B": items in `a` but not `b`) of two
/// theta sketches via [`Self::compute`]. Stateless between calls — unlike
/// [`super::ThetaUnion`]/[`super::ThetaIntersection`], there is no
/// accumulation across repeated calls.
pub struct ThetaAnotB {
    inner: UniquePtr<sys::ThetaAnotBShim>,
}

unsafe impl Send for ThetaAnotB {}

impl Default for ThetaAnotB {
    fn default() -> Self {
        Self::new()
    }
}

impl ThetaAnotB {
    /// Creates a new, reusable a-not-b calculator.
    pub fn new() -> Self {
        Self {
            inner: sys::new_theta_a_not_b(),
        }
    }

    /// Computes the set difference `a - b` (items in `a` that are not in
    /// `b`) as a [`CompactThetaSketch`]. `a` and `b` may independently be
    /// any of [`super::ThetaSketch`], [`CompactThetaSketch`], or
    /// [`super::WrappedCompactThetaSketch`]. If `ordered` is `true`, the
    /// result's entries are sorted by hash value.
    pub fn compute(
        &self,
        a: &impl ThetaInput,
        b: &impl ThetaInput,
        ordered: bool,
    ) -> CompactThetaSketch {
        let inner = match (a.as_theta_input(), b.as_theta_input()) {
            (ThetaInputRef::Sketch(a), ThetaInputRef::Sketch(b)) => {
                self.inner.compute_sketch_sketch(a, b, ordered)
            }
            (ThetaInputRef::Sketch(a), ThetaInputRef::Compact(b)) => {
                self.inner.compute_sketch_compact(a, b, ordered)
            }
            (ThetaInputRef::Sketch(a), ThetaInputRef::Wrapped(b)) => {
                self.inner.compute_sketch_wrapped(a, b, ordered)
            }
            (ThetaInputRef::Compact(a), ThetaInputRef::Sketch(b)) => {
                self.inner.compute_compact_sketch(a, b, ordered)
            }
            (ThetaInputRef::Compact(a), ThetaInputRef::Compact(b)) => {
                self.inner.compute_compact_compact(a, b, ordered)
            }
            (ThetaInputRef::Compact(a), ThetaInputRef::Wrapped(b)) => {
                self.inner.compute_compact_wrapped(a, b, ordered)
            }
            (ThetaInputRef::Wrapped(a), ThetaInputRef::Sketch(b)) => {
                self.inner.compute_wrapped_sketch(a, b, ordered)
            }
            (ThetaInputRef::Wrapped(a), ThetaInputRef::Compact(b)) => {
                self.inner.compute_wrapped_compact(a, b, ordered)
            }
            (ThetaInputRef::Wrapped(a), ThetaInputRef::Wrapped(b)) => {
                self.inner.compute_wrapped_wrapped(a, b, ordered)
            }
        };
        CompactThetaSketch::from_shim(inner)
    }
}
