use apache_datasketches_sys::theta_intersection::ffi as intersection_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;

#[test]
fn intersect_two_sketches() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    let mut b = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    for i in 250..750u64 {
        b.pin_mut().update_u64(i);
    }

    let mut isect = intersection_ffi::new_theta_intersection();
    assert!(!isect.has_result());
    isect.pin_mut().update_with_sketch(&a);
    isect.pin_mut().update_with_sketch(&b);
    assert!(isect.has_result());

    let result = isect.get_result(true).unwrap();
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}

#[test]
fn get_result_before_update_throws() {
    let isect = intersection_ffi::new_theta_intersection();
    assert!(isect.get_result(true).is_err());
}
