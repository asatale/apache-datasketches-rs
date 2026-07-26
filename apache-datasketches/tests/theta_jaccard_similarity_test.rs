//! Ported from theta/test/theta_jaccard_similarity_test.cpp (tag 5.2.0).
//! The upstream file's custom-seed cases are not ported: this crate never
//! exposes a seed parameter (see Global Constraints), so there is no
//! reachable equivalent through the public API.
use apache_datasketches::theta::{jaccard_similarity, ThetaSketchBuilder};

#[test]
fn empty_sketches() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let b = ThetaSketchBuilder::new().build().unwrap();
    let bounds = jaccard_similarity(&a, &b);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
}

#[test]
fn first_empty_second_not() {
    let a = ThetaSketchBuilder::new().build().unwrap();
    let mut b = ThetaSketchBuilder::new().build().unwrap();
    b.update_u64(1);
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.estimate.abs() < 1e-9);
}

#[test]
fn second_empty_first_not() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    a.update_u64(1);
    let b = ThetaSketchBuilder::new().build().unwrap();
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.estimate.abs() < 1e-9);
}

#[test]
fn exact_mode_identical() {
    let mut a = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..100u64 {
        a.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &a);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
    assert!((bounds.lower_bound - 1.0).abs() < 1e-9);
    assert!((bounds.upper_bound - 1.0).abs() < 1e-9);
}

#[test]
fn estimation_mode_similar_sketches() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    for i in 0..10_000u64 {
        b.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.lower_bound <= bounds.estimate);
    assert!(bounds.estimate <= bounds.upper_bound);
    assert!((bounds.estimate - 1.0).abs() < 0.05);
}

#[test]
fn estimation_mode_half_overlap() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        a.update_u64(i);
    }
    for i in 5_000..15_000u64 {
        b.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.lower_bound <= bounds.estimate);
    assert!(bounds.estimate <= bounds.upper_bound);
    // union ~15000, intersection ~5000 => jaccard ~= 1/3
    assert!((bounds.estimate - (1.0 / 3.0)).abs() < 0.05);
}
