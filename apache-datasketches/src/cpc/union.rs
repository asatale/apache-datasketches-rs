use crate::cpc::sketch::CpcSketch;
use crate::error::SketchError;
use apache_datasketches_sys::cpc_union::ffi as sys;
use cxx::UniquePtr;

/// Merges multiple [`CpcSketch`]es into one, e.g. combining per-shard or
/// per-day counts into a total distinct count across all of them. Build
/// one with [`CpcUnionBuilder`](super::CpcUnionBuilder).
///
/// Unlike HLL's `HllUnion`, `CpcUnion` has no
/// `get_estimate`/`is_empty`/`reset` convenience methods of its own —
/// upstream's `cpc_union` doesn't have them either; query the sketch
/// returned by [`Self::get_result`] instead.
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

    /// Merges the given sketch's state into this union.
    pub fn update(&mut self, sketch: &CpcSketch) {
        self.inner.pin_mut().update_sketch(&sketch.inner);
    }

    /// Returns the current merged result as a standalone [`CpcSketch`].
    pub fn get_result(&self) -> CpcSketch {
        let inner = self.inner.get_result();
        CpcSketch { inner }
    }
}
