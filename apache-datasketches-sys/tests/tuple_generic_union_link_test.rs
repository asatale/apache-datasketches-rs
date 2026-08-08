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

#[test]
fn union_accepts_compact_input_and_resets() {
    let a = sketch(0..100, 1);
    let compact = a.compact(true);
    let mut u = union_ffi::new_tuple_generic_union(12, 8, 1.0).unwrap();
    u.pin_mut().update_with_compact(&compact);
    assert_eq!(u.get_result(true).get_estimate(), 100.0);
    u.pin_mut().reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn invalid_config_returns_err() {
    assert!(union_ffi::new_tuple_generic_union(4, 8, 1.0).is_err());
    assert!(union_ffi::new_tuple_generic_union(12, 3, 1.0).is_err());
}
