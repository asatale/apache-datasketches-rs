#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi as sketch_ffi, RawSummaryOps, RustSummary};
use apache_datasketches_sys::tuple_generic_intersection::ffi as isect_ffi;
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
fn intersection_half_overlap() {
    let mut i = isect_ffi::new_tuple_generic_intersection();
    assert!(!i.has_result());
    i.pin_mut().update_with_sketch(&sketch(0..1000, 1));
    i.pin_mut().update_with_sketch(&sketch(500..1500, 1));
    assert!(i.has_result());
    assert_eq!(i.get_result(true).unwrap().get_estimate(), 500.0);
}

// The operand order here is load-bearing. `Sum::intersection_combine` is
// `min`, so the retained value must be the *second* operand for the assertion
// to distinguish all three outcomes:
//
//   32 -> the callback never ran (retained summary left untouched),
//   42 -> `rust_summary_union_combine` was invoked (the trampolines crossed),
//   10 -> `rust_summary_intersection_combine` ran, which is correct.
//
// Feeding 10 before 32 would make "correct" and "never ran" both produce 10.
#[test]
fn intersection_uses_the_intersection_trampoline_not_the_union_one() {
    let mut i = isect_ffi::new_tuple_generic_intersection();
    i.pin_mut().update_with_sketch(&sketch(7..8, 32));
    i.pin_mut().update_with_sketch(&sketch(7..8, 10));
    let result = i.get_result(true).unwrap();
    assert_eq!(result.entry_count(), 1);
    assert_eq!(
        value_at(&result, 0),
        10,
        "min, not sum -- 42 means the trampolines are crossed, 32 means neither ran"
    );
}

#[test]
fn intersection_accepts_compact_input() {
    let mut i = isect_ffi::new_tuple_generic_intersection();
    i.pin_mut().update_with_sketch(&sketch(7..8, 32));
    i.pin_mut()
        .update_with_compact(&sketch(7..8, 10).compact(true));
    let result = i.get_result(true).unwrap();
    assert_eq!(result.entry_count(), 1);
    assert_eq!(value_at(&result, 0), 10);
}

// Disjoint operands are a *defined* state with an empty result -- distinct
// from the no-operand state below, which upstream treats as the undefined
// infinite universe.
#[test]
fn disjoint_operands_have_a_result_that_is_empty() {
    let mut i = isect_ffi::new_tuple_generic_intersection();
    i.pin_mut().update_with_sketch(&sketch(0..100, 1));
    i.pin_mut().update_with_sketch(&sketch(100..200, 1));
    assert!(i.has_result());
    let result = i.get_result(true).unwrap();
    assert_eq!(result.entry_count(), 0);
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn get_result_without_update_is_err() {
    let i = isect_ffi::new_tuple_generic_intersection();
    assert!(!i.has_result());
    assert!(i.get_result(true).is_err());
}
