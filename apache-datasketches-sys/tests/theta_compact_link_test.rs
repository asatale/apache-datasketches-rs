#![cfg(feature = "theta")]

use apache_datasketches_sys::theta_compact::ffi as compact_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;

#[test]
fn compact_and_serialize_round_trip() {
    let mut sketch = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }

    let compact = compact_ffi::theta_sketch_compact(&sketch, true);
    assert!((compact.get_estimate() - 1000.0).abs() < 1.0);
    assert!(compact.is_ordered());

    let bytes = compact.serialize_compact();
    let restored = compact_ffi::compact_theta_sketch_deserialize(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());

    let compressed = compact.serialize_compressed();
    let restored_compressed = compact_ffi::compact_theta_sketch_deserialize(&compressed).unwrap();
    assert_eq!(compact.get_estimate(), restored_compressed.get_estimate());
}
