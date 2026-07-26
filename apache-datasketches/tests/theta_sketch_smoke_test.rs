use apache_datasketches::theta::ThetaSketchBuilder;

#[test]
fn construct_update_estimate() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_is_err() {
    assert!(ThetaSketchBuilder::new().lg_k(4).build().is_err());
}
