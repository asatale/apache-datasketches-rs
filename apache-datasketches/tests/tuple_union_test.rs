// apache-datasketches/tests/tuple_union_test.rs
//! Rust-authored coverage for [`ArrayOfDoublesUnion`], structured to mirror
//! this repo's own `theta_union_test.rs` rather than any upstream C++ file.
//!
//! This is *not* a port of an upstream test file. Upstream ships exactly one
//! tuple test file,
//! `vendor/datasketches-cpp/tuple/test/array_of_doubles_sketch_test.cpp`,
//! and that file was already ported 1:1 to
//! `apache-datasketches/tests/array_of_doubles_sketch_test.rs`. There is no
//! second upstream file for this crate to port for union coverage. Instead,
//! the tests below follow `theta_union_test.rs`'s structure and naming
//! (empty operands, exact-mode disjoint union, `reset()`, estimation-mode
//! tolerance, builder `lg_k` precedence, and out-of-range `lg_k` rejection),
//! adapted to this family's per-entry `f64` value arrays. `theta_union_test`
//! itself only reaches k = 4096 (the default) with ≤1500 keys and so never
//! leaves exact mode; the tests here push well past that (up to 30,000 keys)
//! specifically to exercise estimation-mode entry eviction, which is new
//! coverage relative to what upstream's tuple test file or `theta_union_test`
//! provide.
//!
//! Additional tests beyond `theta_union_test`'s shape check what this family
//! adds over Theta: the summed-per-index values of surviving entries in
//! estimation mode, `get_result(false)` (unordered) parity with the ordered
//! result, and `get_num_values()` preservation.
use apache_datasketches::tuple::{
    ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder,
};

fn sketch(num_values: u8, keys: std::ops::Range<u64>) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    let values: Vec<f64> = (0..num_values).map(|i| (i + 1) as f64).collect();
    for key in keys {
        s.update_u64(key, &values).unwrap();
    }
    s
}

#[test]
fn union_of_empty_sketches_is_empty() {
    let a = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let b = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let mut union_ = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    union_.update(&a).unwrap();
    union_.update(&b).unwrap();
    let result = union_.get_result(true);
    assert!(result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
}

#[test]
fn union_with_one_empty_one_nonempty() {
    let a = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let b = sketch(1, 0..1);
    let mut union_ = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    union_.update(&a).unwrap();
    union_.update(&b).unwrap();
    let result = union_.get_result(true);
    assert_eq!(result.get_estimate(), 1.0);
    assert!(!result.is_empty());
}

#[test]
fn union_exact_mode_no_overlap() {
    let a = sketch(1, 0..5);
    let b = sketch(1, 5..10);
    let mut union_ = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    union_.update(&a).unwrap();
    union_.update(&b).unwrap();
    let result = union_.get_result(true);
    assert_eq!(result.get_estimate(), 10.0);
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_num_retained(), 10);
}

#[test]
fn union_reset_clears_state() {
    let a = sketch(1, 0..1);
    let mut union_ = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    union_.update(&a).unwrap();
    assert!(!union_.get_result(true).is_empty());
    assert!(union_.get_result(true).get_num_retained() > 0);

    union_.reset();
    let result = union_.get_result(true);
    assert!(result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
}

#[test]
fn union_estimation_mode_large_overlapping_sets_within_tolerance() {
    let a = sketch(1, 0..10_000);
    let b = sketch(1, 5_000..15_000);
    let mut union_ = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    union_.update(&a).unwrap();
    union_.update(&b).unwrap();
    let result = union_.get_result(true);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 15_000.0).abs() < 15_000.0 * 0.01);

    union_.reset();
    let after_reset = union_.get_result(true);
    assert_eq!(after_reset.get_num_retained(), 0);
    assert!(after_reset.is_empty());
    assert!(!after_reset.is_estimation_mode());
}

