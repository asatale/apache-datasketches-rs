use super::checks::{check_item, check_rank, check_split_points};
use super::KllItem;
use crate::error::SketchError;
use cxx::UniquePtr;

/// One entry of a [`KllSortedView`]: a retained item, its weight, and the
/// total weight of all entries up to and including this one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SortedViewEntry<T> {
    /// The retained item.
    pub item: T,
    /// How many input items this entry stands for.
    pub weight: u64,
    /// The sum of `weight` over this entry and every entry before it; the
    /// last entry's equals the sketch's `n`.
    pub cumulative_weight: u64,
}

/// An immutable, sorted snapshot of a [`KllSketch`](super::KllSketch)'s
/// retained items, built by [`KllSketch::sorted_view`](super::KllSketch::sorted_view).
///
/// It answers the same rank, quantile, PMF and CDF queries as the sketch,
/// with identical results, and does not change when the sketch is later
/// updated. Unlike the sketch it is `Sync`, so it is the way to share query
/// access between threads (e.g. in an `Arc`).
pub struct KllSortedView<T: KllItem> {
    inner: UniquePtr<T::View>,
}

// SAFETY: upstream quantiles_sorted_view has no mutable state — every query
// is a const binary search over entries fixed at construction — so shared
// and cross-thread access are both sound. The view is also independent of
// the sketch's lifetime (hence no lifetime parameter) because upstream
// quantiles_sorted_view stores arithmetic items (f64/f32) by value; for
// non-arithmetic types it would store pointers into the sketch, so this impl
// and the absence of a lifetime are only sound for arithmetic `KllItem`s.
unsafe impl<T: KllItem> Send for KllSortedView<T> {}
unsafe impl<T: KllItem> Sync for KllSortedView<T> {}

impl<T: KllItem> KllSortedView<T> {
    pub(crate) fn from_inner(inner: UniquePtr<T::View>) -> Self {
        Self { inner }
    }

    /// The number of entries (the sketch's retained item count).
    pub fn len(&self) -> usize {
        T::view_size(&self.inner)
    }

    /// Returns `true` if the view has no entries (built from an empty sketch).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Same as [`KllSketch::get_quantile`](super::KllSketch::get_quantile).
    pub fn get_quantile(&self, rank: f64, inclusive: bool) -> Result<T, SketchError> {
        self.require_non_empty()?;
        check_rank(rank)?;
        Ok(T::view_get_quantile(&self.inner, rank, inclusive)?)
    }

    /// Same as [`KllSketch::get_rank`](super::KllSketch::get_rank).
    pub fn get_rank(&self, item: T, inclusive: bool) -> Result<f64, SketchError> {
        self.require_non_empty()?;
        check_item(item)?;
        Ok(T::view_get_rank(&self.inner, item, inclusive)?)
    }

    /// Same as [`KllSketch::get_pmf`](super::KllSketch::get_pmf).
    pub fn get_pmf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError> {
        self.require_non_empty()?;
        check_split_points(split_points)?;
        Ok(T::view_get_pmf(&self.inner, split_points, inclusive)?)
    }

    /// Same as [`KllSketch::get_cdf`](super::KllSketch::get_cdf).
    pub fn get_cdf(&self, split_points: &[T], inclusive: bool) -> Result<Vec<f64>, SketchError> {
        self.require_non_empty()?;
        check_split_points(split_points)?;
        Ok(T::view_get_cdf(&self.inner, split_points, inclusive)?)
    }

    /// The entries in ascending item order. Copied out once when called, so
    /// the iterator does not borrow the view.
    pub fn iter(&self) -> impl Iterator<Item = SortedViewEntry<T>> {
        let items = T::view_items(&self.inner);
        let weights = T::view_weights(&self.inner);
        let cumulative = T::view_cumulative_weights(&self.inner);
        items
            .into_iter()
            .zip(weights)
            .zip(cumulative)
            .map(|((item, weight), cumulative_weight)| SortedViewEntry {
                item,
                weight,
                cumulative_weight,
            })
    }

    fn require_non_empty(&self) -> Result<(), SketchError> {
        if self.is_empty() {
            return Err(SketchError::EmptySketch);
        }
        Ok(())
    }
}
