#![cfg(feature = "theta")]

use apache_datasketches_sys::theta_compact::ffi as compact_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;
use apache_datasketches_sys::theta_wrapped::ffi as wrapped_ffi;

#[test]
fn wrap_matches_compact_estimate() {
    let mut sketch = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i);
    }
    let compact = compact_ffi::theta_sketch_compact(&sketch, true);
    let bytes = compact.serialize_compact();

    let wrapped = wrapped_ffi::wrapped_compact_theta_sketch_wrap(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), wrapped.get_estimate());
    assert_eq!(compact.get_num_retained(), wrapped.get_num_retained());
}
