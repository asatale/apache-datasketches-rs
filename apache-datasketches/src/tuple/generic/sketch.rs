use super::builder::resize_factor_multiplier;
use super::summary::{erase, refill, TupleSummary};
use crate::error::SketchError;
use crate::tuple::ResizeFactor;
use apache_datasketches_sys::tuple_generic::ffi as sys;
use apache_datasketches_sys::tuple_generic::RustSummary;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// A mutable, update-only Tuple sketch carrying a user-defined summary `S`
/// per distinct key. Build one with
/// [`TupleSketchBuilder`](super::TupleSketchBuilder).
pub struct TupleSketch<S: TupleSummary> {
    pub(crate) inner: UniquePtr<sys::TupleGenericSketchShim>,
    /// A single erased summary, reused as the value argument of every update.
    ///
    /// Every `update_*` has to hand C++ a `RustSummary`, and building a fresh
    /// one per call heap-allocated a `Box<dyn RawSummaryOps>` every time —
    /// before the FFI crossing, so before upstream's theta screen, meaning even
    /// a key C++ immediately discarded paid for it. Holding one box and
    /// overwriting its contents (see `summary::refill`) makes the update path
    /// allocation-free.
    ///
    /// `None` until the first update, because constructing one needs an `S` and
    /// there is no `S` to be had before a caller supplies an update value.
    ///
    /// Not part of the sketch's logical state: it is scratch space, holds
    /// whatever the last update left behind, and is never read except by the
    /// update that just wrote it.
    ///
    /// One visible consequence: this keeps one `S` alive for the sketch's
    /// lifetime, and [`Self::reset`] does not release it — reset clears the C++
    /// table but leaves the scratch box allocated, deliberately, since the
    /// sketch is likely to be updated again. It is freed when the sketch drops.
    scratch: Option<RustSummary>,
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
            scratch: None,
            _marker: PhantomData,
        })
    }

    /// Loads `value` into the reused scratch summary, allocating only on the
    /// first call for this sketch.
    ///
    /// Returns nothing, and callers read `self.scratch` themselves, so that the
    /// immutable borrow of `scratch` and the mutable borrow of `inner` are two
    /// disjoint field borrows. A helper returning `&RustSummary` from
    /// `&mut self` would borrow the whole sketch and conflict with the
    /// `inner.pin_mut()` that has to follow.
    fn load_scratch(&mut self, value: S) {
        if let Some(existing) = self.scratch.as_mut() {
            refill(existing, value);
        } else {
            self.scratch = Some(erase(value));
        }
    }

    /// The scratch summary `load_scratch` just filled.
    ///
    /// Takes the field, not `&self`: a `&self` method borrows the whole sketch
    /// and so cannot coexist with the `self.inner.pin_mut()` that follows.
    /// Passing `&self.scratch` keeps it to one field, which leaves `inner` free
    /// to be borrowed mutably.
    ///
    /// The `expect` is unreachable — every caller runs `load_scratch` first, and
    /// that leaves `scratch` engaged on both paths.
    fn filled(scratch: &Option<RustSummary>) -> &RustSummary {
        scratch
            .as_ref()
            .expect("load_scratch always leaves the scratch summary engaged")
    }

    /// Adds a `u64` key. `S::create` runs first, entirely in Rust; the
    /// resulting summary is combined into an existing entry or cloned into a
    /// new one.
    pub fn update_u64(&mut self, key: u64, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_u64(key, summary);
    }

    /// Adds an `i64` key. See [`Self::update_u64`].
    pub fn update_i64(&mut self, key: i64, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_i64(key, summary);
    }

    /// Adds a `u32` key. See [`Self::update_u64`].
    pub fn update_u32(&mut self, key: u32, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_u32(key, summary);
    }

    /// Adds an `i32` key. See [`Self::update_u64`].
    pub fn update_i32(&mut self, key: i32, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_i32(key, summary);
    }

    /// Adds a `u16` key. See [`Self::update_u64`].
    pub fn update_u16(&mut self, key: u16, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_u16(key, summary);
    }

    /// Adds an `i16` key. See [`Self::update_u64`].
    pub fn update_i16(&mut self, key: i16, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_i16(key, summary);
    }

    /// Adds a `u8` key. See [`Self::update_u64`].
    pub fn update_u8(&mut self, key: u8, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_u8(key, summary);
    }

    /// Adds an `i8` key. See [`Self::update_u64`].
    pub fn update_i8(&mut self, key: i8, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_i8(key, summary);
    }

    /// Adds an `f64` key. See [`Self::update_u64`].
    pub fn update_f64(&mut self, key: f64, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_f64(key, summary);
    }

    /// Adds a string key. See [`Self::update_u64`].
    pub fn update_str(&mut self, key: &str, value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_str(key, summary);
    }

    /// Adds an arbitrary byte-slice key. See [`Self::update_u64`].
    pub fn update_bytes(&mut self, key: &[u8], value: &S::Update) {
        self.load_scratch(S::create(value));
        let summary = Self::filled(&self.scratch);
        self.inner.pin_mut().update_bytes(key, summary);
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

    /// Produces an immutable
    /// [`CompactTupleSketch`](super::CompactTupleSketch) snapshot. If
    /// `ordered` is `true`, its entries are sorted by hash value.
    pub fn compact(&self, ordered: bool) -> super::CompactTupleSketch<S> {
        super::CompactTupleSketch::from_shim(self.inner.compact(ordered))
    }
}
