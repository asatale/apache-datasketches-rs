use apache_datasketches::theta::{jaccard_similarity, ThetaSketchBuilder};

#[test]
fn jaccard_identical_sketches_is_one() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        a.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &a);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
}

#[test]
fn jaccard_disjoint_sketches_is_zero() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    for i in 500..1000u64 {
        b.update_u64(i);
    }
    let bounds = jaccard_similarity(&a, &b);
    assert!(bounds.estimate.abs() < 1e-9);
}
