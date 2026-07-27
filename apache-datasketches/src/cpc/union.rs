use crate::cpc::sketch::CpcSketch;
use crate::error::SketchError;
use apache_datasketches_sys::cpc_union::ffi as sys;
use cxx::UniquePtr;

pub struct CpcUnion {
    inner: UniquePtr<sys::CpcUnionShim>,
}

unsafe impl Send for CpcUnion {}

impl CpcUnion {
    pub(crate) fn from_lg_k(lg_k: u8) -> Result<Self, SketchError> {
        let inner = sys::new_cpc_union(lg_k)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    pub fn update(&mut self, sketch: &CpcSketch) {
        self.inner.pin_mut().update_sketch(&sketch.inner);
    }

    pub fn get_result(&self) -> CpcSketch {
        let inner = self.inner.get_result();
        CpcSketch { inner }
    }
}
