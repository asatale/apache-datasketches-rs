// apache-datasketches/tests/theta_a_not_b_test.rs
//! Ported from theta/test/theta_a_not_b_test.cpp (tag 5.2.0).
//!
//! Upstream has 11 `TEST_CASE`s: "empty", "non empty no retained keys",
//! "exact mode half overlap", "exact mode disjoint", "exact mode full
//! overlap", "estimation mode half overlap", "estimation mode half overlap
//! wrapped compact", "estimation mode disjoint", "estimation mode full
//! overlap", "seed mismatch", and "issue #152".
//!
//! `a_not_b_both_empty_is_empty` and `a_not_b_a_empty_is_empty` cover the
//! same territory as upstream's "empty" case. `a_not_b_b_empty_returns_a`
//! and `a_not_b_disjoint_sets_returns_a` cover a-not-b-with-empty-b and
//! disjoint-set behavior consistent with "exact mode disjoint" (b empty is
//! degenerate disjointness).
//!
//! `a_not_b_exact_partial_overlap_returns_exact_difference` and
//! `a_not_b_result_ordered_vs_unordered` port "exact mode half overlap"
//! (split into the exact-count assertion and the ordered/unordered/`A`-
//! forces-order sub-assertions). `a_not_b_self_is_always_empty` ports
//! "exact mode full overlap". `a_not_b_estimation_mode_half_overlap_within_
//! tolerance` ports "estimation mode half overlap".
//! `a_not_b_estimation_mode_wrapped_compact_inputs` ports "estimation mode
//! half overlap wrapped compact". `a_not_b_estimation_mode_disjoint_full_
//! difference` ports "estimation mode disjoint". `a_not_b_issue_152_large_
//! size_mismatch` ports "issue #152" (a much larger `b` than `a`, checking
//! the a-not-b estimate is still close to the true difference).
//!
//! Not ported: upstream's "seed mismatch" — this crate never exposes a seed
//! parameter (every sketch always uses upstream's `DEFAULT_SEED`), so there
//! is no reachable equivalent through the public API. "estimation mode full
//! overlap" is not separately ported since it exercises the same
//! self-difference-is-empty property as "exact mode full overlap"
//! (`a_not_b_self_is_always_empty`) and `theta_input_dispatch_test.rs`'s
//! `a_not_b_accepts_all_nine_combinations`, just in estimation mode rather
//! than exact mode; `ThetaInput` dispatch parity across
//! `ThetaSketch`/`CompactThetaSketch`/`WrappedCompactThetaSketch` beyond the
//! wrapped-compact case above is already exhaustively covered by that same
//! dispatch test.
//!
//! Plus `a_not_b_empty_b_with_different_seed_is_ignored`, ported from tag 5.3.0 at the end of
//! this file; it reaches the seed-hash path without a seed API by hand-editing the seed-hash
//! bytes of an empty compact image.
use apache_datasketches::theta::{ThetaAnotB, ThetaSketchBuilder};

#[test]
fn a_not_b_both_empty_is_empty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let b = ThetaSketchBuilder::new().build().unwrap();
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert!(result.is_empty());
}

#[test]
fn a_not_b_a_empty_is_empty() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    b.update_u64(1);
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert!(result.is_empty());
}

#[test]
fn a_not_b_b_empty_returns_a() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..5u64 {
        a.update_u64(i);
    }
    let b = ThetaSketchBuilder::new().build().unwrap();
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert_eq!(result.get_estimate(), 5.0);
}

#[test]
fn a_not_b_disjoint_sets_returns_a() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..5u64 {
        a.update_u64(i);
    }
    for i in 5..10u64 {
        b.update_u64(i);
    }
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert_eq!(result.get_estimate(), 5.0);
}

#[test]
fn a_not_b_exact_partial_overlap_returns_exact_difference() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..1000u64 {
        a.update_u64(i);
    }
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 500..1500u64 {
        b.update_u64(i);
    }
    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert!(!result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 500.0);
}

#[test]
fn a_not_b_result_ordered_vs_unordered() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..1000u64 {
        a.update_u64(i);
    }
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 500..1500u64 {
        b.update_u64(i);
    }
    let a_not_b = ThetaAnotB::new();

    // unordered inputs, ordered result
    let result = a_not_b.compute(&a, &b, true);
    assert!(result.is_ordered());
    assert_eq!(result.get_estimate(), 500.0);

    // unordered inputs, unordered result
    let result = a_not_b.compute(&a, &b, false);
    assert!(!result.is_ordered());
    assert_eq!(result.get_estimate(), 500.0);

    // ordered (compact) inputs
    let result = a_not_b.compute(&a.compact(true), &b.compact(true), true);
    assert!(result.is_ordered());
    assert_eq!(result.get_estimate(), 500.0);

    // A is ordered, so the result is ordered regardless of the `ordered` flag
    let result = a_not_b.compute(&a.compact(true), &b, false);
    assert!(result.is_ordered());
    assert_eq!(result.get_estimate(), 500.0);
}

