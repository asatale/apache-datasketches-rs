// apache-datasketches/tests/cpc_union_test.rs
//! Ported from cpc/test/cpc_union_test.cpp (tag 5.2.0). 6 of 9 upstream
//! cases ported; 3 excluded:
//! - "copy" — tests C++ copy-constructor/assignment semantics; this
//!   crate's `CpcUnion` doesn't implement `Clone`.
//! - "custom seed" — no seed parameter is exposed in this crate's public
//!   API.
//! - "moving update" — exercises a C++-specific move-constructor update
//!   overload (`update(cpc_sketch_alloc&&)`) purely as a copy-avoidance
//!   optimization; behaviorally identical to updating via a reference,
//!   which every other ported case already exercises through
//!   `CpcUnion::update(&CpcSketch)`.
//!
//! "large" is adapted: upstream additionally asserts
//! `r.get_num_coupons() == s.get_num_coupons()`, but `get_num_coupons()` is
//! not exposed (marked `@private` upstream, for internal debugging use
//! only) — the estimate-comparison assertion below is kept.
use apache_datasketches::cpc::{CpcSketchBuilder, CpcUnionBuilder};

const RELATIVE_ERROR_FOR_LG_K_11: f64 = 0.02;

#[test]
fn lg_k_limits() {
    assert!(CpcUnionBuilder::new().lg_k(4).build().is_ok());
    assert!(CpcUnionBuilder::new().lg_k(26).build().is_ok());
    assert!(CpcUnionBuilder::new().lg_k(3).build().is_err());
    assert!(CpcUnionBuilder::new().lg_k(27).build().is_err());
}

#[test]
fn empty() {
    let union = CpcUnionBuilder::new().lg_k(11).build().unwrap();
    let result = union.get_result();
    assert!(result.is_empty());
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn large() {
    let mut s = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let mut union = CpcUnionBuilder::new().lg_k(11).build().unwrap();
    let mut key = 0u64;
    for _ in 0..1000 {
        let mut tmp = CpcSketchBuilder::new().lg_k(11).build().unwrap();
        for _ in 0..10_000 {
            s.update_u64(key);
            tmp.update_u64(key);
            key += 1;
        }
        union.update(&tmp);
    }
    let r = union.get_result();
    let expected = s.get_estimate();
    assert!((r.get_estimate() - expected).abs() < expected * RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn reduce_k_empty() {
    let mut s = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..10_000u64 {
        s.update_u64(i);
    }
    let mut union = CpcUnionBuilder::new().lg_k(12).build().unwrap();
    union.update(&s);
    let r = union.get_result();
    assert_eq!(r.get_lg_k(), 11);
    let estimate = r.get_estimate();
    assert!((estimate - 10_000.0).abs() < 10_000.0 * RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn reduce_k_sparse() {
    let mut union = CpcUnionBuilder::new().lg_k(12).build().unwrap();

    let mut s12 = CpcSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..100u64 {
        s12.update_u64(i);
    }
    union.update(&s12);

    let mut s11 = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..1000u64 {
        s11.update_u64(i);
    }
    union.update(&s11);

    let r = union.get_result();
    assert_eq!(r.get_lg_k(), 11);
    let estimate = r.get_estimate();
    assert!((estimate - 1000.0).abs() < 1000.0 * RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn reduce_k_window() {
    let mut union = CpcUnionBuilder::new().lg_k(12).build().unwrap();

    let mut s12 = CpcSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        s12.update_u64(i);
    }
    union.update(&s12);

    let mut s11 = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..1000u64 {
        s11.update_u64(i);
    }
    union.update(&s11);

    let r = union.get_result();
    assert_eq!(r.get_lg_k(), 11);
    let estimate = r.get_estimate();
    assert!((estimate - 1000.0).abs() < 1000.0 * RELATIVE_ERROR_FOR_LG_K_11);
}
