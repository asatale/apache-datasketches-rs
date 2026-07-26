use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;
use apache_datasketches_sys::theta_union::ffi as union_ffi;

#[test]
fn union_two_sketches() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    let mut b = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    for i in 250..750u64 {
        b.pin_mut().update_u64(i);
    }

    let mut union_ = union_ffi::new_theta_union(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    union_.pin_mut().update_with_sketch(&a);
    union_.pin_mut().update_with_sketch(&b);

    let result = union_.get_result(true);
    assert!((result.get_estimate() - 750.0).abs() < 20.0);
}
