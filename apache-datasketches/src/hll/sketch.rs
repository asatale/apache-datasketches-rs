use crate::error::SketchError;
use apache_datasketches_sys::hll::ffi as sys;
use cxx::UniquePtr;

/// Controls the internal representation HLL uses to store per-bucket
/// state, trading memory for accuracy. Mirrors upstream's
/// `datasketches::target_hll_type`.
///
/// `Hll4` is the most memory-compact; `Hll8` is the least compact but
/// fastest to update and most accurate at small cardinalities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetHllType {
    /// 4 bits per bucket — most memory-compact.
    Hll4,
    /// 6 bits per bucket.
    Hll6,
    /// 8 bits per bucket — least compact, fastest, most accurate at small n.
    Hll8,
}

impl From<TargetHllType> for sys::TargetHllType {
    fn from(t: TargetHllType) -> Self {
        match t {
            TargetHllType::Hll4 => sys::TargetHllType::Hll4,
            TargetHllType::Hll6 => sys::TargetHllType::Hll6,
            TargetHllType::Hll8 => sys::TargetHllType::Hll8,
        }
    }
}

impl From<sys::TargetHllType> for TargetHllType {
    fn from(t: sys::TargetHllType) -> Self {
        match t {
            sys::TargetHllType::Hll4 => TargetHllType::Hll4,
            sys::TargetHllType::Hll6 => TargetHllType::Hll6,
            sys::TargetHllType::Hll8 => TargetHllType::Hll8,
            _ => unreachable!("unknown TargetHllType variant from cxx bridge"),
        }
    }
}

/// A HyperLogLog sketch: estimates the number of distinct items added via
/// `update_*`, using bounded memory regardless of how many items are added.
///
/// `lg_config_k` (passed to [`HllSketch::new`], valid range `4..=21`) trades
/// memory for accuracy: higher values are more accurate but use more space.
pub struct HllSketch {
    pub(crate) inner: UniquePtr<sys::HllSketchShim>,
}

unsafe impl Send for HllSketch {}

impl HllSketch {
    /// Creates a new, empty sketch with the given `lg_config_k` (`4..=21`)
    /// and internal representation. Returns [`SketchError::InvalidConfig`]
    /// if `lg_config_k` is out of range.
    pub fn new(lg_config_k: u8, tgt_type: TargetHllType) -> Result<Self, SketchError> {
        let inner = sys::new_hll_sketch(lg_config_k, tgt_type.into())
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Returns a copy of this sketch converted to a different
    /// [`TargetHllType`], preserving its current state.
    pub fn copy_as(&self, tgt_type: TargetHllType) -> Self {
        let inner = sys::hll_sketch_copy_as(&self.inner, tgt_type.into());
        Self { inner }
    }

    /// Reconstructs a sketch from bytes produced by
    /// [`Self::serialize_compact`] or [`Self::serialize_updatable`].
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::hll_sketch_deserialize(bytes)
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

    /// Returns the `lg_config_k` this sketch was built with.
    pub fn get_lg_config_k(&self) -> u8 {
        self.inner.get_lg_config_k()
    }

    /// Returns the [`TargetHllType`] this sketch currently uses.
    pub fn get_target_type(&self) -> TargetHllType {
        self.inner.get_target_type().into()
    }

    /// Returns `true` if no items have been added to this sketch.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Resets this sketch to its initial, empty state.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns a human-readable, multi-line summary of this sketch's
    /// internal state — useful for debugging, not for parsing.
    pub fn to_string_summary(&self) -> String {
        self.inner.to_string_summary()
    }

    /// Serializes this sketch in compact form (read-only once
    /// deserialized): smaller on the wire, but a
    /// [`Self::deserialize`]d sketch produced from these bytes cannot be
    /// updated further. Use [`Self::serialize_updatable`] if you need to
    /// resume adding items after deserializing.
    pub fn serialize_compact(&self) -> Vec<u8> {
        self.inner.serialize_compact().as_slice().to_vec()
    }

    /// Serializes this sketch in updatable form: larger on the wire than
    /// [`Self::serialize_compact`], but a deserialized sketch can have more
    /// items added to it.
    pub fn serialize_updatable(&self) -> Vec<u8> {
        self.inner.serialize_updatable().as_slice().to_vec()
    }
}
