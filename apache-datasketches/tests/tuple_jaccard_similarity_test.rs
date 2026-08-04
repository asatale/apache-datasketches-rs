// apache-datasketches/tests/tuple_jaccard_similarity_test.rs
//! Rust-authored coverage for [`array_of_doubles_jaccard_similarity`],
//! structured to mirror this repo's own `theta_jaccard_similarity_test.rs`
//! rather than any upstream C++ file.
//!
//! This is *not* a port of an upstream test file. Upstream ships exactly one
//! tuple test file,
//! `vendor/datasketches-cpp/tuple/test/array_of_doubles_sketch_test.cpp`,
//! and that file was already ported 1:1 to
//! `apache-datasketches/tests/array_of_doubles_sketch_test.rs`, which does
//! not exercise the Jaccard function at all. There is no second upstream
//! file for this crate to port for Jaccard coverage. Instead, the tests
//! below follow `theta_jaccard_similarity_test.rs`'s structure and naming
//! (empty operands, exact-mode full overlap/disjoint/identical, one-empty
//! variants, and estimation-mode similar/half-overlap regimes), adapted to
//! this family's per-entry `f64` value arrays.
//! `theta_jaccard_similarity_test`'s estimation-mode cases use `lg_k(12)`
//! with 10,000–15,000 keys; the same configuration is reused here.
//!
//! Additional tests beyond `theta_jaccard_similarity_test`'s shape check what
//! this family adds over Theta: that per-entry *values* do not affect the
//! Jaccard result (only keys matter, per
//! `array_of_doubles_jaccard_similarity`'s doc comment), and that the
//! confidence-interval bounds are correctly ordered
//! (`lower_bound <= estimate <= upper_bound`) in a regime where the interval
//! is genuinely non-degenerate.
use apache_datasketches::tuple::{array_of_doubles_jaccard_similarity, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder};

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
fn empty_sketches() {
    let a = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let b = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert!((bounds.lower_bound - 1.0).abs() < 1e-9);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
    assert!((bounds.upper_bound - 1.0).abs() < 1e-9);
}

#[test]
fn full_overlap_exact_mode() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 0..1000, &[1.0]);
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert!((bounds.lower_bound - 1.0).abs() < 1e-9);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
    assert!((bounds.upper_bound - 1.0).abs() < 1e-9);
}

#[test]
fn disjoint_exact_mode() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 1000..2000, &[1.0]);
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert!(bounds.lower_bound.abs() < 1e-9);
    assert!(bounds.estimate.abs() < 1e-9);
    assert!(bounds.upper_bound.abs() < 1e-9);
}

#[test]
fn first_empty_second_not() {
    let a = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let b = sketch(1, 0..1, &[1.0]);
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert!(bounds.estimate.abs() < 1e-9);
}

#[test]
fn second_empty_first_not() {
    let a = sketch(1, 0..1, &[1.0]);
    let b = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert!(bounds.estimate.abs() < 1e-9);
}

#[test]
fn exact_mode_identical() {
    let a = sketch(1, 0..100, &[1.0]);
    let bounds = array_of_doubles_jaccard_similarity(&a, &a).unwrap();
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
    assert!((bounds.lower_bound - 1.0).abs() < 1e-9);
    assert!((bounds.upper_bound - 1.0).abs() < 1e-9);
}

#[test]
fn estimation_mode_similar_sketches() {
    let mut a = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .build()
        .unwrap();
    let mut b = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .build()
        .unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i, &[1.0]).unwrap();
    }
    for i in 0..10_000u64 {
        b.update_u64(i, &[1.0]).unwrap();
    }
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert!(bounds.lower_bound <= bounds.estimate);
    assert!(bounds.estimate <= bounds.upper_bound);
    assert!((bounds.estimate - 1.0).abs() < 0.05);
}

#[test]
fn estimation_mode_half_overlap() {
    let mut a = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .build()
        .unwrap();
    let mut b = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .build()
        .unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i, &[1.0]).unwrap();
    }
    for i in 5_000..15_000u64 {
        b.update_u64(i, &[1.0]).unwrap();
    }
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert!(bounds.lower_bound <= bounds.estimate);
    assert!(bounds.estimate <= bounds.upper_bound);
    // union ~15000, intersection ~5000 => jaccard ~= 1/3
    assert!((bounds.estimate - (1.0 / 3.0)).abs() < 0.05);
    // Beyond theta: the interval is genuinely non-degenerate here, so
    // strict ordering is worth asserting explicitly rather than only via
    // the `<=` checks above.
    assert!(bounds.lower_bound < bounds.upper_bound);
}

/// Beyond theta: per-entry values must not affect the Jaccard result — only
/// keys matter, per `array_of_doubles_jaccard_similarity`'s doc comment. Two
/// sketches with identical keys but different per-entry values must produce
/// identical bounds to the case where both operands use the same values.
#[test]
fn values_do_not_affect_result() {
    let a_same_values = sketch(2, 0..10_000, &[1.0, 2.0]);
    let b_same_values = sketch(2, 5_000..15_000, &[1.0, 2.0]);
    let baseline = array_of_doubles_jaccard_similarity(&a_same_values, &b_same_values).unwrap();

    let a_diff_values = sketch(2, 0..10_000, &[1.0, 2.0]);
    let b_diff_values = sketch(2, 5_000..15_000, &[99.0, -5.0]);
    let with_different_values =
        array_of_doubles_jaccard_similarity(&a_diff_values, &b_diff_values).unwrap();

    assert_eq!(baseline, with_different_values);
}
