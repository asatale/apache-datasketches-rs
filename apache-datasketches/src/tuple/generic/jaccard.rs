use super::{TupleInput, TupleSummary};
use crate::tuple::JaccardBounds;
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;
use apache_datasketches_sys::tuple_generic_jaccard::ffi as sys;

impl From<sys::TupleGenericJaccardBoundsFfi> for JaccardBounds {
    fn from(ffi: sys::TupleGenericJaccardBoundsFfi) -> Self {
        Self {
            lower_bound: ffi.lower_bound,
            estimate: ffi.estimate,
            upper_bound: ffi.upper_bound,
        }
    }
}

/// Estimates the Jaccard index (intersection-over-union) of two generic
/// Tuple sketches.
///
/// Only the keys affect the result — per-entry summaries do not, and no
/// summary callback influences the returned bounds.
pub fn tuple_jaccard_similarity<S: TupleSummary>(
    a: &impl TupleInput<S>,
    b: &impl TupleInput<S>,
) -> JaccardBounds {
    let ffi = match (a.as_input(), b.as_input()) {
        (TupleGenericInputRef::Sketch(a), TupleGenericInputRef::Sketch(b)) => {
            sys::tuple_generic_jaccard_sketch_sketch(a, b)
        }
        (TupleGenericInputRef::Sketch(a), TupleGenericInputRef::Compact(b)) => {
            sys::tuple_generic_jaccard_sketch_compact(a, b)
        }
        (TupleGenericInputRef::Compact(a), TupleGenericInputRef::Sketch(b)) => {
            sys::tuple_generic_jaccard_compact_sketch(a, b)
        }
        (TupleGenericInputRef::Compact(a), TupleGenericInputRef::Compact(b)) => {
            sys::tuple_generic_jaccard_compact_compact(a, b)
        }
    };
    ffi.into()
}
