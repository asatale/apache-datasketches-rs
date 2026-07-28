use crate::error::SketchError;
use apache_datasketches_sys::theta_compact::ffi as sys;
use cxx::UniquePtr;

/// An immutable, serializable snapshot of a theta sketch. Produced by
/// [`super::ThetaSketch::compact`], by any set operation's result
/// (`ThetaUnion::get_result`, `ThetaIntersection::get_result`,
/// `ThetaAnotB::compute`), or by [`Self::deserialize`].
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

    /// Returns the current estimate of the number of distinct items in this
    /// sketch.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`]. See
    /// [`ThetaSketch::get_lower_bound`](super::ThetaSketch::get_lower_bound)
    /// for the meaning of `num_std_dev`.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See
    /// [`ThetaSketch::get_lower_bound`](super::ThetaSketch::get_lower_bound)
    /// for the meaning of `num_std_dev`.
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if this sketch represents an empty set.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if this sketch's theta threshold is below `1.0`
    /// (i.e. [`Self::get_estimate`] is a statistical estimate rather than
    /// an exact count).
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if this sketch's retained entries are sorted by hash
    /// value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold (`1.0` if not in estimation
    /// mode).
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of entries currently retained by this sketch.
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
