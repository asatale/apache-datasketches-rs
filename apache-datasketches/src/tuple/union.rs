use super::input::ArrayOfDoublesInput;
use super::{CompactArrayOfDoublesSketch, ResizeFactor};
use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;
use apache_datasketches_sys::array_of_doubles_union::ffi as sys;
use cxx::UniquePtr;

/// Builder for [`ArrayOfDoublesUnion`], mirroring upstream's
/// `array_of_doubles_union::builder`. `lg_k` defaults to `12`,
/// `resize_factor` to [`ResizeFactor::X8`], `p` to `1.0` (no sampling), and
/// `num_values` to `1`. As with
/// [`ArrayOfDoublesSketchBuilder`](super::ArrayOfDoublesSketchBuilder), the
/// seed is never exposed.
#[derive(Debug, Clone, Copy)]
pub struct ArrayOfDoublesUnionBuilder {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
    num_values: u8,
}

impl Default for ArrayOfDoublesUnionBuilder {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::default(),
            p: 1.0,
            num_values: 1,
        }
    }
}

impl ArrayOfDoublesUnionBuilder {
    /// Creates a new builder with default settings (`lg_k = 12`,
    /// `resize_factor = X8`, `p = 1.0`, `num_values = 1`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the target number of retained entries in the
    /// union's result.
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Sets the hash table's growth [`ResizeFactor`].
    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    /// Sets the sampling probability. `1.0` (the default) disables sampling.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Sets the fixed number of `f64` values per entry. Must be at least `1`,
    /// and must match every sketch later passed to
    /// [`ArrayOfDoublesUnion::update`].
    pub fn num_values(mut self, num_values: u8) -> Self {
        self.num_values = num_values;
        self
    }

    /// Builds the union. Returns [`SketchError::InvalidConfig`] if `lg_k` is
    /// out of range, `p` is outside `(0, 1]`, or `num_values` is `0`.
    pub fn build(self) -> Result<ArrayOfDoublesUnion, SketchError> {
        if self.num_values == 0 {
            return Err(SketchError::InvalidConfig(
                "num_values must be at least 1".to_string(),
            ));
        }
        let inner = sys::new_array_of_doubles_union(
            self.lg_k,
            self.resize_factor.into(),
            self.p,
            self.num_values,
        )
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(ArrayOfDoublesUnion {
            inner,
            num_values: self.num_values,
        })
    }
}

/// A streaming union accumulator over ArrayOfDoubles sketches. Values are
/// summed per index when the same key appears in more than one input, using
/// upstream's `default_array_of_doubles_union_policy`.
///
/// Accepts either [`super::ArrayOfDoublesSketch`] or
/// [`CompactArrayOfDoublesSketch`] via the sealed [`ArrayOfDoublesInput`]
/// trait.
pub struct ArrayOfDoublesUnion {
    inner: UniquePtr<sys::ArrayOfDoublesUnionShim>,
    num_values: u8,
}

unsafe impl Send for ArrayOfDoublesUnion {}

impl ArrayOfDoublesUnion {
    /// Merges the given sketch into this union's running result.
    ///
    /// Returns [`SketchError::InvalidConfig`] if the sketch's `num_values`
    /// differs from this union's. Upstream does not validate this itself —
    /// merging mismatched array widths would read and write past the shorter
    /// array's bounds rather than error — so the check happens here, before
    /// the sketch crosses the FFI boundary.
    pub fn update(&mut self, input: &impl ArrayOfDoublesInput) -> Result<(), SketchError> {
        let actual = input.get_num_values();
        if actual != self.num_values {
            return Err(SketchError::InvalidConfig(format!(
                "num_values mismatch: union has {}, input has {actual}",
                self.num_values
            )));
        }
        match input.as_input() {
            ArrayOfDoublesInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            ArrayOfDoublesInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
        }
        Ok(())
    }

    /// Returns the union's current result as a
    /// [`CompactArrayOfDoublesSketch`]. If `ordered` is `true`, the result's
    /// entries are sorted by hash value.
    pub fn get_result(&self, ordered: bool) -> CompactArrayOfDoublesSketch {
        CompactArrayOfDoublesSketch::from_shim(self.inner.get_result(ordered))
    }

    /// Resets this union to its initial, empty state. `num_values` is
    /// preserved.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns the fixed number of `f64` values per entry this union was
    /// built with. Every input passed to [`Self::update`] must match it.
    pub fn get_num_values(&self) -> u8 {
        self.num_values
    }
}
