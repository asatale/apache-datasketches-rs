use crate::error::SketchError;
use apache_datasketches_sys::theta_compact::ffi as sys;
use cxx::UniquePtr;

pub struct CompactThetaSketch {
    pub(crate) inner: UniquePtr<sys::CompactThetaSketchShim>,
}

unsafe impl Send for CompactThetaSketch {}

impl CompactThetaSketch {
    pub(crate) fn from_shim(inner: UniquePtr<sys::CompactThetaSketchShim>) -> Self {
        Self { inner }
    }

    /// Deserializes v1/v2/v3 (uncompressed) bytes. Upstream's `deserialize()`
    /// auto-detects the serial version transparently, including v4
    /// (compressed) — see [`Self::deserialize_compressed`], which calls the
    /// exact same underlying routine; the two Rust names exist purely for
    /// call-site symmetry with `serialize_compact`/`serialize_compressed`.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::compact_theta_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Deserializes v4 (compressed) bytes. See [`Self::deserialize`] — both
    /// methods call the same upstream auto-detecting `deserialize()`.
    pub fn deserialize_compressed(bytes: &[u8]) -> Result<Self, SketchError> {
        Self::deserialize(bytes)
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

    /// Serializes in the v3 (uncompressed) format. Note: unlike the design
    /// spec's initially-sketched signature, this takes no `ordered`
    /// parameter — upstream's `compact_theta_sketch::serialize()` has none;
    /// orderedness is fixed when this sketch was created (e.g. via
    /// `ThetaSketch::compact(ordered)`).
    pub fn serialize_compact(&self) -> Vec<u8> {
        self.inner.serialize_compact()
    }

    /// Serializes in the v4 (compressed) format.
    pub fn serialize_compressed(&self) -> Vec<u8> {
        self.inner.serialize_compressed()
    }
}
