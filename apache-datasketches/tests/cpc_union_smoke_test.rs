use apache_datasketches::cpc::{CpcSketchBuilder, CpcUnionBuilder};

#[test]
fn union_merges_two_sketches() {
    let mut a = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    let mut b = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 500..1000u64 {
        b.update_u64(i);
    }

    let mut union = CpcUnionBuilder::new().lg_k(11).build().unwrap();
    union.update(&a);
    union.update(&b);

    let result = union.get_result();
    let estimate = result.get_estimate();
    assert!((estimate - 1000.0).abs() < 30.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_is_err() {
    assert!(CpcUnionBuilder::new().lg_k(3).build().is_err());
}
