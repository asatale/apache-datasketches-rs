#![cfg(feature = "cpc")]

use apache_datasketches_sys::cpc_sketch::ffi;

#[test]
fn construct_update_estimate_roundtrip() {
    let mut sketch = ffi::new_cpc_sketch(11).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 30.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = ffi::new_cpc_sketch(3);
    assert!(result.is_err());
}
