#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_sketch::ffi;

#[test]
fn construct_update_estimate() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &[1.0]);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
    assert_eq!(sketch.get_num_values(), 1);
    assert_eq!(sketch.get_num_retained(), 1000);
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = ffi::new_array_of_doubles_sketch(4, ffi::TupleResizeFactor::X8, 1.0, 1);
    assert!(result.is_err());
}

#[test]
fn entries_expose_hashes_and_values() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 2).unwrap();
    sketch.pin_mut().update_u64(1, &[3.0, 4.0]);
    sketch.pin_mut().update_u64(1, &[3.0, 4.0]);
    assert_eq!(sketch.get_num_retained(), 1);
    let hashes = sketch.entry_hashes();
    let values = sketch.entry_values();
    assert_eq!(hashes.len(), 1);
    // Two updates of the same key sum their values.
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], 6.0);
    assert_eq!(values[1], 8.0);
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    sketch.pin_mut().update_u64(1, &[1.0]);
    assert!(!sketch.is_empty());
    sketch.pin_mut().reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 0);
}
