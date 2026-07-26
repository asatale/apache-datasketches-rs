use super::input::ThetaInput;
use apache_datasketches_sys::theta_input::ThetaInputRef;
use apache_datasketches_sys::theta_jaccard::ffi as sys;

/// The result of [`jaccard_similarity`]: a confidence interval around the
/// estimated Jaccard index of two theta sketches, in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JaccardBounds {
    pub lower_bound: f64,
    pub estimate: f64,
    pub upper_bound: f64,
}

impl From<sys::JaccardBoundsFfi> for JaccardBounds {
    fn from(ffi: sys::JaccardBoundsFfi) -> Self {
        Self {
            lower_bound: ffi.lower_bound,
            estimate: ffi.estimate,
            upper_bound: ffi.upper_bound,
        }
    }
}

/// Estimates the Jaccard index (intersection-over-union) of two theta
/// sketches, each of which may independently be a [`super::ThetaSketch`],
/// [`super::CompactThetaSketch`], or [`super::WrappedCompactThetaSketch`].
pub fn jaccard_similarity(a: &impl ThetaInput, b: &impl ThetaInput) -> JaccardBounds {
    let ffi = match (a.as_theta_input(), b.as_theta_input()) {
        (ThetaInputRef::Sketch(a), ThetaInputRef::Sketch(b)) => sys::jaccard_sketch_sketch(a, b),
        (ThetaInputRef::Sketch(a), ThetaInputRef::Compact(b)) => sys::jaccard_sketch_compact(a, b),
        (ThetaInputRef::Sketch(a), ThetaInputRef::Wrapped(b)) => sys::jaccard_sketch_wrapped(a, b),
        (ThetaInputRef::Compact(a), ThetaInputRef::Sketch(b)) => sys::jaccard_compact_sketch(a, b),
        (ThetaInputRef::Compact(a), ThetaInputRef::Compact(b)) => sys::jaccard_compact_compact(a, b),
        (ThetaInputRef::Compact(a), ThetaInputRef::Wrapped(b)) => sys::jaccard_compact_wrapped(a, b),
        (ThetaInputRef::Wrapped(a), ThetaInputRef::Sketch(b)) => sys::jaccard_wrapped_sketch(a, b),
        (ThetaInputRef::Wrapped(a), ThetaInputRef::Compact(b)) => sys::jaccard_wrapped_compact(a, b),
        (ThetaInputRef::Wrapped(a), ThetaInputRef::Wrapped(b)) => sys::jaccard_wrapped_wrapped(a, b),
    };
    ffi.into()
}
