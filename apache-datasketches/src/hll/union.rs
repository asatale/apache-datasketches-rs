// apache-datasketches/src/hll/union.rs
use crate::error::SketchError;
use crate::hll::sketch::{HllSketch, TargetHllType};
use apache_datasketches_sys::hll::ffi as sys;
use cxx::UniquePtr;

/// Merges multiple [`HllSketch`]es into one, e.g. combining per-shard or
/// per-day counts into a total distinct count across all of them.
pub struct HllUnion {
    inner: UniquePtr<sys::HllUnionShim>,
}

unsafe impl Send for HllUnion {}

impl HllUnion {
    /// Creates a new, empty union with the given maximum `lg_config_k`
    /// (`4..=21`) — the union's result will use at most this `lg_config_k`,
    /// even if a merged-in sketch used a larger one. Returns
    /// [`SketchError::InvalidConfig`] if `lg_max_k` is out of range.
    pub fn new(lg_max_k: u8) -> Result<Self, SketchError> {
        let inner = sys::new_hll_union(lg_max_k)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Merges the given sketch's state into this union.
    pub fn update_sketch(&mut self, sketch: &HllSketch) {
        self.inner.pin_mut().update_sketch(&sketch.inner);
    }

    /// Adds a `u64` value directly to the union, as if it were added to
    /// every sketch merged into it.
    pub fn update_u64(&mut self, value: u64) {
        self.inner.pin_mut().update_u64(value);
    }

    /// Adds an `i64` value directly to the union. See [`Self::update_u64`].
    pub fn update_i64(&mut self, value: i64) {
        self.inner.pin_mut().update_i64(value);
    }

    /// Adds an `f64` value directly to the union. See [`Self::update_u64`].
    pub fn update_f64(&mut self, value: f64) {
        self.inner.pin_mut().update_f64(value);
    }

    /// Adds a string value directly to the union. See [`Self::update_u64`].
    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    /// Adds an arbitrary byte slice directly to the union. See
    /// [`Self::update_u64`].
    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

    /// Returns the current merged result as a standalone [`HllSketch`] of
    /// the given [`TargetHllType`].
    pub fn get_result(&self, tgt_type: TargetHllType) -> HllSketch {
        let inner = self.inner.get_result(tgt_type.into());
        HllSketch { inner }
    }

    /// Serializes `get_result(tgt_type)` in compact form. A union has no
    /// serializable state of its own upstream (only the result sketch
    /// does) — to resume accumulating after deserializing, use
    /// `HllSketch::deserialize` and feed the sketch back in via
    /// `update_sketch`.
    pub fn serialize_compact(&self, tgt_type: TargetHllType) -> Vec<u8> {
        self.inner.serialize_compact(tgt_type.into())
    }

    /// Serializes `get_result(tgt_type)` in updatable form. See
    /// [`Self::serialize_compact`] for why `HllUnion` has no `deserialize`.
    pub fn serialize_updatable(&self, tgt_type: TargetHllType) -> Vec<u8> {
        self.inner.serialize_updatable(tgt_type.into())
    }

    /// Returns the current estimate of the number of distinct items merged
    /// into this union so far.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`]. See [`HllSketch::get_lower_bound`] for the
    /// meaning of `num_std_dev`.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See [`HllSketch::get_lower_bound`] for the
    /// meaning of `num_std_dev`.
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if no sketch or item has been merged into this union.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Resets this union to its initial, empty state.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }
}
