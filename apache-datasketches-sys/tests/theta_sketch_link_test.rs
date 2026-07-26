#![cfg(feature = "theta")]

use apache_datasketches_sys::theta_sketch::ffi;

#[test]
fn construct_update_estimate_roundtrip() {
    let mut sketch = ffi::new_theta_sketch(12, ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = ffi::new_theta_sketch(4, ffi::ResizeFactor::X8, 1.0);
    assert!(result.is_err());
}

#[test]
fn compact_via_sketch_method() {
    let mut sketch = ffi::new_theta_sketch(12, ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }
    let compact = sketch.compact(true);
    assert!((compact.get_estimate() - 1000.0).abs() < 1.0);
}
