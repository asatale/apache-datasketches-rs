use super::checks::{check_item, check_k, check_rank, check_split_points};
use super::KllItem;
use super::KllSortedView;
use crate::error::SketchError;
use cxx::UniquePtr;

/// Upstream `kll_constants::DEFAULT_K`: about 1.33% normalized rank error
/// for rank/quantile/CDF queries, about 1.65% for PMF queries.
const DEFAULT_K: u16 = 200;

/// The first merged `n` upstream merge cannot handle: merges must keep `n`
/// strictly below 2^63.
///
/// Upstream `merge` sums the two `n`s in a plain `uint64_t` and passes the
/// result to `kll_helper::ub_on_num_levels` (kll_sketch_impl.hpp:788, in
/// `merge_higher_levels`). That calls `floor_of_log2_of_fraction(n, 1)`
/// (kll_helper_impl.hpp:38-51), which doubles a `uint64_t` until it exceeds
/// `n`; for `n >= 2^63` the doubling wraps to 0 and it loops forever, and a
/// wrapped sum would undersize merge's work arrays (a heap overflow in
/// `populate_work_arrays`).
///
/// Below 2^63 the level count also stays within the 61 levels upstream's
/// capacity math allows (`int_cap_aux` rejects depth > 60): a level `j` is
/// only created by compacting at least `m` = 8 items one level down, leaving
/// at least 4 items of weight 2^j, so 4 * 2^j <= n < 2^63 gives j <= 60.
/// The deserializer enforces the same bound on incoming bytes.
const MERGED_N_LIMIT: u64 = 1 << 63;

/// A KLL quantiles sketch over `f64` or `f32` items: answers rank, quantile,
/// PMF and CDF queries over a stream using bounded memory, with an error
/// bound set by `k` (see [`Self::get_normalized_rank_error`]).
///
/// Merging is a method ([`Self::merge`]); there is no separate union type.
///
/// # Thread safety
///
/// `KllSketch` is `Send` but **not** `Sync`. Upstream's query methods
/// (`get_quantile`, `get_rank`, `get_pmf`, `get_cdf`) are logically
/// read-only but build and cache a sorted view inside the sketch, so two
/// threads querying one sketch at once would race. To share query access
/// across threads, build a `KllSortedView` once with `sorted_view()` and
/// share that (it is `Sync`).
///
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<apache_datasketches::kll::KllSketch<f64>>();
/// ```
///
/// ```
/// fn assert_send<T: Send>() {}
/// assert_send::<apache_datasketches::kll::KllSketch<f64>>();
/// ```
///
/// # Randomness
///
/// Compaction is randomized upstream and cannot be seeded, so once more than
/// `k` items have been added, two sketches fed identical input can give
/// slightly different (equally valid) answers.
pub struct KllSketch<T: KllItem> {
    inner: UniquePtr<T::Sketch>,
}

// SAFETY: the shim owns its kll_sketch outright and holds no thread-affine
// state, so moving it to another thread is sound. The random state upstream
// uses for compaction is not owned by the sketch: it lives in namespace-level
// `thread_local` engines (common_defs.hpp), so a moved sketch simply draws
// from the new thread's engine. It is deliberately not Sync: see the
// type-level docs.
unsafe impl<T: KllItem> Send for KllSketch<T> {}

