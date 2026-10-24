use super::checks::{
    check_item, check_k, check_k_static, check_num_std_dev, check_rank, check_split_points,
};
use super::ReqItem;
use super::ReqSortedView;
use crate::error::SketchError;
use cxx::UniquePtr;

/// The `k` Java and Python default to: about 1% relative error at 95%
/// confidence.
const DEFAULT_K: u16 = 12;

/// The merged `n` must stay below this. Upstream's
/// `quantiles_sorted_view::get_quantile` (quantiles_sorted_view_impl.hpp:78)
/// casts `rank * n` to `uint64_t`, which is undefined behaviour once the
/// `double` rounds to 2^64 (n within about 2^10 of `u64::MAX`), and upstream's
/// `merge` adds `n_ += other.n_` unchecked (req_sketch_impl.hpp:204). 2^63 is
/// a round bound well below that, matching KLL's. The deserializer enforces
/// the same bound on incoming bytes.
const MERGED_N_LIMIT: u64 = 1 << 63;

/// Which end of the rank domain gets REQ's relative-error guarantee.
///
/// Upstream calls this `hra` (high rank accuracy), a `bool` defaulting to
/// `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankAccuracy {
    /// Accuracy concentrated at high ranks (p99, p99.9, …). The default.
    High,
    /// Accuracy concentrated at low ranks (p1, p0.1, …).
    Low,
}

impl RankAccuracy {
    fn is_high(self) -> bool {
        self == RankAccuracy::High
    }

    fn from_hra(hra: bool) -> Self {
        if hra {
            RankAccuracy::High
        } else {
            RankAccuracy::Low
        }
    }
}

/// A REQ (Relative Error Quantiles) sketch over `f64` or `f32` items.
///
/// Like a KLL sketch it answers rank, quantile, PMF and CDF queries in
/// bounded memory, but its error is *relative* to the rank, so it stays
/// accurate at one end of the distribution — the high ranks by default (see
/// [`RankAccuracy`]). Merging is a method ([`Self::merge`]); there is no
/// separate union type.
///
/// # Thread safety
///
/// `ReqSketch` is `Send` but **not** `Sync`: upstream's query methods are
/// logically read-only but build and cache a sorted view inside the sketch.
/// To share query access across threads, build a `ReqSortedView` once with
/// `sorted_view()` and share that (it is `Sync`).
///
/// ```compile_fail
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<apache_datasketches::req::ReqSketch<f64>>();
/// ```
///
/// ```
/// fn assert_send<T: Send>() {}
/// assert_send::<apache_datasketches::req::ReqSketch<f64>>();
/// ```
///
/// # Randomness
///
/// Compaction is randomized upstream and cannot be seeded, so once the
/// sketch is in estimation mode two sketches fed identical input can give
/// slightly different (equally valid) answers.
pub struct ReqSketch<T: ReqItem> {
    inner: UniquePtr<T::Sketch>,
}

// SAFETY: the shim owns its req_sketch outright and holds no thread-affine
// state. Upstream's compaction coin flips come from namespace-level
// `thread_local` engines (common_defs.hpp), not from the sketch, so a moved
// sketch simply draws from the new thread's engine. Deliberately not Sync:
// see the type-level docs.
unsafe impl<T: ReqItem> Send for ReqSketch<T> {}

