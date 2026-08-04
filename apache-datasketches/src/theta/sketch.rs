use crate::error::SketchError;
use crate::theta::builder::ResizeFactor;
use apache_datasketches_sys::theta_sketch::ffi as sys;
use cxx::UniquePtr;

/// A mutable, update-only theta sketch: estimates the number of distinct
/// items added via `update_*`. Build one with
/// [`ThetaSketchBuilder`](super::ThetaSketchBuilder).
///
/// Call [`Self::compact`] to produce an immutable, serializable
/// [`super::CompactThetaSketch`] snapshot for storage, transmission, or use
/// as input to a set operation.
pub struct ThetaSketch {
    pub(crate) inner: UniquePtr<sys::ThetaSketchShim>,
}

unsafe impl Send for ThetaSketch {}

impl ThetaSketch {
    pub(crate) fn from_parts(lg_k: u8, rf: ResizeFactor, p: f32) -> Result<Self, SketchError> {
        let inner = sys::new_theta_sketch(lg_k, rf.into(), p)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Adds a `u64` value to the sketch.
    pub fn update_u64(&mut self, value: u64) {
        self.inner.pin_mut().update_u64(value);
    }

    /// Adds an `i64` value to the sketch.
    pub fn update_i64(&mut self, value: i64) {
        self.inner.pin_mut().update_i64(value);
    }

    /// Adds a `u32` value to the sketch.
    pub fn update_u32(&mut self, value: u32) {
        self.inner.pin_mut().update_u32(value);
    }

    /// Adds an `i32` value to the sketch.
    pub fn update_i32(&mut self, value: i32) {
        self.inner.pin_mut().update_i32(value);
    }

    /// Adds a `u16` value to the sketch.
    pub fn update_u16(&mut self, value: u16) {
        self.inner.pin_mut().update_u16(value);
    }

    /// Adds an `i16` value to the sketch.
    pub fn update_i16(&mut self, value: i16) {
        self.inner.pin_mut().update_i16(value);
    }

    /// Adds a `u8` value to the sketch.
    pub fn update_u8(&mut self, value: u8) {
        self.inner.pin_mut().update_u8(value);
    }

    /// Adds an `i8` value to the sketch.
    pub fn update_i8(&mut self, value: i8) {
        self.inner.pin_mut().update_i8(value);
    }

    /// Adds an `f64` value to the sketch.
    pub fn update_f64(&mut self, value: f64) {
        self.inner.pin_mut().update_f64(value);
    }

    /// Adds a string value to the sketch.
    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    /// Adds an arbitrary byte slice to the sketch.
    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

    /// Removes retained entries in excess of the nominal size `k`, lowering
    /// the theta threshold to do so.
    ///
    /// Note that this *does* shift [`Self::get_estimate`] — trimming lowers
    /// theta, and the estimate is derived from the retained count and theta
    /// together. Upstream only guarantees the excess entries are dropped.
    pub fn trim(&mut self) {
        self.inner.pin_mut().trim();
    }

    /// Resets this sketch to its initial, empty state.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns the current estimate of the number of distinct items added.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`], for the given number of standard deviations
    /// (`1`, `2`, or `3`, corresponding to roughly 67%, 95%, and 99%
    /// confidence). Returns [`SketchError::InvalidConfig`] for any other
    /// value.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See [`Self::get_lower_bound`] for the meaning
    /// of `num_std_dev`.
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if no items have been added to this sketch.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if this sketch's theta threshold is below `1.0`
    /// (i.e. it has begun sampling and [`Self::get_estimate`] is a
    /// statistical estimate rather than an exact count).
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if this sketch's retained entries are sorted by hash
    /// value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold (`1.0` until sampling begins).
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of entries currently retained by this sketch.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }

    /// Produces an immutable, serializable [`super::CompactThetaSketch`]
    /// snapshot of this sketch's current state. If `ordered` is `true`,
    /// the snapshot's entries are sorted by hash value.
    pub fn compact(&self, ordered: bool) -> super::CompactThetaSketch {
        super::CompactThetaSketch::from_shim(self.inner.compact(ordered))
    }
}
