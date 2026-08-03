#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_intersection::ffi as intersection_ffi;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;

fn sketch(
    num_values: u8,
    keys: std::ops::Range<u64>,
    values: &[f64],
) -> cxx::UniquePtr<sketch_ffi::ArrayOfDoublesSketchShim> {
    let mut s = sketch_ffi::new_array_of_doubles_sketch(
        12,
        sketch_ffi::TupleResizeFactor::X8,
        1.0,
        num_values,
    )
    .unwrap();
    for key in keys {
        s.pin_mut().update_u64(key, values);
    }
    s
}

#[test]
fn intersection_half_overlap() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 500..1500, &[1.0]);
    let mut i = intersection_ffi::new_array_of_doubles_intersection(1);
    assert!(!i.has_result());
    i.pin_mut().update_with_sketch(&a);
    i.pin_mut().update_with_sketch(&b);
    assert!(i.has_result());
    let result = i.get_result(true).unwrap();
    assert_eq!(result.get_estimate(), 500.0);
    assert_eq!(result.get_num_values(), 1);
}

#[test]
fn intersection_sums_values_on_collision() {
    let a = sketch(2, 0..1, &[1.0, 10.0]);
    let b = sketch(2, 0..1, &[2.0, 20.0]);
    let mut i = intersection_ffi::new_array_of_doubles_intersection(2);
    i.pin_mut().update_with_sketch(&a);
    i.pin_mut().update_with_compact(&b.compact(true));
    let result = i.get_result(true).unwrap();
    assert_eq!(result.get_num_retained(), 1);
    let values: Vec<f64> = result.entry_values().into_iter().collect();
    assert_eq!(values, vec![3.0, 30.0]);
}

#[test]
fn get_result_without_update_returns_err() {
    let i = intersection_ffi::new_array_of_doubles_intersection(1);
    assert!(!i.has_result());
    assert!(i.get_result(true).is_err());
}
