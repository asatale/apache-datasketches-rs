use super::input::ArrayOfDoublesInput;
use super::CompactArrayOfDoublesSketch;
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;
use apache_datasketches_sys::array_of_doubles_intersection::ffi as sys;
use cxx::UniquePtr;

/// Computes the intersection of ArrayOfDoubles sketches fed via
/// [`Self::update`]. Values are summed per index for keys present in every
/// input.
///
/// Unlike [`super::ArrayOfDoublesUnion`] there is no builder — upstream's
/// `array_of_doubles_intersection` has a plain constructor, and the
/// intersecting universe is defined entirely by the sketches passed to
/// `update`. Only `num_values` (which the combine policy needs at
/// construction time) must be supplied up front.
///
/// Upstream ships no default combine policy for this type; v1 uses
/// sum-on-collision, mirroring the union's policy. Additional policies
/// (min/max, etc.) can be added later without changing this type's shape.
pub struct ArrayOfDoublesIntersection {
    inner: UniquePtr<sys::ArrayOfDoublesIntersectionShim>,
    num_values: u8,
}

unsafe impl Send for ArrayOfDoublesIntersection {}

impl ArrayOfDoublesIntersection {
    /// Creates a new intersection accumulator for sketches carrying
    /// `num_values` values per entry, with no result yet — call
    /// [`Self::update`] at least once before [`Self::get_result`].
    ///
    /// Returns [`SketchError::InvalidConfig`] if `num_values` is `0`.
    pub fn new(num_values: u8) -> Result<Self, SketchError> {
        if num_values == 0 {
            return Err(SketchError::InvalidConfig(
                "num_values must be at least 1".to_string(),
            ));
        }
        Ok(Self {
            inner: sys::new_array_of_doubles_intersection(num_values),
            num_values,
        })
    }

    /// Narrows this intersection's running result to also require membership
    /// in the given sketch. The first call establishes the initial universe;
    /// each subsequent call intersects further.
    ///
    /// Returns [`SketchError::InvalidConfig`] if the sketch's `num_values`
    /// differs from this intersection's — upstream does not validate this
    /// itself, and mismatched widths would read and write out of bounds.
    pub fn update(&mut self, input: &impl ArrayOfDoublesInput) -> Result<(), SketchError> {
        let actual = input.get_num_values();
        if actual != self.num_values {
            return Err(SketchError::InvalidConfig(format!(
                "num_values mismatch: intersection has {}, input has {actual}",
                self.num_values
            )));
        }
        match input.as_input() {
            ArrayOfDoublesInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ArrayOfDoublesInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
        }
        Ok(())
    }

    /// Returns the current intersection result as a
    /// [`CompactArrayOfDoublesSketch`], or
    /// [`SketchError::EmptyIntersection`] if [`Self::update`] has never been
    /// called. If `ordered` is `true`, the result's entries are sorted by
    /// hash value.
    pub fn get_result(&self, ordered: bool) -> Result<CompactArrayOfDoublesSketch, SketchError> {
        if !self.inner.has_result() {
            return Err(SketchError::EmptyIntersection);
        }
        let inner = self
            .inner
            .get_result(ordered)
            .map_err(|e| SketchError::Cpp(e.what().to_string()))?;
        Ok(CompactArrayOfDoublesSketch::from_shim(inner))
    }

    /// Returns `true` if [`Self::update`] has been called at least once.
    pub fn has_result(&self) -> bool {
        self.inner.has_result()
    }

    /// Returns the fixed number of `f64` values per entry this intersection
    /// was created with. Every input passed to [`Self::update`] must match it.
    pub fn get_num_values(&self) -> u8 {
        self.num_values
    }
}
