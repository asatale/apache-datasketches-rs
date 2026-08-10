//! Every generic Tuple type is `Send` but not `Sync`, matching the rest of
//! the crate. These tests move real sketches across threads rather than only
//! asserting the bound compiles, and `all_types_are_send_and_not_sync` uses
//! an autoref-specialisation probe (see `SyncProbe` below) to check the
//! `!Sync` half at runtime -- a plain `fn assert_sync<T: Sync>()` can only
//! express the positive.
//!
//! The per-component smoke tests each assert `Send` at compile time, but
//! nothing there actually moves a generic sketch between threads. Since the
//! summary is a boxed trait object owned by C++, a `Send` bound that compiles
//! but is unsound would only show up at runtime — which is what this file
//! reaches.

use apache_datasketches::tuple::generic::{
    CompactTupleSketch, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnion, TupleUnionBuilder,
};

#[derive(Clone, Debug, PartialEq)]
struct Tally {
    hits: u64,
    label: String,
}

impl TupleSummary for Tally {
    type Update = str;
    fn create(update: &str) -> Self {
        Tally {
            hits: 1,
            label: update.to_string(),
        }
    }
    fn union_combine(&mut self, other: &Self) {
        self.hits += other.hits;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.hits = self.hits.min(other.hits);
    }
}

/// Probe carrier for the `Sync`-detection trick below (autoref
/// specialisation). Duplicated from
/// `tuple_generic_compact_smoke_test.rs::SyncProbe` -- integration test
/// binaries are separate compilation units with no shared support module, so
/// there is nothing to `use` it from.
struct SyncProbe<T>(std::marker::PhantomData<T>);

/// Specialised arm: only applicable when `T: Sync`.
trait ProbeViaSync {
    fn is_sync(&self) -> bool;
}
impl<T: Sync> ProbeViaSync for &SyncProbe<T> {
    fn is_sync(&self) -> bool {
        true
    }
}

/// Fallback arm: applicable for every `T`, but one autoref step further away,
/// so the compiler only reaches it when the specialised arm does not apply.
trait ProbeViaFallback {
    fn is_sync(&self) -> bool;
}
impl<T> ProbeViaFallback for SyncProbe<T> {
    fn is_sync(&self) -> bool {
        false
    }
}

/// `true` iff `T: Sync`, via the autoref specialisation above.
///
/// This CANNOT be a plain generic `fn probe_is_sync<T>() -> bool` -- that
/// was tried and it silently always returns `false` (confirmed by the `u64`
/// positive control failing). Inside a generic function body, `T` carries no
/// `Sync` bound, so trait selection for `impl<T: Sync> ProbeViaSync for
/// &SyncProbe<T>` cannot apply regardless of what `T` is monomorphized to at
/// a given call site -- method resolution falls through to
/// `ProbeViaFallback` unconditionally, and `ProbeViaSync` goes dead (rustc
/// warns `trait ProbeViaSync is never used`, which is the tell). The
/// specialisation only works when the compiler can see the CONCRETE type at
/// the call site doing the resolution, so this is a macro that expands
/// inline at each site instead of a function.
macro_rules! probe_is_sync {
    ($ty:ty) => {{
        let probe = SyncProbe::<$ty>(std::marker::PhantomData);
        // The double borrow is what selects `ProbeViaSync` over
        // `ProbeViaFallback` via autoref when `$ty: Sync` holds; see
        // `compact_is_not_sync` in `tuple_generic_compact_smoke_test.rs` for
        // the full mechanism writeup.
        #[allow(clippy::needless_borrow)]
        (&&probe).is_sync()
    }};
}

/// Every generic type must be `Send` (the compile-time half -- `assert_send`
/// fails the *build*, not the test, if a bound is missing) and NOT `Sync`
/// (the runtime-checkable half, via the autoref probe above). A bare
/// `fn assert_sync<T: Sync>()` cannot express the negative at all, which is
/// why `all_types_are_send` previously asserted only `Send` and the doc
/// comment's "not Sync" claim went unchecked.
#[test]
fn all_types_are_send_and_not_sync() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleSketch<Tally>>();
    assert_send::<CompactTupleSketch<Tally>>();
    assert_send::<TupleUnion<Tally>>();
    assert_send::<TupleIntersection<Tally>>();
    assert_send::<TupleAnotB<Tally>>();

    assert!(
        !probe_is_sync!(TupleSketch<Tally>),
        "TupleSketch<S> must not be Sync"
    );
    assert!(
        !probe_is_sync!(CompactTupleSketch<Tally>),
        "CompactTupleSketch<S> must not be Sync"
    );
    assert!(
        !probe_is_sync!(TupleUnion<Tally>),
        "TupleUnion<S> must not be Sync"
    );
    assert!(
        !probe_is_sync!(TupleIntersection<Tally>),
        "TupleIntersection<S> must not be Sync"
    );
    assert!(
        !probe_is_sync!(TupleAnotB<Tally>),
        "TupleAnotB<S> must not be Sync"
    );

    // Positive control: without this, a probe broken to always answer
    // `false` would pass every assertion above vacuously. `u64` really is
    // `Sync`, so the probe must report `true` for it.
    assert!(
        probe_is_sync!(u64),
        "probe is broken: it does not detect Sync at all"
    );
}

#[test]
fn a_sketch_built_on_one_thread_is_usable_on_another() {
    let handle = std::thread::spawn(|| {
        let mut sketch: TupleSketch<Tally> = TupleSketchBuilder::new().build().unwrap();
        for i in 0..1_000u64 {
            sketch.update_u64(i, "worker");
        }
        sketch.compact(true)
    });
    let compact = handle.join().unwrap();

    // The heap-owning summaries must survive the move intact.
    assert_eq!(compact.entries().count(), 1_000);
    let bad: Vec<(u64, Tally)> = compact
        .entries()
        .filter(|(_, t)| {
            *t != Tally {
                hits: 1,
                label: "worker".to_string(),
            }
        })
        .collect();
    assert!(
        bad.is_empty(),
        "found entries that did not survive the cross-thread move intact: {bad:?}"
    );
}

#[test]
fn per_thread_sketches_merge_correctly() {
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            std::thread::spawn(move || {
                let mut sketch: TupleSketch<Tally> = TupleSketchBuilder::new().build().unwrap();
                for i in (t * 2_500)..((t + 1) * 2_500) {
                    sketch.update_u64(i, "shard");
                }
                sketch.compact(true)
            })
        })
        .collect();

    let mut union = TupleUnionBuilder::<Tally>::new().build().unwrap();
    for handle in handles {
        union.update(&handle.join().unwrap());
    }
    let result = union.get_result(true);
    let estimate = result.get_estimate();
    assert!(
        (estimate - 10_000.0).abs() < 10_000.0 * 0.03,
        "union estimate out of tolerance: {estimate}"
    );

    // The shards are disjoint (each thread owns its own key range), so no
    // combine callback fires here -- this is a clone-path check: every
    // heap-owning `label` must have survived the cross-thread `update` and
    // the union's internal copy unchanged, not just the cardinality.
    let entries: Vec<(u64, Tally)> = result.entries().collect();
    let bad: Vec<(u64, Tally)> = entries
        .into_iter()
        .filter(|(_, t)| {
            *t != Tally {
                hits: 1,
                label: "shard".to_string(),
            }
        })
        .collect();
    assert!(
        bad.is_empty(),
        "found union result entries with the wrong hits/label: {bad:?}"
    );
}
