#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_compact::ffi as compact_ffi;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;

#[test]
fn compact_via_free_function_and_method() {
    let mut sketch =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &[1.0, 2.0]);
    }

    let via_free_fn = compact_ffi::array_of_doubles_sketch_compact(&sketch, true);
    assert!((via_free_fn.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(via_free_fn.get_num_values(), 2);
    assert!(via_free_fn.is_ordered());

    let via_method = sketch.compact(true);
    assert_eq!(
        via_method.get_num_retained(),
        via_free_fn.get_num_retained()
    );
}

#[test]
fn serialize_deserialize_round_trip() {
    let mut sketch =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &[1.0, 2.0]);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize();
    assert!(!bytes.is_empty());

    let restored = compact_ffi::compact_array_of_doubles_sketch_deserialize(&bytes).unwrap();
    assert_eq!(restored.get_num_retained(), compact.get_num_retained());
    assert_eq!(restored.get_num_values(), compact.get_num_values());
    assert_eq!(restored.get_estimate(), compact.get_estimate());
    // Collect into std Vecs: cxx::Vec does not implement PartialEq.
    let restored_hashes: Vec<u64> = restored.entry_hashes().into_iter().collect();
    let compact_hashes: Vec<u64> = compact.entry_hashes().into_iter().collect();
    assert_eq!(restored_hashes, compact_hashes);
    let restored_values: Vec<f64> = restored.entry_values().into_iter().collect();
    let compact_values: Vec<f64> = compact.entry_values().into_iter().collect();
    assert_eq!(restored_values, compact_values);
}

#[test]
fn deserialize_garbage_returns_err() {
    let bytes = [0u8; 8];
    assert!(compact_ffi::compact_array_of_doubles_sketch_deserialize(&bytes).is_err());
}
