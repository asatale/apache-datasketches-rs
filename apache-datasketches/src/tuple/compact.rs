use crate::error::SketchError;
use apache_datasketches_sys::array_of_doubles_compact::ffi as sys;
use cxx::UniquePtr;

/// An immutable, serializable snapshot of an ArrayOfDoubles Tuple sketch.
/// Produced by [`super::ArrayOfDoublesSketch::compact`], by any set
/// operation's result, or by [`Self::deserialize`].
pub struct CompactArrayOfDoublesSketch {
    pub(crate) inner: UniquePtr<sys::CompactArrayOfDoublesSketchShim>,
}

unsafe impl Send for CompactArrayOfDoublesSketch {}

impl CompactArrayOfDoublesSketch {
    pub(crate) fn from_shim(inner: UniquePtr<sys::CompactArrayOfDoublesSketchShim>) -> Self {
        Self { inner }
    }

    /// Deserializes bytes produced by [`Self::serialize`]. Returns
    /// [`SketchError::Deserialization`] if the bytes are truncated, corrupt,
    /// or not an ArrayOfDoubles sketch.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = sys::compact_array_of_doubles_sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Serializes this sketch. Unlike Theta, this family has exactly one
    /// serialization format upstream — there is no compressed variant — and
    /// no `ordered` parameter: orderedness is fixed when the snapshot was
    /// created (e.g. via
    /// [`ArrayOfDoublesSketch::compact`](super::ArrayOfDoublesSketch::compact)).
    pub fn serialize(&self) -> Vec<u8> {
        self.inner.serialize().as_slice().to_vec()
    }

    /// Returns the current estimate of the number of distinct keys in this
    /// sketch.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`]. See
    /// [`ArrayOfDoublesSketch::get_lower_bound`](super::ArrayOfDoublesSketch::get_lower_bound)
    /// for the meaning of `num_std_dev`.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See
    /// [`ArrayOfDoublesSketch::get_lower_bound`](super::ArrayOfDoublesSketch::get_lower_bound)
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
    /// (i.e. [`Self::get_estimate`] is a statistical estimate rather than an
    /// exact count).
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if this sketch's retained entries are sorted by hash
    /// value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold (`1.0` if not in estimation mode).
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of entries retained by this sketch.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }

    /// Returns the fixed number of `f64` values each retained entry carries.
    pub fn get_num_values(&self) -> u8 {
        self.inner.get_num_values()
    }

    /// Iterates the retained entries as `(hash, values)` pairs, where
    /// `values.len() == self.get_num_values()`. Ordered by hash if
    /// [`Self::is_ordered`] is `true`.
    ///
    /// The entries are copied out of C++ in two FFI calls up front (cxx
    /// cannot hand back a live C++ iterator), so each item owns its `Vec`
    /// rather than borrowing from the sketch.
    pub fn entries(&self) -> impl Iterator<Item = (u64, Vec<f64>)> {
        let num_values = self.inner.get_num_values() as usize;
        let hashes: Vec<u64> = self.inner.entry_hashes().as_slice().to_vec();
        let values: Vec<f64> = self.inner.entry_values().as_slice().to_vec();
        let grouped: Vec<Vec<f64>> = if num_values == 0 {
            Vec::new()
        } else {
            values.chunks(num_values).map(|c| c.to_vec()).collect()
        };
        hashes.into_iter().zip(grouped)
    }
}
