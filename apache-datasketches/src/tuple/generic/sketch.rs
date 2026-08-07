use super::builder::resize_factor_multiplier;
use super::summary::{erase, TupleSummary};
use crate::error::SketchError;
use crate::tuple::ResizeFactor;
use apache_datasketches_sys::tuple_generic::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// A mutable, update-only Tuple sketch carrying a user-defined summary `S`
/// per distinct key. Build one with
/// [`TupleSketchBuilder`](super::TupleSketchBuilder).
pub struct TupleSketch<S: TupleSummary> {
    pub(crate) inner: UniquePtr<sys::TupleGenericSketchShim>,
    pub(crate) _marker: PhantomData<fn() -> S>,
}

// Sound because `S: Send` is a supertrait of `TupleSummary` and the sys-crate
// box is `Box<dyn RawSummaryOps + Send>`. Deliberately not `Sync`.
unsafe impl<S: TupleSummary> Send for TupleSketch<S> {}

impl<S: TupleSummary> TupleSketch<S> {
    pub(crate) fn from_parts(lg_k: u8, rf: ResizeFactor, p: f32) -> Result<Self, SketchError> {
        let inner = sys::new_tuple_generic_sketch(lg_k, resize_factor_multiplier(rf), p)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self {
            inner,
            _marker: PhantomData,
        })
    }

    /// Adds a `u64` key. `S::create` runs first, entirely in Rust; the
    /// resulting summary is combined into an existing entry or cloned into a
    /// new one.
    pub fn update_u64(&mut self, key: u64, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_u64(key, &summary);
    }

    /// Adds an `i64` key. See [`Self::update_u64`].
    pub fn update_i64(&mut self, key: i64, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_i64(key, &summary);
    }

    /// Adds a `u32` key. See [`Self::update_u64`].
    pub fn update_u32(&mut self, key: u32, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_u32(key, &summary);
    }

    /// Adds an `i32` key. See [`Self::update_u64`].
    pub fn update_i32(&mut self, key: i32, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_i32(key, &summary);
    }

    /// Adds a `u16` key. See [`Self::update_u64`].
    pub fn update_u16(&mut self, key: u16, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_u16(key, &summary);
    }

    /// Adds an `i16` key. See [`Self::update_u64`].
    pub fn update_i16(&mut self, key: i16, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_i16(key, &summary);
    }

    /// Adds a `u8` key. See [`Self::update_u64`].
    pub fn update_u8(&mut self, key: u8, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_u8(key, &summary);
    }

    /// Adds an `i8` key. See [`Self::update_u64`].
    pub fn update_i8(&mut self, key: i8, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_i8(key, &summary);
    }

    /// Adds an `f64` key. See [`Self::update_u64`].
    pub fn update_f64(&mut self, key: f64, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_f64(key, &summary);
    }

    /// Adds a string key. See [`Self::update_u64`].
    pub fn update_str(&mut self, key: &str, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_str(key, &summary);
    }

    /// Adds an arbitrary byte-slice key. See [`Self::update_u64`].
    pub fn update_bytes(&mut self, key: &[u8], value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_bytes(key, &summary);
    }

    /// Removes retained entries in excess of the nominal size `k`, lowering
    /// theta to do so. Note this shifts [`Self::get_estimate`].
    pub fn trim(&mut self) {
        self.inner.pin_mut().trim();
    }

    /// Resets this sketch to its initial, empty state.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns the current estimate of the number of distinct keys added.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`], for `num_std_dev` of `1`, `2`, or `3`.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See [`Self::get_lower_bound`].
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if no keys have been added.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the sketch has begun sampling.
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if retained entries are sorted by hash value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold.
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of retained entries.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }
}
