use super::{CompactTupleSketch, TupleSketch, TupleSummary};
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;

mod sealed {
    use super::TupleSummary;
    pub trait Sealed {}
    impl<S: TupleSummary> Sealed for super::TupleSketch<S> {}
    impl<S: TupleSummary> Sealed for super::CompactTupleSketch<S> {}
}

/// Either generic Tuple sketch type can be fed into this module's set
/// operations. Sealed — the shims have concrete overloads for these two types
/// only.
pub trait TupleInput<S: TupleSummary>: sealed::Sealed {
    #[doc(hidden)]
    fn as_input(&self) -> TupleGenericInputRef<'_>;
}

impl<S: TupleSummary> TupleInput<S> for TupleSketch<S> {
    fn as_input(&self) -> TupleGenericInputRef<'_> {
        TupleGenericInputRef::Sketch(&self.inner)
    }
}

impl<S: TupleSummary> TupleInput<S> for CompactTupleSketch<S> {
    fn as_input(&self) -> TupleGenericInputRef<'_> {
        TupleGenericInputRef::Compact(&self.inner)
    }
}