#[test]
fn a_not_b_self_is_always_empty() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let a_not_b = ThetaAnotB::new();

    // unordered inputs
    let result = a_not_b.compute(&sketch, &sketch, true);
    assert!(result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);

    // ordered (compact) inputs
    let compact = sketch.compact(true);
    let result = a_not_b.compute(&compact, &compact, true);
    assert!(result.is_empty());
    assert!(!result.is_estimation_mode());
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn a_not_b_estimation_mode_half_overlap_within_tolerance() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 5_000..15_000u64 {
        b.update_u64(i);
    }
    let a_not_b = ThetaAnotB::new();

    // unordered inputs
    let result = a_not_b.compute(&a, &b, true);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);

    // ordered (compact) inputs
    let result = a_not_b.compute(&a.compact(true), &b.compact(true), true);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);
}

#[test]
fn a_not_b_estimation_mode_wrapped_compact_inputs() {
    use apache_datasketches::theta::WrappedCompactThetaSketch;

    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    let bytes_a = a.compact(true).serialize_compact();

    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 5_000..15_000u64 {
        b.update_u64(i);
    }
    let bytes_b = b.compact(true).serialize_compact();

    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(
        &WrappedCompactThetaSketch::wrap(&bytes_a).unwrap(),
        &WrappedCompactThetaSketch::wrap(&bytes_b).unwrap(),
        true,
    );
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.02);
}

#[test]
fn a_not_b_estimation_mode_disjoint_full_difference() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 10_000..20_000u64 {
        b.update_u64(i);
    }
    let a_not_b = ThetaAnotB::new();

    // unordered inputs
    let result = a_not_b.compute(&a, &b, true);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 10_000.0).abs() < 10_000.0 * 0.02);

    // ordered (compact) inputs
    let result = a_not_b.compute(&a.compact(true), &b.compact(true), true);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 10_000.0).abs() < 10_000.0 * 0.02);
}

#[test]
fn a_not_b_issue_152_large_size_mismatch() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    for i in 5_000..30_000u64 {
        b.update_u64(i);
    }
    let a_not_b = ThetaAnotB::new();

    // unordered inputs
    let result = a_not_b.compute(&a, &b, true);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.03);

    // ordered (compact) inputs
    let result = a_not_b.compute(&a.compact(true), &b.compact(true), true);
    assert!(!result.is_empty());
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 5_000.0).abs() < 5_000.0 * 0.03);
}

// Ported from vendor/datasketches-cpp/theta/test/theta_a_not_b_test.cpp,
// TEST_CASE("theta a-not-b: empty B with different seed must be ignored") at tag 5.3.0.
//
// An empty sketch carries no hashes, so its seed hash is meaningless; a-not-b must skip the seed
// hash check for an empty B, as deserialization, union and intersection already do. This matters
// across languages because datasketches-java serializes every empty compact sketch with a seed
// hash of 0.
//
// In 5.3.0 (and in Java) an empty compact sketch serializes seed hash 0, whatever its seed. In
// 5.2.0 it serialized its real seed hash, and a-not-b compared B's seed hash against the default
// seed hash. On a mismatch it threw from inside `theta_a_not_b::compute`, which this crate's
// bridge does not declare as `Result`-returning, so the whole process aborted instead of
// returning an error. This crate exposes no seed API, so the test serializes an empty
// default-seed compact sketch and hand-edits its seed-hash bytes (offset 6..8, little-endian) to
// an explicit foreign value, `compute_seed_hash(123)` = 0x05b6, to exercise that same path.
// Verified: this aborts the process on 5.2.0 and passes on 5.3.0.
#[test]
fn a_not_b_empty_b_with_different_seed_is_ignored() {
    use apache_datasketches::theta::{CompactThetaSketch, WrappedCompactThetaSketch};

    // A: non-empty, but with zero retained entries (all hashes exceeded theta)
    let mut a = ThetaSketchBuilder::new().p(1e-6).build().unwrap();
    a.update_u64(1);
    assert!(!a.is_empty());
    assert_eq!(a.get_num_retained(), 0);
    let a = a.compact(true);

    // B: empty, carrying the seed hash of a different seed (123)
    let mut b_bytes = ThetaSketchBuilder::new()
        .build()
        .unwrap()
        .compact(true)
        .serialize_compact();
    assert_eq!(
        b_bytes.len(),
        8,
        "empty compact image is a single preamble long"
    );
    b_bytes[6..8].copy_from_slice(&0x05b6u16.to_le_bytes());
    let b = CompactThetaSketch::deserialize(&b_bytes).unwrap();
    assert!(b.is_empty());

    let a_not_b = ThetaAnotB::new(); // default seed
    let result = a_not_b.compute(&a, &b, true);
    assert!(!result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
    assert_eq!(result.get_estimate(), a.get_estimate());

    // Same check with B wrapped rather than deserialized.
    let b_wrapped = WrappedCompactThetaSketch::wrap(&b_bytes).unwrap();
    assert!(b_wrapped.is_empty());
    let result = a_not_b.compute(&a, &b_wrapped, true);
    assert!(!result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
    assert_eq!(result.get_estimate(), a.get_estimate());
}
