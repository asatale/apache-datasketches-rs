use crate::error::SketchError;
use apache_datasketches_sys::cpc_sketch::ffi as sys;
use cxx::UniquePtr;

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

    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::cpc_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn update_u64(&mut self, value: u64) {
        self.inner.pin_mut().update_u64(value);
    }

    pub fn update_i64(&mut self, value: i64) {
        self.inner.pin_mut().update_i64(value);
    }

    pub fn update_u32(&mut self, value: u32) {
        self.inner.pin_mut().update_u32(value);
    }

    pub fn update_i32(&mut self, value: i32) {
        self.inner.pin_mut().update_i32(value);
    }

    pub fn update_u16(&mut self, value: u16) {
        self.inner.pin_mut().update_u16(value);
    }

    pub fn update_i16(&mut self, value: i16) {
        self.inner.pin_mut().update_i16(value);
    }

    pub fn update_u8(&mut self, value: u8) {
        self.inner.pin_mut().update_u8(value);
    }

    pub fn update_i8(&mut self, value: i8) {
        self.inner.pin_mut().update_i8(value);
    }

    pub fn update_f64(&mut self, value: f64) {
        self.inner.pin_mut().update_f64(value);
    }

    pub fn update_f32(&mut self, value: f32) {
        self.inner.pin_mut().update_f32(value);
    }

    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
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

    pub fn get_lg_k(&self) -> u8 {
        self.inner.get_lg_k()
    }

    pub fn to_string_summary(&self) -> String {
        self.inner.to_string_summary()
    }

    pub fn serialize(&self) -> Vec<u8> {
        self.inner.serialize()
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
