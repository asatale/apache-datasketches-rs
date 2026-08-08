use super::summary::{unerase, TupleSummary};
use crate::error::SketchError;
use apache_datasketches_sys::tuple_generic::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// An immutable snapshot of a generic Tuple sketch, produced by
/// [`TupleSketch::compact`](super::TupleSketch::compact) or by any set
/// operation's result.
///
/// Serialization is not part of this version; it is the subject of a
/// follow-up design.
pub struct CompactTupleSketch<S: TupleSummary> {
    pub(crate) inner: UniquePtr<sys::CompactTupleGenericSketchShim>,
    pub(crate) _marker: PhantomData<fn() -> S>,
}

// Sound because `S: Send` is a supertrait of `TupleSummary` and the erased
// box is `Box<dyn RawSummaryOps + Send>`. Deliberately NOT `Sync`: the C++
// shim lazily populates a `mutable` entry cache (`entries_`/`entries_built_`
// in `tuple_generic_compact_shim.h`) from otherwise-`const` methods, which is
// only safe without concurrent `&`-access to the same instance. Do not add a
// `Sync` impl, and do not wrap the shim in a `Sync` newtype.
unsafe impl<S: TupleSummary> Send for CompactTupleSketch<S> {}

impl<S: TupleSummary> CompactTupleSketch<S> {
    /// Wraps a shim produced by [`TupleSketch::compact`](super::TupleSketch::compact)
    /// or a set operation.
    ///
    /// `inner` should have every summary reachable from it be an `Adapter<S>`
    /// for this same `S` — that is, it should have originated from a sketch
    /// or operation typed over `S`.
    ///
    /// # Panics
    ///
    /// [`Self::entries`] calls `unerase` on every summary it reads back.
    /// `unerase`'s invariant panic is otherwise unreachable, but passing a
    /// shim whose summaries were erased for a different `S` makes it
    /// reachable, and the panic fires there instead of here.
    pub(crate) fn from_shim(inner: UniquePtr<sys::CompactTupleGenericSketchShim>) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }

    /// Returns the current estimate of the number of distinct keys.
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

    /// Returns `true` if this sketch represents an empty set.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the estimate is a statistical estimate rather than
    /// an exact count.
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

    /// Iterates the retained entries as `(hash, summary)` pairs.
    ///
    /// Each summary is cloned out of C++, so the items are owned. Ordered by
    /// hash if [`Self::is_ordered`] is `true`.
    pub fn entries(&self) -> impl Iterator<Item = (u64, S)> + '_ {
        (0..self.inner.entry_count()).map(move |i| {
            let hash = self
                .inner
                .entry_hash(i)
                .expect("index derived from entry_count is always in range");
            let summary = self
                .inner
                .entry_summary(i)
                .expect("index derived from entry_count is always in range");
            (hash, unerase::<S>(&summary))
        })
    }
}
