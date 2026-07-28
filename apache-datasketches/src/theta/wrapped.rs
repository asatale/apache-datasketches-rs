use crate::error::SketchError;
use apache_datasketches_sys::theta_wrapped::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// A zero-copy, read-only view over an already-serialized compact theta
/// sketch's bytes, usable directly as set-operation input without a full
/// [`super::CompactThetaSketch::deserialize`]. Build one with [`Self::wrap`].
///
/// The lifetime `'a` ties this view to the borrowed `&'a [u8]` buffer it
/// wraps — it cannot outlive the bytes it was built from.
pub struct WrappedCompactThetaSketch<'a> {
    pub(crate) inner: UniquePtr<sys::WrappedCompactThetaSketchShim>,
    _marker: PhantomData<&'a [u8]>,
}

unsafe impl<'a> Send for WrappedCompactThetaSketch<'a> {}

impl<'a> WrappedCompactThetaSketch<'a> {
    /// Wraps a serialized compact theta sketch's bytes without copying or
    /// fully deserializing them.
    pub fn wrap(bytes: &'a [u8]) -> Result<Self, SketchError> {
        let inner = sys::wrapped_compact_theta_sketch_wrap(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self {
            inner,
            _marker: PhantomData,
        })
    }

    /// Returns the current estimate of the number of distinct items in the
    /// wrapped sketch.
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

    /// Returns `true` if the wrapped sketch represents an empty set.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the wrapped sketch's theta threshold is below
    /// `1.0` (i.e. [`Self::get_estimate`] is a statistical estimate rather
    /// than an exact count).
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if the wrapped sketch's retained entries are sorted
    /// by hash value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the wrapped sketch's current theta threshold (`1.0` if not
    /// in estimation mode).
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of entries retained by the wrapped sketch.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }
}
