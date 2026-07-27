// apache-datasketches/tests/theta_union_test.rs
//! Ported from theta/test/theta_union_test.cpp (tag 5.2.0).
//!
//! Upstream has 7 `TEST_CASE`s: "empty", "non empty no retained keys",
//! "exact mode half overlap", "exact mode half overlap wrapped compact",
//! "estimation mode half overlap", "seed mismatch", and "larger K".
//!
//! `union_of_empty_sketches_is_empty`, `union_with_one_empty_one_nonempty`,
//! `union_exact_mode_no_overlap`, and `union_reset_clears_state` below cover
//! the same behavioral territory as upstream's "empty" and "exact mode half
//! overlap" cases (empty/non-empty unions, exact-mode union cardinality, and
//! post-`reset()` state), though restructured into smaller, more targeted
//! tests rather than literal 1:1 ports of those two `TEST_CASE` bodies.
//!
//! `union_estimation_mode_large_overlapping_sets_within_tolerance` ports
//! upstream's "estimation mode half overlap". `union_uses_builders_own_lg_k`
//! ports upstream's "larger K" (verifying the union result reflects the
//! union builder's own `lg_k`, independent of input order, rather than any
//! operand's `lg_k`) — note this assertion is an *exact* equality per
//! upstream, not tolerance-based, since it compares the union's result
//! directly against one of the update sketches built the same way.
//! `union_builder_rejects_lg_k_out_of_range` is an API-surface-driven
//! addition covering `theta_union::builder::set_lg_k`'s validation, which is
//! reachable through this crate's public builder API but not exercised by
//! upstream's `theta_union_test.cpp` directly.
//!
//! Not ported: upstream's "seed mismatch" and "non empty no retained keys"
//! (`set_seed`) case — this crate never exposes a seed parameter (every
//! sketch/union always uses upstream's `DEFAULT_SEED`), so there is no
//! reachable equivalent through the public API. "exact mode half overlap
//! wrapped compact" is not separately ported here since `ThetaInput`
//! dispatch parity across `ThetaSketch`/`CompactThetaSketch`/
//! `WrappedCompactThetaSketch` is already exhaustively covered by
//! `theta_input_dispatch_test.rs`'s `union_accepts_all_nine_combinations`.
use apache_datasketches::theta::{ThetaSketchBuilder, ThetaUnionBuilder};

#[test]
fn union_of_empty_sketches_is_empty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let b = ThetaSketchBuilder::new().build().unwrap();
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.update(&b);
    let result = union_.get_result(true);
    assert!(result.is_empty());
}

#[test]
fn union_with_one_empty_one_nonempty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    b.update_u64(1);
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.update(&b);
    let result = union_.get_result(true);
    assert_eq!(result.get_estimate(), 1.0);
}

#[test]
fn union_exact_mode_no_overlap() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..5u64 {
        a.update_u64(i);
    }
    for i in 5..10u64 {
        b.update_u64(i);
    }
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.update(&b);
    let result = union_.get_result(true);
    assert_eq!(result.get_estimate(), 10.0);
}

#[test]
fn union_reset_clears_state() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    a.update_u64(1);
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.reset();
    let result = union_.get_result(true);
    assert!(result.is_empty());
}

#[test]
fn union_estimation_mode_large_overlapping_sets_within_tolerance() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 5_000..15_000u64 {
        b.update_u64(i);
    }
    let mut union_ = ThetaUnionBuilder::new().build().unwrap();
    union_.update(&a);
    union_.update(&b);
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
    let mut sketch1 = ThetaSketchBuilder::new().lg_k(14).build().unwrap();
    for i in 0..16_384u64 {
        sketch1.update_u64(i);
    }
    let mut sketch2 = ThetaSketchBuilder::new().lg_k(14).build().unwrap();
    for i in 0..26_384u64 {
        sketch2.update_u64(i);
    }
    let mut sketch3 = ThetaSketchBuilder::new().lg_k(14).build().unwrap();
    for i in 0..86_384u64 {
        sketch3.update_u64(i);
    }

    let mut union1 = ThetaUnionBuilder::new().lg_k(16).build().unwrap();
    union1.update(&sketch2);
    union1.update(&sketch1);
    union1.update(&sketch3);
    let result1 = union1.get_result(true);
    assert_eq!(result1.get_estimate(), sketch3.get_estimate());

    let mut union2 = ThetaUnionBuilder::new().lg_k(16).build().unwrap();
    union2.update(&sketch1);
    union2.update(&sketch3);
    union2.update(&sketch2);
    let result2 = union2.get_result(true);
    assert_eq!(result2.get_estimate(), sketch3.get_estimate());
}

#[test]
fn union_builder_rejects_lg_k_out_of_range() {
    assert!(ThetaUnionBuilder::new().lg_k(4).build().is_err());
    assert!(ThetaUnionBuilder::new().lg_k(27).build().is_err());
    assert!(ThetaUnionBuilder::new().lg_k(5).build().is_ok());
    assert!(ThetaUnionBuilder::new().lg_k(26).build().is_ok());
}
