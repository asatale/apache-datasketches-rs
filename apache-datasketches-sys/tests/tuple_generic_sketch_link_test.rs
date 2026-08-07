#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi, RawSummaryOps, RustSummary};
use std::any::Any;
use std::sync::{Mutex, OnceLock};

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn summary(v: i64) -> RustSummary {
    RustSummary::new(Box::new(Sum(v)))
}

#[test]
fn construct_update_estimate() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &summary(1));
    }
    assert!((sketch.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(sketch.get_num_retained(), 1000);
    assert!(!sketch.is_empty());
}

#[test]
fn repeated_key_combines_rather_than_inserting() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for _ in 0..5 {
        sketch.pin_mut().update_u64(42, &summary(10));
    }
    assert_eq!(
        sketch.get_num_retained(),
        1,
        "same key must not insert twice"
    );
}

#[test]
fn invalid_lg_k_returns_err() {
    assert!(ffi::new_tuple_generic_sketch(4, 8, 1.0).is_err());
}

#[test]
fn invalid_resize_factor_returns_err() {
    assert!(ffi::new_tuple_generic_sketch(12, 3, 1.0).is_err());
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    sketch.pin_mut().update_u64(1, &summary(1));
    assert!(!sketch.is_empty());
    sketch.pin_mut().reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 0);
}

/// A summary type private to [`rehash_and_resize_preserve_summaries`] that
/// records, into its own dedicated observation vec, the value it held
/// immediately *before* each `union_combine` call. That is the only channel
/// this task exposes for reading a summary's state back out of the sketch
/// after it may have been move-constructed by a rehash or resize (Task 3
/// adds `entries()`, which would let a value be read directly).
///
/// The observation vec is a `static` scoped to this one summary type, which
/// is used by no other test, so this stays parallel-safe: no other test's
/// assertions read or depend on it.
struct ObservingSum(i64);

static OBSERVED_BEFORE_COMBINE: OnceLock<Mutex<Vec<i64>>> = OnceLock::new();

fn observed_before_combine() -> &'static Mutex<Vec<i64>> {
    OBSERVED_BEFORE_COMBINE.get_or_init(|| Mutex::new(Vec::new()))
}

impl RawSummaryOps for ObservingSum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(ObservingSum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        observed_before_combine().lock().unwrap().push(self.0);
        self.0 += other.as_any().downcast_ref::<ObservingSum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self
            .0
            .min(other.as_any().downcast_ref::<ObservingSum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn observing_summary(v: i64) -> RustSummary {
    RustSummary::new(Box::new(ObservingSum(v)))
}

/// At `lg_k = 5` the table starts small (32 slots) and 500 distinct keys force
/// several resize/rehash cycles well past the initial capacity. Every rehash
/// move-constructs each retained `DynSummary`, so this exercises the invariant
/// that `engaged()` stays true and `get()` stays valid across that move (the
/// move constructor added while hardening `DynSummary` in Task 1) — not just
/// that the move compiles, but that each summary's *value* survives the moves
/// correctly, which is verified by re-updating every key afterward and
/// reading back what `union_combine` saw as the pre-existing value.
#[test]
fn rehash_and_resize_preserve_summaries() {
    let mut sketch = ffi::new_tuple_generic_sketch(5, 2, 1.0).unwrap();
    for i in 0..500u64 {
        sketch.pin_mut().update_u64(i, &observing_summary(1));
    }
    assert!(
        sketch.is_estimation_mode(),
        "500 keys at lg_k=5 must trigger sampling/estimation"
    );
    // With `p = 1.0` and a fixed key set the estimate is deterministic, and it
    // lands at ~491 (1.8% low) — well inside the estimator's RSE at lg_k=5
    // (1/sqrt(32) ~= 17.7%). Keep the bound tight; the real assertions on move
    // correctness are the value checks below.
    assert!((sketch.get_estimate() - 500.0).abs() / 500.0 < 0.2);
    let retained_before_reupdate = sketch.get_num_retained();
    assert!(retained_before_reupdate > 0);
    assert!(!sketch.is_empty());

    // Re-update every key that was inserted before the resizes. This is a
    // theta sketch, so by now theta has been cut below 1 and only
    // `retained_before_reupdate` of the original 500 keys are still under
    // it; re-updating a key whose hash now exceeds theta is a no-op that
    // never touches a summary, so exactly `retained_before_reupdate`
    // union_combine calls are expected, one per surviving entry (which has
    // been sitting in the table, and moved during every rehash/resize since
    // its insertion). Observing the value union_combine saw *before*
    // combining proves that value is still exactly what was inserted, i.e.
    // the moves did not corrupt it.
    observed_before_combine().lock().unwrap().clear();
    for i in 0..500u64 {
        sketch.pin_mut().update_u64(i, &observing_summary(1));
    }
    let observed = observed_before_combine().lock().unwrap();
    assert_eq!(
        observed.len() as u32,
        retained_before_reupdate,
        "expected one union_combine call per surviving (under-theta) entry"
    );
    assert!(
        observed.iter().all(|&v| v == 1),
        "a summary's value was corrupted by a rehash/resize move: {observed:?}"
    );
}