#[test]
fn union_uses_builders_own_lg_k() {
    let mut sketch1 = ArrayOfDoublesSketchBuilder::new().lg_k(14).build().unwrap();
    for i in 0..16_384u64 {
        sketch1.update_u64(i, &[1.0]).unwrap();
    }
    let mut sketch2 = ArrayOfDoublesSketchBuilder::new().lg_k(14).build().unwrap();
    for i in 0..26_384u64 {
        sketch2.update_u64(i, &[1.0]).unwrap();
    }
    let mut sketch3 = ArrayOfDoublesSketchBuilder::new().lg_k(14).build().unwrap();
    for i in 0..86_384u64 {
        sketch3.update_u64(i, &[1.0]).unwrap();
    }

    let mut union1 = ArrayOfDoublesUnionBuilder::new().lg_k(16).build().unwrap();
    union1.update(&sketch2).unwrap();
    union1.update(&sketch1).unwrap();
    union1.update(&sketch3).unwrap();
    let result1 = union1.get_result(true);
    assert_eq!(result1.get_estimate(), sketch3.get_estimate());

    let mut union2 = ArrayOfDoublesUnionBuilder::new().lg_k(16).build().unwrap();
    union2.update(&sketch1).unwrap();
    union2.update(&sketch3).unwrap();
    union2.update(&sketch2).unwrap();
    let result2 = union2.get_result(true);
    assert_eq!(result2.get_estimate(), sketch3.get_estimate());
}

#[test]
fn union_builder_rejects_lg_k_out_of_range() {
    assert!(ArrayOfDoublesUnionBuilder::new().lg_k(4).build().is_err());
    assert!(ArrayOfDoublesUnionBuilder::new().lg_k(27).build().is_err());
    assert!(ArrayOfDoublesUnionBuilder::new().lg_k(5).build().is_ok());
    assert!(ArrayOfDoublesUnionBuilder::new().lg_k(26).build().is_ok());
}

/// Beyond theta: verifies the surviving entries' *values* in estimation
/// mode, not just cardinality. Both inputs assign the same values to every
/// key ([1.0, 10.0]), so regardless of which entries theta's thresholding
/// happens to keep, every surviving entry's values are fully determined by
/// the sum-on-collision policy: [1.0, 10.0] for a key present in only one
/// input, [2.0, 20.0] for a key present in both. No third value is possible.
#[test]
fn union_estimation_mode_value_correctness() {
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

    let mut union_ = ArrayOfDoublesUnionBuilder::new()
        .num_values(2)
        .build()
        .unwrap();
    union_.update(&a).unwrap();
    union_.update(&b).unwrap();
    let result = union_.get_result(true);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert_eq!(result.get_num_values(), 2);
    assert!((result.get_estimate() - 30_000.0).abs() < 30_000.0 * 0.02);

    let mut saw_single = false;
    let mut saw_summed = false;
    for (_, values) in result.entries() {
        if values == [1.0, 10.0] {
            saw_single = true;
        } else if values == [2.0, 20.0] {
            saw_summed = true;
        } else {
            panic!("unexpected surviving entry value: {values:?}");
        }
    }
    assert!(
        saw_single,
        "expected at least one non-overlapping key to survive"
    );
    assert!(
        saw_summed,
        "expected at least one overlapping key to survive"
    );
}

/// Beyond theta: `get_result(false)` (unordered) must report the same
/// retained count and estimate as `get_result(true)` (ordered) — the only
/// difference is entry ordering, not which entries survived. This does not
/// assert the unordered result is *unsorted*; that is not guaranteed.
#[test]
fn union_get_result_unordered_matches_ordered() {
    let a = sketch(1, 0..10_000);
    let b = sketch(1, 5_000..15_000);
    let mut union_ = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    union_.update(&a).unwrap();
    union_.update(&b).unwrap();

    let ordered = union_.get_result(true);
    let unordered = union_.get_result(false);
    assert!(ordered.is_estimation_mode());
    assert_eq!(unordered.get_num_retained(), ordered.get_num_retained());
    assert_eq!(unordered.get_estimate(), ordered.get_estimate());
    assert!(ordered.is_ordered());
    assert!(!unordered.is_ordered());
}

/// Beyond theta: `get_num_values()` on the compact result reflects the
/// union's configured width even once it is in estimation mode.
#[test]
fn union_get_num_values_preserved_in_estimation_mode() {
    let mut a = ArrayOfDoublesSketchBuilder::new()
        .num_values(3)
        .build()
        .unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i, &[1.0, 2.0, 3.0]).unwrap();
    }
    let mut union_ = ArrayOfDoublesUnionBuilder::new()
        .num_values(3)
        .build()
        .unwrap();
    union_.update(&a).unwrap();
    let result = union_.get_result(true);
    assert!(result.is_estimation_mode());
    assert_eq!(result.get_num_values(), 3);
}