impl<T: ReqItem> ReqSketch<T> {
    /// Creates an empty sketch. `k` must be even and in `4..=254`; larger `k`
    /// means smaller error and more memory. [`Default`] uses `k = 12` and
    /// [`RankAccuracy::High`].
    ///
    /// Upstream documents `k` up to 1024, but its 5.2.0 constructor truncates
    /// `k` to 8 bits, so a larger `k` would silently become a different one
    /// (256 becomes 4, 300 becomes 44); this crate rejects it instead.
    /// Non-empty sketches deserialized from bytes may carry `k` up to 1024.
    pub fn new(k: u16, accuracy: RankAccuracy) -> Result<Self, SketchError> {
        check_k(k)?;
        let inner = T::sketch_new(k, accuracy.is_high())
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Which end of the rank domain this sketch is accurate at.
    pub fn rank_accuracy(&self) -> RankAccuracy {
        RankAccuracy::from_hra(T::is_hra(&self.inner))
    }

    /// Reconstructs a sketch from bytes produced by [`Self::serialize`], or by
    /// Java's `ReqSketch` or Python's float REQ sketch (`f32` only).
    ///
    /// An empty sketch's bytes carry no item type, so they deserialize as
    /// either `f64` or `f32`; non-empty bytes of the other item type are
    /// rejected.
    ///
    /// Bytes are rejected with [`SketchError::Deserialization`] when they:
    ///
    /// - are truncated, have trailing bytes, or set unknown flag bits;
    /// - carry a `k` that is odd or outside `4..=1024`, or, for an empty
    ///   sketch, above 254 (upstream rebuilds empty sketches through its
    ///   constructor, which truncates `k` to 8 bits);
    /// - claim raw items with more than one level or other than 1 to 4 of
    ///   them;
    /// - have no levels, an empty level, a level whose weight is not
    ///   `2^index`, a section count or section size upstream could never
    ///   produce, or a compaction state of `2^46` or more;
    /// - retain as many items as their total nominal capacity;
    /// - have `n` different from the levels' total weight, `n >= 2^63`, or
    ///   more than one level at `n <= 4`;
    /// - contain NaN items.
    ///
    /// The levels' internal sortedness (and min/max against the items) is
    /// not validated, so crafted-but-consistent bytes can yield wrong
    /// answers (never memory unsafety). The compaction-state and `n` bounds
    /// are safety margins, so a sketch accepted here can, after enough merges
    /// or updates, serialize to bytes this validator rejects.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = T::sketch_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    /// Adds an item. NaN is silently ignored, as upstream does.
    pub fn update(&mut self, item: T) {
        T::update(self.inner.pin_mut(), item);
    }

    /// Merges `other` into this sketch. `other` may have a different `k`.
    ///
    /// Returns [`SketchError::InvalidConfig`], leaving this sketch unchanged,
    /// if the two sketches have different [`RankAccuracy`] or if the merged
    /// `n` would reach 2^63, which upstream cannot handle.
    pub fn merge(&mut self, other: &ReqSketch<T>) -> Result<(), SketchError> {
        if self.rank_accuracy() != other.rank_accuracy() {
            return Err(SketchError::InvalidConfig(format!(
                "cannot merge a {:?}-accuracy REQ sketch into a {:?}-accuracy one",
                other.rank_accuracy(),
                self.rank_accuracy()
            )));
        }
        let fits = self
            .get_n()
            .checked_add(other.get_n())
            .is_some_and(|n| n < MERGED_N_LIMIT);
        if !fits {
            return Err(SketchError::InvalidConfig(format!(
                "REQ merge would make n reach 2^63 ({} + {})",
                self.get_n(),
                other.get_n()
            )));
        }
        Ok(T::merge(self.inner.pin_mut(), &other.inner)?)
    }

    /// An approximate lower bound on the true normalized rank of an item
    /// whose estimated rank is `rank`, at `num_std_dev` (1, 2 or 3) standard
    /// deviations.
    pub fn get_rank_lower_bound(&self, rank: f64, num_std_dev: u8) -> Result<f64, SketchError> {
        self.require_non_empty()?;
        check_rank(rank)?;
        check_num_std_dev(num_std_dev)?;
        Ok(T::rank_lower_bound(&self.inner, rank, num_std_dev))
    }

    /// The upper-bound counterpart of [`Self::get_rank_lower_bound`].
    pub fn get_rank_upper_bound(&self, rank: f64, num_std_dev: u8) -> Result<f64, SketchError> {
        self.require_non_empty()?;
        check_rank(rank)?;
        check_num_std_dev(num_std_dev)?;
        Ok(T::rank_upper_bound(&self.inner, rank, num_std_dev))
    }

    /// The a-priori one-standard-deviation error of a rank estimate at
    /// normalized `rank`, for a sketch with this `k` and `accuracy` after `n`
    /// items, in units of normalized rank (at most `0.084 / k`).
    ///
    /// It is `0.0` where upstream treats the rank as exact (for small `n`,
    /// e.g. `n <= 36` at `k = 12`). `k` must be even and in `4..=1024`.
    ///
    /// This is upstream's `get_RSE`. (In datasketches-cpp 5.2.0, `get_RSE`
    /// returned a rank lower bound instead; that was fixed in 5.3.0.)
    pub fn a_priori_rank_error(
        k: u16,
        rank: f64,
        accuracy: RankAccuracy,
        n: u64,
    ) -> Result<f64, SketchError> {
        check_k_static(k)?;
        check_rank(rank)?;
        Ok(T::a_priori_rank_error(k, rank, accuracy.is_high(), n))
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

    /// Returns `true` once the sketch has more than one level, i.e. once
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
        // Upstream's iterator reads past the end on an empty sketch. The shim
        // guards `retained_items`/`retained_weights` too; this keeps the
        // safe layer independent of that.
        let (items, weights) = if self.is_empty() {
            (Vec::new(), Vec::new())
        } else {
            (
                T::retained_items(&self.inner),
                T::retained_weights(&self.inner),
            )
        };
        items.into_iter().zip(weights)
    }

    /// A sorted, immutable snapshot of the retained items, answering the
    /// same queries as this sketch. Build one when running many queries
    /// against a sketch that is no longer changing, or to share query access
    /// between threads (the view is `Sync`; the sketch is not). An empty
    /// sketch gives an empty view, whose queries return
    /// [`SketchError::EmptySketch`].
    pub fn sorted_view(&self) -> ReqSortedView<T> {
        ReqSortedView::from_inner(T::sorted_view(&self.inner))
    }

    fn require_non_empty(&self) -> Result<(), SketchError> {
        if self.is_empty() {
            return Err(SketchError::EmptySketch);
        }
        Ok(())
    }
}

impl<T: ReqItem> Clone for ReqSketch<T> {
    fn clone(&self) -> Self {
        Self {
            inner: T::clone_shim(&self.inner),
        }
    }
}

impl<T: ReqItem> Default for ReqSketch<T> {
    /// An empty sketch with `k = 12` and [`RankAccuracy::High`].
    fn default() -> Self {
        Self::new(DEFAULT_K, RankAccuracy::High).expect("DEFAULT_K is valid")
    }
}
