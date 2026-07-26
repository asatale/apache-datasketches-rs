// apache-datasketches/src/hll/union.rs
use crate::error::SketchError;
use crate::hll::sketch::{HllSketch, TargetHllType};
use apache_datasketches_sys::hll::ffi as sys;
use cxx::UniquePtr;

pub struct HllUnion {
    inner: UniquePtr<sys::HllUnionShim>,
}

unsafe impl Send for HllUnion {}

impl HllUnion {
    pub fn new(lg_max_k: u8) -> Result<Self, SketchError> {
        let inner = sys::new_hll_union(lg_max_k)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn update_sketch(&mut self, sketch: &HllSketch) {
        self.inner.pin_mut().update_sketch(&sketch.inner);
    }

    pub fn update_u64(&mut self, value: u64) {
        self.inner.pin_mut().update_u64(value);
    }

    pub fn update_i64(&mut self, value: i64) {
        self.inner.pin_mut().update_i64(value);
    }

    pub fn update_f64(&mut self, value: f64) {
        self.inner.pin_mut().update_f64(value);
    }

    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

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

    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }
}
