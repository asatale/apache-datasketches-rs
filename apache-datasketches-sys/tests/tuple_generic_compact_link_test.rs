#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi, RawSummaryOps, RustSummary};
use std::any::Any;

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

fn value_of(s: &RustSummary) -> i64 {
    s.ops().as_any().downcast_ref::<Sum>().unwrap().0
}

#[test]
fn compact_preserves_estimate_and_entries() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    // Each key gets a distinct summary value (key * 7), not a shared
    // constant: compaction is the first thing that exercises DynSummary's
    // copy constructor and the clone trampoline, so a summary getting
    // shuffled onto the wrong hash is exactly the new risk this shim
    // introduces. A constant value across every entry cannot detect that.
    for i in 0..500u64 {
        sketch.pin_mut().update_u64(i, &summary(i as i64 * 7));
    }
    let compact = ffi::tuple_generic_sketch_compact(&sketch, true);
    assert!((compact.get_estimate() - 500.0).abs() < 1.0);
    assert_eq!(compact.entry_count(), 500);
    // entry_count() and get_num_retained() are independently-computed
    // readings (entries().size() vs. the underlying sketch's own counter);
    // pin them against each other so neither can silently under/over-report
    // alone.
    assert_eq!(compact.entry_count(), compact.get_num_retained());
    assert!(compact.is_ordered());

    // Hash order is murmur3, so which hash a given key landed on can't be
    // recovered here -- but the *multiset* of summary values must be
    // unchanged by compaction. This still catches a value corrupted or
    // duplicated in the clone/shuffle path even without per-entry pairing.
    let mut got: Vec<i64> = (0..compact.entry_count())
        .map(|i| value_of(&compact.entry_summary(i).unwrap()))
        .collect();
    got.sort_unstable();
    let mut expected: Vec<i64> = (0..500u64).map(|i| i as i64 * 7).collect();
    expected.sort_unstable();
    assert_eq!(got, expected);
}

#[test]
fn entry_access_out_of_range_is_err() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    sketch.pin_mut().update_u64(1, &summary(1));
    let compact = ffi::tuple_generic_sketch_compact(&sketch, true);
    // entries().at(index) throws std::out_of_range, which cxx turns into
    // Result::Err because entry_hash/entry_summary are declared to return
    // Result -- prove that really happens rather than a deterministic abort.
    assert!(compact.entry_hash(compact.entry_count()).is_err());
    assert!(compact.entry_summary(compact.entry_count()).is_err());
}

#[test]
fn ordered_entry_hashes_are_sorted() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for i in 0..200u64 {
        sketch.pin_mut().update_u64(i, &summary(1));
    }
    let compact = ffi::tuple_generic_sketch_compact(&sketch, true);
    let hashes: Vec<u64> = (0..compact.entry_count())
        .map(|i| compact.entry_hash(i).unwrap())
        .collect();
    let mut sorted = hashes.clone();
    sorted.sort_unstable();
    assert_eq!(hashes, sorted);
}

#[test]
fn repeated_updates_are_unioned_in_the_compact_form() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for _ in 0..4 {
        sketch.pin_mut().update_u64(9, &summary(5));
    }
    let compact = ffi::tuple_generic_sketch_compact(&sketch, true);
    assert_eq!(compact.entry_count(), 1);
    assert_eq!(
        value_of(&compact.entry_summary(0).unwrap()),
        20,
        "4 x 5 summed"
    );
}
