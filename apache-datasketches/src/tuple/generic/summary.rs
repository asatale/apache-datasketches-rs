use apache_datasketches_sys::tuple_generic::{RawSummaryOps, RustSummary};
use std::any::Any;

/// A user-defined per-entry summary for [`TupleSketch`](super::TupleSketch).
///
/// Implement this on your own type to get a Tuple sketch that carries it.
///
/// # Panics and the FFI boundary
///
/// [`union_combine`](Self::union_combine),
/// [`intersection_combine`](Self::intersection_combine), and
/// [`Clone::clone`] are invoked by C++. A panic cannot unwind across that
/// boundary, and the underlying C++ combine has no way to report failure and
/// roll back an insert, so a panic in any of those three **aborts the
/// process** after printing a diagnostic. Make them total.
///
/// [`create`](Self::create) is different: it runs entirely in Rust before any
/// C++ call, so a panic there is an ordinary Rust panic that propagates to
/// your caller.
pub trait TupleSummary: Clone + Send + 'static {
    /// The value passed to the sketch's `update_*` methods.
    ///
    /// May be unsized — `type Update = str` and `type Update = [f64]` both
    /// work, so callers need not allocate to update.
    type Update: ?Sized;

    /// Builds a summary from a single update value.
    fn create(update: &Self::Update) -> Self;

    /// Merges `other` into `self` with union semantics. Used both when a key
    /// is updated more than once and when two sketches are unioned.
    fn union_combine(&mut self, other: &Self);

    /// Merges `other` into `self` with intersection semantics.
    ///
    /// There is deliberately no default: upstream notes that no intersection
    /// policy is sensible in general, and silently reusing union semantics
    /// would be a correctness trap. If union semantics *are* what you want,
    /// call `self.union_combine(other)` here explicitly.
    fn intersection_combine(&mut self, other: &Self);
}

/// Erases a `TupleSummary` to the sys crate's `RawSummaryOps`.
///
/// Private: users never name this. It exists because cxx requires the opaque
/// `extern "Rust"` type to live in the crate that declares the bridge, so the
/// ergonomic trait here has to be adapted to the minimal trait there.
pub(crate) struct Adapter<S: TupleSummary> {
    value: S,
}

impl<S: TupleSummary> Adapter<S> {
    fn new(value: S) -> Self {
        Self { value }
    }

    /// Recovers `&S` from an erased operand.
    ///
    /// The typed façade makes a mismatch unreachable: `TupleSketch<S>` (via
    /// [`erase`]) is the only producer of erased summaries in this crate, so
    /// every operand a combine callback sees was wrapped as `Adapter<S>` for
    /// the same `S`. A failure here means an internal invariant broke, not
    /// user error.
    fn downcast(other: &dyn RawSummaryOps) -> &S {
        match other.as_any().downcast_ref::<Adapter<S>>() {
            Some(adapter) => &adapter.value,
            None => panic!(
                "apache-datasketches internal invariant violated: a generic Tuple \
                 summary of a different concrete type reached a combine callback. \
                 This should be impossible through the public API; please report it."
            ),
        }
    }
}

impl<S: TupleSummary> RawSummaryOps for Adapter<S> {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Adapter::new(self.value.clone()))
    }

    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        let other = Self::downcast(other);
        self.value.union_combine(other);
    }

    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        let other = Self::downcast(other);
        self.value.intersection_combine(other);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Wraps a user summary in the opaque type that crosses the FFI boundary.
pub(crate) fn erase<S: TupleSummary>(value: S) -> RustSummary {
    RustSummary::new(Box::new(Adapter::new(value)))
}

/// Recovers an owned `S` from a summary that crossed back from C++.
///
/// [`erase`] is the sole producer of erased summaries, and it is only ever
/// called with `Adapter<S>` for the `S` of the calling sketch, so a mismatch
/// here means an internal invariant broke, not user error.
pub(crate) fn unerase<S: TupleSummary>(summary: &RustSummary) -> S {
    match summary.ops().as_any().downcast_ref::<Adapter<S>>() {
        Some(adapter) => adapter.value.clone(),
        None => panic!(
            "apache-datasketches internal invariant violated: a generic Tuple summary \
             of a different concrete type was returned from the sketch. This should be \
             impossible through the public API; please report it."
        ),
    }
}
