use apache_datasketches::hll::{HllSketch, TargetHllType};

#[test]
fn construct_update_estimate() {
    let mut sketch = HllSketch::new(8, TargetHllType::Hll8).unwrap();
    for i in 0..100u64 {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 100.0).abs() < 5.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_config_k_is_err() {
    assert!(HllSketch::new(3, TargetHllType::Hll8).is_err());
}
