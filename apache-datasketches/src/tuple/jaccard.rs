use super::input::ArrayOfDoublesInput;
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;
use apache_datasketches_sys::array_of_doubles_jaccard::ffi as sys;

/// The result of [`array_of_doubles_jaccard_similarity`]: a confidence
/// interval around the estimated Jaccard index of two ArrayOfDoubles
/// sketches, in `[0.0, 1.0]`.
///
/// This is a distinct type from the theta module's `JaccardBounds` with the
/// same shape — the two sketch families are independently feature-gated and
/// do not share types. (Deliberately not an intra-doc link: `theta` may not
/// be compiled in.)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JaccardBounds {
    /// Lower bound of the confidence interval around [`Self::estimate`].
    pub lower_bound: f64,
    /// The estimated Jaccard index.
    pub estimate: f64,
    /// Upper bound of the confidence interval around [`Self::estimate`].
    pub upper_bound: f64,
}

impl From<sys::TupleJaccardBoundsFfi> for JaccardBounds {
    fn from(ffi: sys::TupleJaccardBoundsFfi) -> Self {
        Self {
            lower_bound: ffi.lower_bound,
            estimate: ffi.estimate,
            upper_bound: ffi.upper_bound,
        }
    }
}

/// Estimates the Jaccard index (intersection-over-union) of two
/// ArrayOfDoubles sketches, each of which may independently be a
/// [`super::ArrayOfDoublesSketch`] or a
/// [`super::CompactArrayOfDoublesSketch`].
///
/// Only the keys matter — the per-entry values do not affect the result.
///
/// Returns [`SketchError::InvalidConfig`] if the two sketches disagree on
/// `num_values`, for consistency with the other set operations (the
/// underlying computation would tolerate a mismatch, but accepting one here
/// would let a genuine modelling error pass silently).
pub fn array_of_doubles_jaccard_similarity(
    a: &impl ArrayOfDoublesInput,
    b: &impl ArrayOfDoublesInput,
) -> Result<JaccardBounds, SketchError> {
    let (a_num, b_num) = (a.get_num_values(), b.get_num_values());
    if a_num != b_num {
        return Err(SketchError::InvalidConfig(format!(
            "num_values mismatch: a has {a_num}, b has {b_num}"
        )));
    }
    let ffi = match (a.as_input(), b.as_input()) {
        (ArrayOfDoublesInputRef::Sketch(a), ArrayOfDoublesInputRef::Sketch(b)) => {
            sys::tuple_jaccard_sketch_sketch(a, b)
        }
        (ArrayOfDoublesInputRef::Sketch(a), ArrayOfDoublesInputRef::Compact(b)) => {
            sys::tuple_jaccard_sketch_compact(a, b)
        }
        (ArrayOfDoublesInputRef::Compact(a), ArrayOfDoublesInputRef::Sketch(b)) => {
            sys::tuple_jaccard_compact_sketch(a, b)
        }
        (ArrayOfDoublesInputRef::Compact(a), ArrayOfDoublesInputRef::Compact(b)) => {
            sys::tuple_jaccard_compact_compact(a, b)
        }
    };
    Ok(ffi.into())
}
