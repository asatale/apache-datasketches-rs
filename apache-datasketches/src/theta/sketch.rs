use crate::error::SketchError;
use crate::theta::builder::ResizeFactor;
use apache_datasketches_sys::theta_sketch::ffi as sys;
use cxx::UniquePtr;

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

    pub fn update_str(&mut self, value: &str) {
        self.inner.pin_mut().update_str(value);
    }

    pub fn update_bytes(&mut self, value: &[u8]) {
        self.inner.pin_mut().update_bytes(value);
    }

    pub fn trim(&mut self) {
        self.inner.pin_mut().trim();
    }

    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
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

    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }

    pub fn compact(&self, ordered: bool) -> super::CompactThetaSketch {
        super::CompactThetaSketch::from_shim(self.inner.compact(ordered))
    }
}
