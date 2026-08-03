use super::input::ArrayOfDoublesInput;
use super::CompactArrayOfDoublesSketch;
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_a_not_b::ffi as sys;
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;
use cxx::UniquePtr;

/// Computes the set difference ("A not B": keys in `a` but not `b`) of two
/// ArrayOfDoubles sketches via [`Self::compute`]. Retained entries keep `a`'s
/// values unchanged. Stateless between calls — unlike
/// [`super::ArrayOfDoublesUnion`]/[`super::ArrayOfDoublesIntersection`], there
/// is no accumulation across repeated calls.
pub struct ArrayOfDoublesAnotB {
    inner: UniquePtr<sys::ArrayOfDoublesAnotBShim>,
}

unsafe impl Send for ArrayOfDoublesAnotB {}

impl Default for ArrayOfDoublesAnotB {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrayOfDoublesAnotB {
    /// Creates a new, reusable a-not-b calculator.
    pub fn new() -> Self {
        Self {
            inner: sys::new_array_of_doubles_a_not_b(),
        }
    }

    /// Computes the set difference `a - b` (keys in `a` that are not in `b`)
    /// as a [`CompactArrayOfDoublesSketch`]. `a` and `b` may independently be
    /// a [`super::ArrayOfDoublesSketch`] or a
    /// [`CompactArrayOfDoublesSketch`]. If `ordered` is `true`, the result's
    /// entries are sorted by hash value.
    ///
    /// Returns [`SketchError::InvalidConfig`] if `a` and `b` disagree on
    /// `num_values` — upstream does not validate this itself, and mismatched
    /// widths would read out of bounds.
    pub fn compute(
        &self,
        a: &impl ArrayOfDoublesInput,
        b: &impl ArrayOfDoublesInput,
        ordered: bool,
    ) -> Result<CompactArrayOfDoublesSketch, SketchError> {
        let (a_num, b_num) = (a.get_num_values(), b.get_num_values());
        if a_num != b_num {
            return Err(SketchError::InvalidConfig(format!(
                "num_values mismatch: a has {a_num}, b has {b_num}"
            )));
        }
        let inner = match (a.as_input(), b.as_input()) {
            (ArrayOfDoublesInputRef::Sketch(a), ArrayOfDoublesInputRef::Sketch(b)) => {
                self.inner.compute_sketch_sketch(a, b, ordered)
            }
            (ArrayOfDoublesInputRef::Sketch(a), ArrayOfDoublesInputRef::Compact(b)) => {
                self.inner.compute_sketch_compact(a, b, ordered)
            }
            (ArrayOfDoublesInputRef::Compact(a), ArrayOfDoublesInputRef::Sketch(b)) => {
                self.inner.compute_compact_sketch(a, b, ordered)
            }
            (ArrayOfDoublesInputRef::Compact(a), ArrayOfDoublesInputRef::Compact(b)) => {
                self.inner.compute_compact_compact(a, b, ordered)
            }
        };
        Ok(CompactArrayOfDoublesSketch::from_shim(inner))
    }
}
