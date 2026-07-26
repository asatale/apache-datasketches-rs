use apache_datasketches_sys::hll::ffi;

#[test]
fn construct_update_estimate_roundtrip() {
    let mut sketch = ffi::new_hll_sketch(8, ffi::TargetHllType::Hll8).unwrap();
    for i in 0..100u64 {
        sketch.pin_mut().update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 100.0).abs() < 5.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_config_k_returns_err() {
    let result = ffi::new_hll_sketch(3, ffi::TargetHllType::Hll8);
    assert!(result.is_err());
}
