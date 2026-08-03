#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;
use apache_datasketches_sys::array_of_doubles_union::ffi as union_ffi;

fn sketch(keys: std::ops::Range<u64>) -> cxx::UniquePtr<sketch_ffi::ArrayOfDoublesSketchShim> {
    let mut s =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1)
            .unwrap();
    for key in keys {
        s.pin_mut().update_u64(key, &[1.0]);
    }
    s
}

#[test]
fn union_half_overlap() {
    let a = sketch(0..1000);
    let b = sketch(500..1500);
    let mut u =
        union_ffi::new_array_of_doubles_union(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_sketch(&b);
    let result = u.get_result(true);
    assert_eq!(result.get_estimate(), 1500.0);
    assert_eq!(result.get_num_values(), 1);
}

#[test]
fn union_accepts_compact_and_resets() {
    let a = sketch(0..100);
    let compact = a.compact(true);
    let mut u =
        union_ffi::new_array_of_doubles_union(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    u.pin_mut().update_with_compact(&compact);
    assert_eq!(u.get_result(true).get_estimate(), 100.0);
    u.pin_mut().reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn union_sums_values_on_collision() {
    let mut a =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    a.pin_mut().update_u64(1, &[1.0, 10.0]);
    let mut b =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    b.pin_mut().update_u64(1, &[2.0, 20.0]);

    let mut u =
        union_ffi::new_array_of_doubles_union(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_sketch(&b);
    let result = u.get_result(true);
    assert_eq!(result.get_num_retained(), 1);
    let values: Vec<f64> = result.entry_values().into_iter().collect();
    assert_eq!(values, vec![3.0, 30.0]);
}

#[test]
fn invalid_lg_k_returns_err() {
    assert!(
        union_ffi::new_array_of_doubles_union(4, sketch_ffi::TupleResizeFactor::X8, 1.0, 1).is_err()
    );
}
