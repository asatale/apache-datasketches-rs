use crate::error::SketchError;
use apache_datasketches_sys::cpc_sketch::ffi as sys;
use cxx::UniquePtr;

/// A CPC (Compressed Probabilistic Counting) sketch: estimates the number
/// of distinct items added via `update_*`. Unlike Theta's `ThetaSketch`,
/// there is no separate compact/wrapped variant — this single type is
/// both the mutable/update type and the serializable type, since CPC's
/// serialized form is always compressed by construction. Build one with
/// [`CpcSketchBuilder`](super::CpcSketchBuilder).
pub struct CpcSketch {
    pub(crate) inner: UniquePtr<sys::CpcSketchShim>,
}

unsafe impl Send for CpcSketch {}

impl CpcSketch {
    pub(crate) fn from_lg_k(lg_k: u8) -> Result<Self, SketchError> {
        let inner = sys::new_cpc_sketch(lg_k)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Reconstructs a sketch from bytes produced by [`Self::serialize`].
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::cpc_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
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

    /// Adds an `f32` value to the sketch.
    pub fn update_f32(&mut self, value: f32) {
        self.inner.pin_mut().update_f32(value);
    }

    /// Adds a string value to the sketch.
    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    /// Adds an arbitrary byte slice to the sketch.
    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

    /// Returns `true` if no items have been added to this sketch.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the current estimate of the number of distinct items added.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`], for the given number of standard deviations
    /// (`1`, `2`, or `3`, corresponding to roughly 67%, 95%, and 99%
    /// confidence — upstream calls this parameter `kappa`). Returns
    /// [`SketchError::InvalidConfig`] for any other value.
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

    /// Returns the `lg_k` this sketch was built with.
    pub fn get_lg_k(&self) -> u8 {
        self.inner.get_lg_k()
    }

    /// Returns a human-readable, multi-line summary of this sketch's
    /// internal state — useful for debugging, not for parsing.
    pub fn to_string_summary(&self) -> String {
        self.inner.to_string_summary()
    }

    /// Serializes this sketch to bytes. CPC's on-wire format is always
    /// compressed, so there is only this one serialization method (unlike
    /// Theta's separate compressed/uncompressed formats).
    pub fn serialize(&self) -> Vec<u8> {
        self.inner.serialize().as_slice().to_vec()
    }
}

/// The estimated maximum compressed serialized size, in bytes, of a CPC
/// sketch built with the given `lg_k`. Useful for pre-allocating buffers.
///
/// Returns an error if `lg_k` is outside the valid range (`4..=26`).
pub fn get_max_serialized_size_bytes(lg_k: u8) -> Result<usize, SketchError> {
    sys::cpc_sketch_max_serialized_size_bytes(lg_k)
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
}
