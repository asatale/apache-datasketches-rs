use crate::error::SketchError;
use crate::tuple::builder::ResizeFactor;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sys;
use cxx::UniquePtr;

/// A mutable, update-only ArrayOfDoubles Tuple sketch: estimates the number
/// of distinct keys added via `update_*`, and carries a fixed-width array of
/// `f64` values per retained key, summed on collision. Build one with
/// [`ArrayOfDoublesSketchBuilder`](super::ArrayOfDoublesSketchBuilder).
///
/// Call [`Self::compact`] to produce an immutable, serializable
/// [`super::CompactArrayOfDoublesSketch`] snapshot for storage, transmission,
/// or use as input to a set operation.
pub struct ArrayOfDoublesSketch {
    pub(crate) inner: UniquePtr<sys::ArrayOfDoublesSketchShim>,
    /// Cached copy of the shim's `get_num_values()`.
    ///
    /// Every `update_*` has to validate the caller's slice length (see
    /// [`Self::check_values`]), and reading `num_values` back from C++ to do
    /// so cost a full FFI crossing per update — on the order of the update
    /// itself.
    ///
    /// Caching is sound because the value is fixed for the sketch's lifetime:
    /// it lives in the C++ update policy, is set once when the builder
    /// constructs the sketch, has no setter, and `reset()` clears the hash
    /// table without touching the policy.
    ///
    /// **This invariant is what makes the cache correct, and the compiler will
    /// not enforce it.** `from_parts` is currently the only constructor. Any
    /// new one — a `deserialize`, or a `from_shim` wrapping a sketch built
    /// elsewhere — must populate this field from the shim rather than assume a
    /// value, or the cache silently disagrees with C++ and `check_values`
    /// starts rejecting valid slices (or admitting short ones).
    num_values: u8,
}

unsafe impl Send for ArrayOfDoublesSketch {}

