use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};
use std::sync::{Mutex, OnceLock};

/// Sums on union, takes the minimum on intersection. This shape alone does
/// not prove which callback ran when a key repeats — `get_num_retained() ==
/// 1` after repeated updates is upstream theta-sketch dedup and would hold
/// even if `union_combine` were never called at all. [`ObservingSum`] below
/// is the type that actually verifies the callback ran with the right
/// operands. `intersection_combine` is not exercised by any test in this
/// file: its only callers are `TupleUnion`/`TupleIntersection`
/// set-operations, which do not exist until a later task.
#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

/// A summary type private to [`union_combine_runs_with_correct_operands`].
/// It records, into its own dedicated observation vec, the `(self, other)`
/// pair `union_combine` was called with on each invocation. This is the only
/// way at this layer to prove `union_combine` actually ran with the values
/// the API contract promises — `TupleSketch` exposes no `entries()`-style
/// accessor yet (that arrives in a later task), so a summary's resulting
/// value cannot otherwise be read back out of the sketch.
///
/// The observation vec is a `static` scoped to this one summary type, used
/// by no other test, so this stays parallel-safe: no other test's
/// assertions read or depend on it.
#[derive(Clone, Debug, PartialEq)]
struct ObservingSum(i64);

static UNION_COMBINE_CALLS: OnceLock<Mutex<Vec<(i64, i64)>>> = OnceLock::new();

fn union_combine_calls() -> &'static Mutex<Vec<(i64, i64)>> {
    UNION_COMBINE_CALLS.get_or_init(|| Mutex::new(Vec::new()))
}

impl TupleSummary for ObservingSum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        ObservingSum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        union_combine_calls()
            .lock()
            .unwrap()
            .push((self.0, other.0));
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

#[test]
fn construct_update_estimate() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i, &1);
    }
    assert!((sketch.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(sketch.get_num_retained(), 1000);
}

#[test]
fn repeated_key_unions_its_summaries() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for _ in 0..4 {
        sketch.update_u64(7, &10);
    }
    assert_eq!(sketch.get_num_retained(), 1);
}

/// Verifies the callback core's central behaviour: that a repeated key's
/// `TupleSummary::union_combine` actually runs, in the right order, with the
/// right operands — not just that upstream's theta-sketch dedup collapses
/// the key to one retained entry (`repeated_key_unions_its_summaries`
/// above), which would hold even if the combine callback were never invoked
/// or were cross-wired to `intersection_combine`.
#[test]
fn union_combine_runs_with_correct_operands() {
    let mut sketch: TupleSketch<ObservingSum> = TupleSketchBuilder::new().build().unwrap();
    union_combine_calls().lock().unwrap().clear();

    // First update for key 7 only calls S::create; no combine yet.
    sketch.update_u64(7, &10);
    assert!(union_combine_calls().lock().unwrap().is_empty());

    // Each subsequent update for the same key combines the freshly created
    // summary (value 10) into the one already stored.
    sketch.update_u64(7, &10);
    sketch.update_u64(7, &10);
    sketch.update_u64(7, &10);

    let calls = union_combine_calls().lock().unwrap();
    assert_eq!(
        *calls,
        vec![(10, 10), (20, 10), (30, 10)],
        "union_combine must run once per repeat, combining the running total \
         (10, 20, then 30) with the newly created summary (10)"
    );
}

/// `TupleSketchBuilder<S>`'s `Debug`/`Clone`/`Copy` impls are hand-written
/// precisely so they carry no `S: Debug + Clone + Copy` bound. `Sum` is
/// `Clone + Debug` but deliberately *not* `Copy`, so re-deriving those impls
/// would break this test at compile time — which is the only way to catch it,
/// since a derive is only rejected at a use site like this one.
#[test]
// The redundant `.clone()` on a `Copy` type is the point: it pins the `Clone`
// impl's bounds, which `Copy` alone would not exercise.
#[allow(clippy::clone_on_copy)]
fn builder_is_copy_clone_debug_for_a_non_copy_summary() {
    let builder = TupleSketchBuilder::<Sum>::new().lg_k(8);
    let first = builder;
    let second = builder; // still usable: Copy, not moved
    let _ = format!("{builder:?}");
    assert!(first.build().is_ok());
    assert!(second.clone().build().is_ok());
}

#[test]
fn invalid_config_is_err() {
    assert!(TupleSketchBuilder::<Sum>::new().lg_k(4).build().is_err());
    assert!(TupleSketchBuilder::<Sum>::new().p(0.0).build().is_err());
    assert!(TupleSketchBuilder::<Sum>::new().p(1.5).build().is_err());
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    sketch.update_u64(1, &1);
    assert!(!sketch.is_empty());
    sketch.reset();
    assert!(sketch.is_empty());
}

#[test]
fn every_update_key_type_works() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    sketch.update_u64(1, &1);
    sketch.update_i64(2, &1);
    sketch.update_u32(3, &1);
    sketch.update_i32(4, &1);
    sketch.update_u16(5, &1);
    sketch.update_i16(6, &1);
    sketch.update_u8(7, &1);
    sketch.update_i8(8, &1);
    sketch.update_f64(9.0, &1);
    sketch.update_str("ten", &1);
    sketch.update_bytes(b"eleven", &1);
    assert_eq!(sketch.get_num_retained(), 11);
}

#[test]
fn sketch_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleSketch<Sum>>();
}
