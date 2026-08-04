// apache-datasketches/tests/tuple_intersection_test.rs
//! Rust-authored coverage for [`ArrayOfDoublesIntersection`], structured to
//! mirror this repo's own `theta_intersection_test.rs` rather than any
//! upstream C++ file.
//!
//! This is *not* a port of an upstream test file. Upstream ships exactly one
//! tuple test file,
//! `vendor/datasketches-cpp/tuple/test/array_of_doubles_sketch_test.cpp`,
//! and that file was already ported 1:1 to
//! `apache-datasketches/tests/array_of_doubles_sketch_test.rs`. There is no
//! second upstream file for this crate to port for intersection coverage.
//! Instead, the tests below follow `theta_intersection_test.rs`'s structure
//! and naming (empty-intersection-before-update, empty/disjoint/identical
//! operands, no-retained-keys-via-sampling, exact-mode ordered-compact
//! inputs, and estimation-mode half-overlap/disjoint), adapted to this
//! family's per-entry `f64` value arrays and dropping the wrapped-compact
//! variant (this family has only `ArrayOfDoublesSketch` and
//! `CompactArrayOfDoublesSketch`, no wrapped-compact type).
//! `theta_intersection_test`'s equivalent estimation-mode cases only reach
//! k = 4096 (the default) with 10,000–20,000 keys; the same ranges are
//! reused here to stay in estimation mode while keeping the per-entry value
//! checks (new coverage, described below) tractable.
//!
//! Additional tests beyond `theta_intersection_test`'s shape check what this
//! family adds over Theta: the summed-per-index values of surviving entries
//! in estimation mode, and `get_result(false)` (unordered) parity with the
//! ordered result.
use apache_datasketches::tuple::{
    ArrayOfDoublesIntersection, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder,
};
use apache_datasketches::SketchError;

fn sketch(num_values: u8, keys: std::ops::Range<u64>, values: &[f64]) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    for key in keys {
        s.update_u64(key, values).unwrap();
    }
    s
}

#[test]
fn get_result_before_update_is_empty_intersection() {
    let isect = ArrayOfDoublesIntersection::new(1).unwrap();
    match isect.get_result(true) {
        Err(SketchError::EmptyIntersection) => {}
        Err(other) => panic!("expected EmptyIntersection, got {other:?}"),
        Ok(_) => panic!("expected EmptyIntersection, got Ok"),
    }
}

#[test]
fn has_result_false_before_any_update() {
    let isect = ArrayOfDoublesIntersection::new(1).unwrap();
    assert!(!isect.has_result());
}

#[test]
fn intersect_empty_with_nonempty_is_empty() {
    let a = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let b = sketch(1, 0..1, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&a).unwrap();
    isect.update(&b).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn intersect_disjoint_sets_is_empty() {
    let a = sketch(1, 0..5, &[1.0]);
    let b = sketch(1, 5..10, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&a).unwrap();
    isect.update(&b).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn intersect_identical_sets_preserves_estimate() {
    let a = sketch(1, 0..10, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&a).unwrap();
    isect.update(&a).unwrap();
    let result = isect.get_result(true).unwrap();
    assert_eq!(result.get_estimate(), 10.0);
}

#[test]
fn intersect_non_empty_no_retained_keys() {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(1)
        .p(0.001)
        .build()
        .unwrap();
    s.update_u64(1, &[5.0]).unwrap();
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&s).unwrap();
    let result = isect.get_result(true).unwrap();
    assert_eq!(result.get_num_retained(), 0);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_theta() - 0.001).abs() < 1e-10);
    assert_eq!(result.get_estimate(), 0.0);

    isect.update(&s).unwrap();
    let result = isect.get_result(true).unwrap();
    assert_eq!(result.get_num_retained(), 0);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_theta() - 0.001).abs() < 1e-10);
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn intersect_exact_half_overlap_ordered_compact_inputs() {
    let sketch1 = sketch(1, 0..1000, &[1.0]);
    let sketch2 = sketch(1, 500..1500, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&sketch1.compact(true)).unwrap();
    isect.update(&sketch2.compact(true)).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 500.0);
}