impl<T: KllItem> KllSketch<T> {
    /// Creates an empty sketch. `k` must be in `8..=65535`; larger `k` means
    /// smaller error and more memory. [`Default`] uses `k = 200`.
    pub fn new(k: u16) -> Result<Self, SketchError> {
        check_k(k)?;
        let inner =
            T::sketch_new(k).map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Reconstructs a sketch from bytes produced by [`Self::serialize`], or by
    /// the Java/Python libraries' `KllDoublesSketch`/`KllFloatsSketch` for the
    /// matching item type.
    ///
    /// An empty sketch's bytes carry no item type, so they deserialize as
    /// either `f64` or `f32`; non-empty bytes of the other item type are
    /// rejected.
    ///
    /// Bytes are rejected with [`SketchError::Deserialization`] when their
    /// level boundaries are inconsistent with `n`, they claim more levels
    /// than `n` allows, they carry `n >= 2^63`, they contain NaN items, or
    /// (when non-empty) `k < 8`. The bytes' internal sortedness and `min_k`
    /// are not validated, so crafted-but-consistent bytes can yield wrong
    /// answers (never memory unsafety).
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = T::sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// The normalized rank error a sketch with this `k` would have: the
    /// single-sided error for rank/quantile/CDF queries, or the double-sided
    /// error for PMF queries when `pmf` is `true`.
    pub fn normalized_rank_error_for_k(k: u16, pmf: bool) -> Result<f64, SketchError> {
        check_k(k)?;
        Ok(T::normalized_rank_error_for_k(k, pmf))
    }

    /// Adds an item. NaN is silently ignored, as upstream does.
    pub fn update(&mut self, item: T) {
        T::update(self.inner.pin_mut(), item);
    }

    /// Merges `other` into this sketch. `other` may have a different `k`;
    /// the result's error is then governed by the smaller `k`.
    ///
    /// # Panics
    ///
    /// Panics if the merged `n` (`self.get_n() + other.get_n()`) would reach
    /// 2^63, which upstream cannot handle: its merge then loops forever or
    /// overflows its work arrays. No real stream gets near this (it takes
    /// over 9.2 quintillion items); it is reachable in practice only by
    /// repeatedly merging a clone of a sketch into it, or via crafted bytes
    /// (two sketches near 2^62).
    pub fn merge(&mut self, other: &KllSketch<T>) {
        let fits = self
            .get_n()
            .checked_add(other.get_n())
            .is_some_and(|n| n < MERGED_N_LIMIT);
        assert!(
            fits,
            "KLL merge would make n reach 2^63 ({} + {}), which upstream cannot handle",
            self.get_n(),
            other.get_n()
        );
        T::merge(self.inner.pin_mut(), &other.inner);
    }

    /// Returns `true` if no items have been added.
    pub fn is_empty(&self) -> bool {
        T::is_empty(&self.inner)
    }

    /// The number of items added (NaN updates excluded).
    pub fn get_n(&self) -> u64 {
        T::get_n(&self.inner)
    }

    /// The `k` this sketch was built with.
    pub fn get_k(&self) -> u16 {
        T::get_k(&self.inner)
    }

    /// The number of items currently retained by the sketch.
    pub fn get_num_retained(&self) -> u32 {
        T::get_num_retained(&self.inner)
    }

    /// Returns `true` once the sketch has started compacting — i.e. once
    /// query results are estimates rather than exact.
    pub fn is_estimation_mode(&self) -> bool {
        T::is_estimation_mode(&self.inner)
    }

    /// The smallest item added (exact, never estimated), or `None` if empty.
    pub fn get_min_item(&self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(T::get_min_item(&self.inner).expect("a non-empty sketch has a min item"))
    }

    /// The largest item added (exact, never estimated), or `None` if empty.
    pub fn get_max_item(&self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        Some(T::get_max_item(&self.inner).expect("a non-empty sketch has a max item"))
    }

    /// The approximate item at normalized `rank` (in `[0, 1]`). With
    /// `inclusive`, the rank of an item includes its own weight.
    pub fn get_quantile(&self, rank: f64, inclusive: bool) -> Result<T, SketchError> {
        self.require_non_empty()?;
        check_rank(rank)?;
        Ok(T::get_quantile(&self.inner, rank, inclusive)?)
    }

    /// The approximate normalized rank (in `[0, 1]`) of `item`. With
    /// `inclusive`, the item's own weight is included. `item` must not be
    /// NaN.
    pub fn get_rank(&self, item: T, inclusive: bool) -> Result<f64, SketchError> {
        self.require_non_empty()?;
        check_item(item)?;
        Ok(T::get_rank(&self.inner, item, inclusive)?)
    }

    /// The approximate probability mass in each of the `m + 1` intervals the
    /// `m` split points divide the domain into. `split_points` must be
    /// strictly increasing and NaN-free. With `inclusive`, items equal to a
    /// split point count toward the interval on its left.
    pub fn get_pmf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError> {
        self.require_non_empty()?;
        check_split_points(split_points)?;
        Ok(T::get_pmf(&self.inner, split_points, inclusive)?)
    }

    /// The cumulative analog of [`Self::get_pmf`]: `m + 1` values, the last
    /// always `1.0`.
    pub fn get_cdf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError> {
        self.require_non_empty()?;
        check_split_points(split_points)?;
        Ok(T::get_cdf(&self.inner, split_points, inclusive)?)
    }

    /// This sketch's normalized rank error: single-sided (for rank,
    /// quantile and CDF queries), or double-sided for PMF queries when `pmf`
    /// is `true`.
    pub fn get_normalized_rank_error(&self, pmf: bool) -> f64 {
        T::get_normalized_rank_error(&self.inner, pmf)
    }

    /// Serializes this sketch. The format is shared with the Java and Python
    /// libraries.
    pub fn serialize(&self) -> Vec<u8> {
        T::serialize(&self.inner)
    }

    /// A human-readable, multi-line summary of the sketch's internal state —
    /// for debugging, not for parsing.
    pub fn to_string_summary(&self) -> String {
        T::to_string_summary(&self.inner)
    }

    /// The retained items with their weights, in the sketch's internal level
    /// order (not sorted). The weights sum to [`Self::get_n`].
    ///
    /// The items are copied out once when this is called, so the iterator
    /// does not borrow the sketch.
    pub fn iter(&self) -> impl Iterator<Item = (T, u64)> {
        let items = T::retained_items(&self.inner);
        let weights = T::retained_weights(&self.inner);
        items.into_iter().zip(weights)
    }

    /// A sorted, immutable snapshot of the retained items, answering the
    /// same queries as this sketch. Build one when running many queries
    /// against a sketch that is no longer changing, or to share query access
    /// between threads (the view is `Sync`; the sketch is not). An empty
    /// sketch gives an empty view, whose queries return
    /// [`SketchError::EmptySketch`].
    pub fn sorted_view(&self) -> KllSortedView<T> {
        KllSortedView::from_inner(T::sorted_view(&self.inner))
    }

    /// The shim, for the sibling `kolmogorov_smirnov` module.
    pub(crate) fn shim(&self) -> &T::Sketch {
        &self.inner
    }

    fn require_non_empty(&self) -> Result<(), SketchError> {
        if self.is_empty() {
            return Err(SketchError::EmptySketch);
        }
        Ok(())
    }
}

impl<T: KllItem> Clone for KllSketch<T> {
    fn clone(&self) -> Self {
        Self {
            inner: T::clone_shim(&self.inner),
        }
    }
}

impl<T: KllItem> Default for KllSketch<T> {
    /// An empty sketch with `k = 200`.
    fn default() -> Self {
        Self::new(DEFAULT_K).expect("DEFAULT_K is in range")
    }
}
