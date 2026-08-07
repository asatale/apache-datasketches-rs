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
    for i in 0..500u64 {
        sketch.pin_mut().update_u64(i, &summary(3));
    }
    let compact = ffi::tuple_generic_sketch_compact(&sketch, true);
    assert!((compact.get_estimate() - 500.0).abs() < 1.0);
    assert_eq!(compact.entry_count(), 500);
    assert!(compact.is_ordered());

    // Every entry carries the summary that was inserted.
    for i in 0..compact.entry_count() {
        assert_eq!(value_of(&compact.entry_summary(i).unwrap()), 3);
    }
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
