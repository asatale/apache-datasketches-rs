use super::{CompactThetaSketch, ThetaSketch, WrappedCompactThetaSketch};
use apache_datasketches_sys::theta_input::ThetaInputRef;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ThetaSketch {}
    impl Sealed for super::CompactThetaSketch {}
    impl<'a> Sealed for super::WrappedCompactThetaSketch<'a> {}
}

/// Any of the three theta sketch types can be fed into a `ThetaUnion`,
/// `ThetaIntersection`, `ThetaAnotB`, or `jaccard_similarity`. This trait
/// is sealed — it cannot be implemented outside this crate — since the
/// set-op shims only have concrete overloads for these three exact types.
pub trait ThetaInput: sealed::Sealed {
    #[doc(hidden)]
    fn as_theta_input(&self) -> ThetaInputRef<'_>;
}

impl ThetaInput for ThetaSketch {
    fn as_theta_input(&self) -> ThetaInputRef<'_> {
        ThetaInputRef::Sketch(&self.inner)
    }
}

impl ThetaInput for CompactThetaSketch {
    fn as_theta_input(&self) -> ThetaInputRef<'_> {
        ThetaInputRef::Compact(&self.inner)
    }
}

impl<'a> ThetaInput for WrappedCompactThetaSketch<'a> {
    fn as_theta_input(&self) -> ThetaInputRef<'_> {
        ThetaInputRef::Wrapped(&self.inner)
    }
}
