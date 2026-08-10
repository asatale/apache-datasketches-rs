#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi as sketch_ffi, RawSummaryOps, RustSummary};
use apache_datasketches_sys::tuple_generic_jaccard::ffi as jac_ffi;
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

#[test]
fn identical_sketches_are_fully_similar() {
    let a = sketch(0..1000, 1);
    let b = sketch(0..1000, 1);
    let bounds = jac_ffi::tuple_generic_jaccard_sketch_sketch(&a, &b);
    assert_eq!(bounds.estimate, 1.0);
}

#[test]
fn disjoint_sketches_are_dissimilar() {
    let a = sketch(0..1000, 1);
    let b = sketch(2000..3000, 1);
    assert_eq!(
        jac_ffi::tuple_generic_jaccard_sketch_sketch(&a, &b).estimate,
        0.0
    );
}

#[test]
fn half_overlap_is_about_one_third_in_all_four_combinations() {
    let a = sketch(0..1000, 1);
    let b = sketch(500..1500, 1);
    let ca = a.compact(true);
    let cb = b.compact(true);
    for bounds in [
        jac_ffi::tuple_generic_jaccard_sketch_sketch(&a, &b),
        jac_ffi::tuple_generic_jaccard_sketch_compact(&a, &cb),
        jac_ffi::tuple_generic_jaccard_compact_sketch(&ca, &b),
        jac_ffi::tuple_generic_jaccard_compact_compact(&ca, &cb),
    ] {
        assert!((bounds.estimate - 1.0 / 3.0).abs() < 0.01);
        assert!(bounds.lower_bound <= bounds.estimate);
        assert!(bounds.estimate <= bounds.upper_bound);
    }
}

#[test]
fn summary_values_do_not_affect_the_result() {
    let baseline =
        jac_ffi::tuple_generic_jaccard_sketch_sketch(&sketch(0..1000, 1), &sketch(500..1500, 1));
    let different =
        jac_ffi::tuple_generic_jaccard_sketch_sketch(&sketch(0..1000, 99), &sketch(500..1500, -7));
    assert_eq!(baseline.estimate, different.estimate);
    assert_eq!(baseline.lower_bound, different.lower_bound);
    assert_eq!(baseline.upper_bound, different.upper_bound);
}
