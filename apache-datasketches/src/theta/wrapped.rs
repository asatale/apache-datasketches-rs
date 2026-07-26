use crate::error::SketchError;
use apache_datasketches_sys::theta_wrapped::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

pub struct WrappedCompactThetaSketch<'a> {
    pub(crate) inner: UniquePtr<sys::WrappedCompactThetaSketchShim>,
    _marker: PhantomData<&'a [u8]>,
}

unsafe impl<'a> Send for WrappedCompactThetaSketch<'a> {}

impl<'a> WrappedCompactThetaSketch<'a> {
    pub fn wrap(bytes: &'a [u8]) -> Result<Self, SketchError> {
        let inner = sys::wrapped_compact_theta_sketch_wrap(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self {
            inner,
            _marker: PhantomData,
        })
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
}