#[test]
fn intersect_exact_disjoint_ordered_compact_inputs() {
    let sketch1 = sketch(1, 0..1000, &[1.0]);
    let sketch2 = sketch(1, 1000..2000, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&sketch1.compact(true)).unwrap();
    isect.update(&sketch2.compact(true)).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn intersect_estimation_half_overlap_unordered() {
    let sketch1 = sketch(1, 0..10_000, &[1.0]);
    let sketch2 = sketch(1, 5_000..15_000, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&sketch1).unwrap();
    isect.update(&sketch2).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);
}

#[test]
fn intersect_estimation_half_overlap_ordered_compact_inputs() {
    let sketch1 = sketch(1, 0..10_000, &[1.0]);
    let sketch2 = sketch(1, 5_000..15_000, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&sketch1.compact(true)).unwrap();
    isect.update(&sketch2.compact(true)).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);
}

#[test]
fn intersect_estimation_disjoint_unordered() {
    let sketch1 = sketch(1, 0..10_000, &[1.0]);
    let sketch2 = sketch(1, 10_000..20_000, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&sketch1).unwrap();
    isect.update(&sketch2).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn intersect_estimation_disjoint_ordered_compact_inputs() {
    let sketch1 = sketch(1, 0..10_000, &[1.0]);
    let sketch2 = sketch(1, 10_000..20_000, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&sketch1.compact(true)).unwrap();
    isect.update(&sketch2.compact(true)).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);
}

/// Beyond theta: `get_result(false)` (unordered) must report the same
/// retained count and estimate as `get_result(true)` (ordered).
#[test]
fn intersect_get_result_unordered_matches_ordered() {
    let sketch1 = sketch(1, 0..10_000, &[1.0]);
    let sketch2 = sketch(1, 5_000..15_000, &[1.0]);
    let mut isect = ArrayOfDoublesIntersection::new(1).unwrap();
    isect.update(&sketch1).unwrap();
    isect.update(&sketch2).unwrap();

    let ordered = isect.get_result(true).unwrap();
    let unordered = isect.get_result(false).unwrap();
    assert!(ordered.is_estimation_mode());
    assert_eq!(unordered.get_num_retained(), ordered.get_num_retained());
    assert_eq!(unordered.get_estimate(), ordered.get_estimate());
    assert!(ordered.is_ordered());
    assert!(!unordered.is_ordered());
}

/// Beyond theta: verifies the surviving entries' *values* in estimation
/// mode. Both inputs assign the same values to every key ([1.0, 10.0]), and
/// only keys present in both inputs can survive an intersection at all, so
/// every surviving entry's values must be exactly the per-index sum,
/// [2.0, 20.0] — regardless of which entries theta's thresholding kept.
#[test]
fn intersect_estimation_mode_value_correctness() {
    let mut a = ArrayOfDoublesSketchBuilder::new()
        .num_values(2)
        .build()
        .unwrap();
    for i in 0..20_000u64 {
        a.update_u64(i, &[1.0, 10.0]).unwrap();
    }
    let mut b = ArrayOfDoublesSketchBuilder::new()
        .num_values(2)
        .build()
        .unwrap();
    for i in 10_000..30_000u64 {
        b.update_u64(i, &[1.0, 10.0]).unwrap();
    }

    let mut isect = ArrayOfDoublesIntersection::new(2).unwrap();
    isect.update(&a).unwrap();
    isect.update(&b).unwrap();
    let result = isect.get_result(true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert_eq!(result.get_num_values(), 2);
    assert!((result.get_estimate() - 10_000.0).abs() < 10_000.0 * 0.05);

    let mut count = 0;
    for (_, values) in result.entries() {
        assert_eq!(values, [2.0, 20.0]);
        count += 1;
    }
    assert!(count > 0, "expected at least one surviving entry");
}
