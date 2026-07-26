use crate::error::SketchError;
use apache_datasketches_sys::hll::ffi as sys;
use cxx::UniquePtr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetHllType {
    Hll4,
    Hll6,
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

pub struct HllSketch {
    pub(crate) inner: UniquePtr<sys::HllSketchShim>,
}

unsafe impl Send for HllSketch {}

impl HllSketch {
    pub fn new(lg_config_k: u8, tgt_type: TargetHllType) -> Result<Self, SketchError> {
        let inner = sys::new_hll_sketch(lg_config_k, tgt_type.into())
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn copy_as(&self, tgt_type: TargetHllType) -> Self {
        let inner = sys::hll_sketch_copy_as(&self.inner, tgt_type.into());
        Self { inner }
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::hll_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
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

    pub fn get_lg_config_k(&self) -> u8 {
        self.inner.get_lg_config_k()
    }

    pub fn get_target_type(&self) -> TargetHllType {
        self.inner.get_target_type().into()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    pub fn to_string_summary(&self) -> String {
        self.inner.to_string_summary()
    }

    pub fn serialize_compact(&self) -> Vec<u8> {
        self.inner.serialize_compact()
    }

    pub fn serialize_updatable(&self) -> Vec<u8> {
        self.inner.serialize_updatable()
    }
}
