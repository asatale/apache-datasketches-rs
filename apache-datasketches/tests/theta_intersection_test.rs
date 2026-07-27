// apache-datasketches/tests/theta_intersection_test.rs
//! Ported from theta/test/theta_intersection_test.cpp (tag 5.2.0).
//!
//! Upstream has 13 `TEST_CASE`s: "invalid", "empty", "non empty no retained
//! keys", "exact mode half overlap unordered", "exact mode half overlap
//! ordered", "exact mode disjoint unordered", "exact mode disjoint ordered",
//! "estimation mode half overlap unordered", "estimation mode half overlap
//! ordered", "estimation mode half overlap ordered wrapped compact",
//! "estimation mode disjoint unordered", "estimation mode disjoint ordered",
//! and "seed mismatch".
//!
//! `get_result_before_update_is_empty_intersection` ports "invalid"'s
//! `get_result()`-throws-before-`update()` assertion.
//! `has_result_false_before_any_update` ports "invalid"'s `has_result()`
//! assertion.
//! `intersect_empty_with_nonempty_is_empty` and
//! `intersect_identical_sets_preserves_estimate` cover the same territory as
//! upstream's "empty" case (intersecting with an empty operand, and
//! intersecting a set with itself/an identical copy).
//! `intersect_disjoint_sets_is_empty` ports the disjoint-set behavior
//! asserted by "exact mode disjoint unordered"/"exact mode disjoint
//! ordered" (using plain `ThetaSketch` operands; the ordered/compact variant
//! is ported separately below).
//!
//! `intersect_non_empty_no_retained_keys` ports "non empty no retained
//! keys". `intersect_exact_half_overlap_ordered_compact_inputs` and
//! `intersect_exact_disjoint_ordered_compact_inputs` port "exact mode half
//! overlap ordered" and "exact mode disjoint ordered" (compact/ordered
//! operands rather than raw update sketches). `intersect_estimation_half_
//! overlap_unordered`, `_ordered_compact_inputs`, and `_wrapped_compact_
//! inputs` port "estimation mode half overlap unordered/ordered/ordered
//! wrapped compact". `intersect_estimation_disjoint_unordered` and
//! `_ordered_compact_inputs` port "estimation mode disjoint
//! unordered/ordered".
//!
//! Not ported: upstream's "seed mismatch" — this crate never exposes a
//! seed parameter (every sketch always uses upstream's `DEFAULT_SEED`), so
//! there is no reachable equivalent through the public API. `ThetaInput`
//! dispatch parity across `ThetaSketch`/`CompactThetaSketch`/
//! `WrappedCompactThetaSketch` beyond the ordered/wrapped cases above is
//! already exhaustively covered by `theta_input_dispatch_test.rs`'s
//! `intersection_accepts_all_nine_combinations`.
use apache_datasketches::theta::{ThetaIntersection, ThetaSketchBuilder};
use apache_datasketches::SketchError;

#[test]
fn get_result_before_update_is_empty_intersection() {
    let isect = ThetaIntersection::new();
    match isect.get_result(true) {
        Err(SketchError::EmptyIntersection) => {}
        Err(other) => panic!("expected EmptyIntersection, got {:?}", other),
        Ok(_) => panic!("expected EmptyIntersection, got Ok"),
    }
}

#[test]
fn has_result_false_before_any_update() {
    let isect = ThetaIntersection::new();
    assert!(!isect.has_result());
}