impl ArrayOfDoublesSketch {
    pub(crate) fn from_parts(
        lg_k: u8,
        rf: ResizeFactor,
        p: f32,
        num_values: u8,
    ) -> Result<Self, SketchError> {
        if num_values == 0 {
            return Err(SketchError::InvalidConfig(
                "num_values must be at least 1".to_string(),
            ));
        }
        let inner = sys::new_array_of_doubles_sketch(lg_k, rf.into(), p, num_values)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner, num_values })
    }

    /// Validates that `values` has exactly [`Self::get_num_values`] elements.
    ///
    /// This check cannot be delegated to the C++ layer's exceptions the way
    /// `lg_k`/`num_std_dev` validation is: upstream's update policy indexes
    /// the supplied values blindly for `i in 0..num_values`, so a short slice
    /// would be an out-of-bounds read rather than a graceful failure.
    ///
    /// Reads the cached [`Self::num_values`] rather than calling into C++, so
    /// this costs no FFI crossing.
    fn check_values(&self, values: &[f64]) -> Result<(), SketchError> {
        let expected = self.num_values as usize;
        if values.len() != expected {
            return Err(SketchError::InvalidConfig(format!(
                "expected {expected} values, got {}",
                values.len()
            )));
        }
        Ok(())
    }

    /// Adds a `u64` key with its associated values. Returns
    /// [`SketchError::InvalidConfig`] unless
    /// `values.len() == self.get_num_values()`.
    pub fn update_u64(&mut self, key: u64, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_u64(key, values);
        Ok(())
    }

    /// Adds an `i64` key with its associated values. See [`Self::update_u64`].
    pub fn update_i64(&mut self, key: i64, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_i64(key, values);
        Ok(())
    }

    /// Adds a `u32` key with its associated values. See [`Self::update_u64`].
    pub fn update_u32(&mut self, key: u32, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_u32(key, values);
        Ok(())
    }

    /// Adds an `i32` key with its associated values. See [`Self::update_u64`].
    pub fn update_i32(&mut self, key: i32, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_i32(key, values);
        Ok(())
    }

    /// Adds a `u16` key with its associated values. See [`Self::update_u64`].
    pub fn update_u16(&mut self, key: u16, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_u16(key, values);
        Ok(())
    }

    /// Adds an `i16` key with its associated values. See [`Self::update_u64`].
    pub fn update_i16(&mut self, key: i16, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_i16(key, values);
        Ok(())
    }

    /// Adds a `u8` key with its associated values. See [`Self::update_u64`].
    pub fn update_u8(&mut self, key: u8, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_u8(key, values);
        Ok(())
    }

    /// Adds an `i8` key with its associated values. See [`Self::update_u64`].
    pub fn update_i8(&mut self, key: i8, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_i8(key, values);
        Ok(())
    }

    /// Adds an `f64` key with its associated values. See [`Self::update_u64`].
    pub fn update_f64(&mut self, key: f64, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_f64(key, values);
        Ok(())
    }

    /// Adds a string key with its associated values. See [`Self::update_u64`].
    pub fn update_str(&mut self, key: &str, values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_str(key, values);
        Ok(())
    }

    /// Adds an arbitrary byte-slice key with its associated values. See
    /// [`Self::update_u64`].
    pub fn update_bytes(&mut self, key: &[u8], values: &[f64]) -> Result<(), SketchError> {
        self.check_values(values)?;
        self.inner.pin_mut().update_bytes(key, values);
        Ok(())
    }

    /// Removes retained entries in excess of the nominal size `k`, lowering
    /// the theta threshold to do so.
    ///
    /// Note that this *does* shift [`Self::get_estimate`] — trimming lowers
    /// theta, and the estimate is derived from the retained count and theta
    /// together. Upstream only guarantees the excess entries are dropped.
    pub fn trim(&mut self) {
        self.inner.pin_mut().trim();
    }

    /// Resets this sketch to its initial, empty state. `num_values` is
    /// preserved.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns the current estimate of the number of distinct keys added.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`], for the given number of standard deviations
    /// (`1`, `2`, or `3`, corresponding to roughly 67%, 95%, and 99%
    /// confidence). Returns [`SketchError::InvalidConfig`] for any other
    /// value.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See [`Self::get_lower_bound`] for the meaning
    /// of `num_std_dev`.
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if no keys have been added to this sketch.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if this sketch's theta threshold is below `1.0`
    /// (i.e. it has begun sampling and [`Self::get_estimate`] is a
    /// statistical estimate rather than an exact count).
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if this sketch's retained entries are sorted by hash
    /// value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold (`1.0` until sampling begins).
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of entries currently retained by this sketch.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }

    /// Returns the fixed number of `f64` values each retained entry carries,
    /// as configured at build time.
    pub fn get_num_values(&self) -> u8 {
        self.num_values
    }

    /// Iterates the retained entries as `(hash, values)` pairs, where
    /// `values.len() == self.get_num_values()`.
    ///
    /// The entries are copied out of C++ in two FFI calls up front (cxx
    /// cannot hand back a live C++ iterator), so each item owns its `Vec`
    /// rather than borrowing from the sketch. Iteration order is unspecified
    /// for an update sketch; compact it with `ordered = true` for
    /// hash-ordered iteration.
    pub fn entries(&self) -> impl Iterator<Item = (u64, Vec<f64>)> {
        let num_values = self.num_values as usize;
        let hashes: Vec<u64> = self.inner.entry_hashes().as_slice().to_vec();
        let values: Vec<f64> = self.inner.entry_values().as_slice().to_vec();
        let grouped: Vec<Vec<f64>> = if num_values == 0 {
            Vec::new()
        } else {
            values.chunks(num_values).map(|c| c.to_vec()).collect()
        };
        hashes.into_iter().zip(grouped)
    }

    /// Produces an immutable, serializable
    /// [`super::CompactArrayOfDoublesSketch`] snapshot of this sketch's
    /// current state. If `ordered` is `true`, the snapshot's entries are
    /// sorted by hash value.
    pub fn compact(&self, ordered: bool) -> super::CompactArrayOfDoublesSketch {
        super::CompactArrayOfDoublesSketch::from_shim(self.inner.compact(ordered))
    }
}
