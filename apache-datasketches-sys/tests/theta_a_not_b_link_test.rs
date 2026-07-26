use apache_datasketches_sys::theta_a_not_b::ffi as a_not_b_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;

#[test]
fn a_not_b_two_sketches() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    let mut b = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    for i in 250..750u64 {
        b.pin_mut().update_u64(i);
    }

    let a_not_b = a_not_b_ffi::new_theta_a_not_b();
    let result = a_not_b.compute_sketch_sketch(&a, &b, true);
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}
