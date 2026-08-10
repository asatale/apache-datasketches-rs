#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi as sketch_ffi, RawSummaryOps, RustSummary};
use apache_datasketches_sys::tuple_generic_union::ffi as union_ffi;
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

fn sketch(
    keys: std::ops::Range<u64>,
    v: i64,
) -> cxx::UniquePtr<sketch_ffi::TupleGenericSketchShim> {
    let mut s = sketch_ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for key in keys {
        s.pin_mut()
            .update_u64(key, &RustSummary::new(Box::new(Sum(v))));
    }
    s
}

fn value_at(c: &sketch_ffi::CompactTupleGenericSketchShim, i: u32) -> i64 {
    c.entry_summary(i)
        .unwrap()
        .ops()
        .as_any()
        .downcast_ref::<Sum>()
        .unwrap()
        .0
}

#[test]
fn union_half_overlap() {
    let a = sketch(0..1000, 1);
    let b = sketch(500..1500, 1);
    let mut u = union_ffi::new_tuple_generic_union(12, 8, 1.0).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_sketch(&b);
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);
}

#[test]
fn union_sums_summaries_on_collision() {
    let a = sketch(7..8, 10);
    let b = sketch(7..8, 32);
    let mut u = union_ffi::new_tuple_generic_union(12, 8, 1.0).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_sketch(&b);
    let result = u.get_result(true);
    assert_eq!(result.entry_count(), 1);
    assert_eq!(value_at(&result, 0), 42, "union policy must sum");
}

// Two operands with DISTINCT values, the second of them compact, so the
// compact path's summaries are asserted by value and not only by cardinality.
// With a single operand there is no collision and `union_combine` never runs
// at all, which is what this test used to do.
//
//   50 x 42 -- the overlap, union_combine(10, 32)
//   50 x 10 -- keys 0..50, only in the sketch operand
//   50 x 32 -- keys 100..150, only in the COMPACT operand
//
// The 32-group proves the compact operand's summaries crossed, not just its
// keys. (A dead combine and a cross-wire to `min` both read 10 on the
// overlap: both are caught, they are not distinguished.)
#[test]
fn union_accepts_compact_input_and_resets() {
    let a = sketch(0..100, 10);
    let compact = sketch(50..150, 32).compact(true);
    let mut u = union_ffi::new_tuple_generic_union(12, 8, 1.0).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_compact(&compact);
    let result = u.get_result(true);
    assert_eq!(result.get_estimate(), 150.0);

    let mut got: Vec<i64> = (0..result.entry_count())
        .map(|i| value_at(&result, i))
        .collect();
    assert_eq!(got.len(), 150);
    got.sort_unstable();
    let mut expected: Vec<i64> = [vec![10i64; 50], vec![32; 50], vec![42; 50]].concat();
    expected.sort_unstable();
    assert_eq!(
        got, expected,
        "expected 50 untouched sketch-operand summaries (10), 50 untouched \
         compact-operand summaries (32) and 50 union-combined ones (42)"
    );

    u.pin_mut().reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn invalid_config_returns_err() {
    assert!(union_ffi::new_tuple_generic_union(4, 8, 1.0).is_err());
    assert!(union_ffi::new_tuple_generic_union(12, 3, 1.0).is_err());
}
