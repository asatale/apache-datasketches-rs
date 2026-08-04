// apache-datasketches/tests/tuple_a_not_b_test.rs
//! Rust-authored coverage for [`ArrayOfDoublesAnotB`], structured to mirror
//! this repo's own `theta_a_not_b_test.rs` rather than any upstream C++ file.
//!
//! This is *not* a port of an upstream test file. Upstream ships exactly one
//! tuple test file,
//! `vendor/datasketches-cpp/tuple/test/array_of_doubles_sketch_test.cpp`,
//! and that file was already ported 1:1 to
//! `apache-datasketches/tests/array_of_doubles_sketch_test.rs`. There is no
//! second upstream file for this crate to port for a-not-b coverage. Instead,
//! most of the tests below follow `theta_a_not_b_test.rs`'s structure and
//! naming (empty operands, exact-mode partial overlap, `compute(a, a)`
//! self-difference, ordered/unordered results, and estimation-mode
//! tolerance), adapted to this family's per-entry `f64` value arrays and
//! dropping the wrapped-compact variant (this family has only
//! `ArrayOfDoublesSketch` and `CompactArrayOfDoublesSketch`, no
//! wrapped-compact type). `theta_a_not_b_test`'s equivalent cases only reach
//! k = 4096 (the default) with ≤1500 keys and so never leave exact mode; the
//! tests here push well past that (up to 30,000 keys) specifically to
//! exercise estimation-mode entry eviction, which is new coverage relative to
//! what upstream's tuple test file or `theta_a_not_b_test` provide.
//!
//! One exception: `a_not_b_issue_152_large_size_mismatch` genuinely derives
//! from an upstream theta regression test,
//! `theta/test/theta_a_not_b_test.cpp`'s `TEST_CASE("theta a not b: issue
//! 152")` (also ported in this repo as
//! `theta_a_not_b_test.rs::a_not_b_issue_152_large_size_mismatch`). It is
//! applied here to the tuple family because tuple's a-not-b is built on the
//! same underlying theta set-difference algorithm, so the same large-`b`
//! size-mismatch regression is plausible here too.
//!
//! Additional tests beyond `theta_a_not_b_test`'s shape check what this
//! family adds over Theta: that a-not-b's result preserves `a`'s per-entry
//! values unchanged (not summed or zeroed) even in estimation mode where
//! entries get evicted, and `get_num_values()` preservation on the result.
use apache_datasketches::tuple::{
    ArrayOfDoublesAnotB, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder,
};

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
fn a_not_b_both_empty_is_empty() {
    let a = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let b = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let a_not_b = ArrayOfDoublesAnotB::new();
    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn a_not_b_a_empty_is_empty() {
    let a = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let b = sketch(1, 0..1, &[1.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();
    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn a_not_b_b_empty_returns_a() {
    let a = sketch(2, 0..5, &[3.0, 4.0]);
    let b = ArrayOfDoublesSketchBuilder::new()
        .num_values(2)
        .build()
        .unwrap();
    let a_not_b = ArrayOfDoublesAnotB::new();
    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert_eq!(result.get_estimate(), 5.0);
    for (_, values) in result.entries() {
        assert_eq!(values, [3.0, 4.0]);
    }
}

#[test]
fn a_not_b_disjoint_sets_returns_a() {
    let a = sketch(1, 0..5, &[1.0]);
    let b = sketch(1, 5..10, &[1.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();
    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert_eq!(result.get_estimate(), 5.0);
}

#[test]
fn a_not_b_exact_partial_overlap_returns_exact_difference() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 500..1500, &[1.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();
    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert!(!result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 500.0);
}

/// Unlike theta's a-not-b, this does not assert an ordered-`a` input forces
/// an ordered result — only that ordered vs. unordered results agree on
/// retained count and estimate, and that `is_ordered()` reflects the
/// requested flag.
#[test]
fn a_not_b_result_ordered_vs_unordered() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 500..1500, &[1.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();

    let ordered = a_not_b.compute(&a, &b, true).unwrap();
    assert!(ordered.is_ordered());
    assert_eq!(ordered.get_estimate(), 500.0);

    let unordered = a_not_b.compute(&a, &b, false).unwrap();
    assert!(!unordered.is_ordered());
    assert_eq!(unordered.get_estimate(), 500.0);
    assert_eq!(unordered.get_num_retained(), ordered.get_num_retained());

    // ordered (compact) inputs
    let result = a_not_b
        .compute(&a.compact(true), &b.compact(true), true)
        .unwrap();
    assert!(result.is_ordered());
    assert_eq!(result.get_estimate(), 500.0);
}

#[test]
fn a_not_b_self_is_always_empty() {
    let s = sketch(1, 0..1000, &[1.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();

    let result = a_not_b.compute(&s, &s, true).unwrap();
    assert!(result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);

    let compact = s.compact(true);
    let result = a_not_b.compute(&compact, &compact, true).unwrap();
    assert!(result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn a_not_b_estimation_mode_half_overlap_within_tolerance() {
    let a = sketch(1, 0..10_000, &[1.0]);
    let b = sketch(1, 5_000..15_000, &[1.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();

    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);

    let result = a_not_b
        .compute(&a.compact(true), &b.compact(true), true)
        .unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);
}

#[test]
fn a_not_b_estimation_mode_disjoint_full_difference() {
    let a = sketch(1, 0..10_000, &[1.0]);
    let b = sketch(1, 10_000..20_000, &[1.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();

    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 10_000.0).abs() < 10_000.0 * 0.02);

    let result = a_not_b
        .compute(&a.compact(true), &b.compact(true), true)
        .unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 10_000.0).abs() < 10_000.0 * 0.02);
}

/// Derived from upstream's `theta/test/theta_a_not_b_test.cpp`
/// `TEST_CASE("theta a not b: issue 152")`, already ported to this repo as
/// `theta_a_not_b_test.rs::a_not_b_issue_152_large_size_mismatch`. Applied
/// here because tuple's a-not-b shares the same underlying theta
/// set-difference algorithm, so the same large-`b`-vs-`a` size mismatch is
/// plausible in this family too.
#[test]
fn a_not_b_issue_152_large_size_mismatch() {
    let a = sketch(1, 0..10_000, &[1.0]);
    let b = sketch(1, 5_000..30_000, &[1.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();

    // unordered inputs
    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.03);

    // ordered (compact) inputs
    let result = a_not_b
        .compute(&a.compact(true), &b.compact(true), true)
        .unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.03);
}

/// Beyond theta: verifies a-not-b's documented value-preservation property
/// (`ArrayOfDoublesAnotB`'s doc comment: "Retained entries keep `a`'s values
/// unchanged") in estimation mode, where theta's thresholding evicts most
/// entries. Every key in `a` carries the same values, [7.0, 8.0], so the
/// expected value of a surviving entry is fully determined regardless of
/// which entries theta happens to keep.
#[test]
fn a_not_b_estimation_mode_value_preservation() {
    let a = sketch(2, 0..10_000, &[7.0, 8.0]);
    let b = sketch(2, 10_000..20_000, &[7.0, 8.0]);
    let a_not_b = ArrayOfDoublesAnotB::new();

    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 10_000.0).abs() < 10_000.0 * 0.02);

    let mut count = 0;
    for (_, values) in result.entries() {
        assert_eq!(values, [7.0, 8.0]);
        count += 1;
    }
    assert!(count > 0, "expected at least one surviving entry");
}

/// Beyond theta: `get_num_values()` on the compact result reflects the
/// operands' configured width even once it is in estimation mode.
#[test]
fn a_not_b_get_num_values_preserved_in_estimation_mode() {
    let a = sketch(3, 0..10_000, &[1.0, 2.0, 3.0]);
    let b = ArrayOfDoublesSketchBuilder::new()
        .num_values(3)
        .build()
        .unwrap();
    let a_not_b = ArrayOfDoublesAnotB::new();

    let result = a_not_b.compute(&a, &b, true).unwrap();
    assert!(result.is_estimation_mode());
    assert_eq!(result.get_num_values(), 3);
}