#[test]
fn intersect_empty_with_nonempty_is_empty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    b.update_u64(1);
    let mut isect = ThetaIntersection::new();
    isect.update(&a);
    isect.update(&b);
    let result = isect.get_result(true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn intersect_disjoint_sets_is_empty() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..5u64 {
        a.update_u64(i);
    }
    for i in 5..10u64 {
        b.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&a);
    isect.update(&b);
    let result = isect.get_result(true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn intersect_identical_sets_preserves_estimate() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10u64 {
        a.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&a);
    isect.update(&a);
    let result = isect.get_result(true).unwrap();
    assert_eq!(result.get_estimate(), 10.0);
}

#[test]
fn intersect_non_empty_no_retained_keys() {
    let mut sketch = ThetaSketchBuilder::new().p(0.001).build().unwrap();
    sketch.update_u64(1);
    let mut isect = ThetaIntersection::new();
    isect.update(&sketch);
    let result = isect.get_result(true).unwrap();
    assert_eq!(result.get_num_retained(), 0);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_theta() - 0.001).abs() < 1e-10);
    assert_eq!(result.get_estimate(), 0.0);

    isect.update(&sketch);
    let result = isect.get_result(true).unwrap();
    assert_eq!(result.get_num_retained(), 0);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_theta() - 0.001).abs() < 1e-10);
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn intersect_exact_half_overlap_ordered_compact_inputs() {
    let mut sketch1 = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..1000u64 {
        sketch1.update_u64(i);
    }
    let mut sketch2 = ThetaSketchBuilder::new().build().unwrap();
    for i in 500..1500u64 {
        sketch2.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&sketch1.compact(true));
    isect.update(&sketch2.compact(true));
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 500.0);
}

#[test]
fn intersect_exact_disjoint_ordered_compact_inputs() {
    let mut sketch1 = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..1000u64 {
        sketch1.update_u64(i);
    }
    let mut sketch2 = ThetaSketchBuilder::new().build().unwrap();
    for i in 1000..2000u64 {
        sketch2.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&sketch1.compact(true));
    isect.update(&sketch2.compact(true));
    let result = isect.get_result(true).unwrap();
    assert!(result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn intersect_estimation_half_overlap_unordered() {
    let mut sketch1 = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        sketch1.update_u64(i);
    }
    let mut sketch2 = ThetaSketchBuilder::new().build().unwrap();
    for i in 5_000..15_000u64 {
        sketch2.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&sketch1);
    isect.update(&sketch2);
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);
}

#[test]
fn intersect_estimation_half_overlap_ordered_compact_inputs() {
    let mut sketch1 = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        sketch1.update_u64(i);
    }
    let mut sketch2 = ThetaSketchBuilder::new().build().unwrap();
    for i in 5_000..15_000u64 {
        sketch2.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&sketch1.compact(true));
    isect.update(&sketch2.compact(true));
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);
}

#[test]
fn intersect_estimation_half_overlap_wrapped_compact_inputs() {
    use apache_datasketches::theta::WrappedCompactThetaSketch;

    let mut sketch1 = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        sketch1.update_u64(i);
    }
    let bytes1 = sketch1.compact(true).serialize_compact();

    let mut sketch2 = ThetaSketchBuilder::new().build().unwrap();
    for i in 5_000..15_000u64 {
        sketch2.update_u64(i);
    }
    let bytes2 = sketch2.compact(true).serialize_compact();

    let mut isect = ThetaIntersection::new();
    isect.update(&WrappedCompactThetaSketch::wrap(&bytes1).unwrap());
    isect.update(&WrappedCompactThetaSketch::wrap(&bytes2).unwrap());
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);
}

#[test]
fn intersect_estimation_disjoint_unordered() {
    let mut sketch1 = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        sketch1.update_u64(i);
    }
    let mut sketch2 = ThetaSketchBuilder::new().build().unwrap();
    for i in 10_000..20_000u64 {
        sketch2.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&sketch1);
    isect.update(&sketch2);
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn intersect_estimation_disjoint_ordered_compact_inputs() {
    let mut sketch1 = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        sketch1.update_u64(i);
    }
    let mut sketch2 = ThetaSketchBuilder::new().build().unwrap();
    for i in 10_000..20_000u64 {
        sketch2.update_u64(i);
    }
    let mut isect = ThetaIntersection::new();
    isect.update(&sketch1.compact(true));
    isect.update(&sketch2.compact(true));
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);
}
