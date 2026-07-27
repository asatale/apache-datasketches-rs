use apache_datasketches::cpc::CpcSketchBuilder;

#[test]
fn construct_update_estimate() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 30.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_is_err() {
    assert!(CpcSketchBuilder::new().lg_k(3).build().is_err());
}
