#![cfg(feature = "cpc")]

use apache_datasketches_sys::{cpc_sketch::ffi as sketch_ffi, cpc_union::ffi as union_ffi};

#[test]
fn union_of_two_sketches_merges_estimate() {
    let mut a = sketch_ffi::new_cpc_sketch(11).unwrap();
    for i in 0..500u64 {
        a.pin_mut().update_u64(i);
    }
    let mut b = sketch_ffi::new_cpc_sketch(11).unwrap();
    for i in 500..1000u64 {
        b.pin_mut().update_u64(i);
    }

    let mut u = union_ffi::new_cpc_union(11).unwrap();
    u.pin_mut().update_sketch(&a);
    u.pin_mut().update_sketch(&b);

    let result = u.get_result();
    let estimate = result.get_estimate();
    assert!((estimate - 1000.0).abs() < 30.0, "estimate was {estimate}");
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = union_ffi::new_cpc_union(3);
    assert!(result.is_err());
}
