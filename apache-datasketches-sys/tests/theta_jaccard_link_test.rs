use apache_datasketches_sys::theta_jaccard::ffi as jaccard_ffi;
use apache_datasketches_sys::theta_sketch::ffi as sketch_ffi;

#[test]
fn jaccard_identical_sketches_is_one() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..1000u64 {
        a.pin_mut().update_u64(i);
    }
    let bounds = jaccard_ffi::jaccard_sketch_sketch(&a, &a);
    assert!((bounds.estimate - 1.0).abs() < 1e-9);
    assert!((bounds.lower_bound - 1.0).abs() < 1e-9);
    assert!((bounds.upper_bound - 1.0).abs() < 1e-9);
}

#[test]
fn jaccard_disjoint_sketches_is_zero() {
    let mut a = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    let mut b = sketch_ffi::new_theta_sketch(12, sketch_ffi::ResizeFactor::X8, 1.0).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    for i in 500..1000u64 {
        b.pin_mut().update_u64(i);
    }
    let bounds = jaccard_ffi::jaccard_sketch_sketch(&a, &b);
    assert!(bounds.estimate.abs() < 1e-9);
}
