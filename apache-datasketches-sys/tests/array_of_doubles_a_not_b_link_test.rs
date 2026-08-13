#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_a_not_b::ffi as a_not_b_ffi;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;

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
fn a_not_b_half_overlap_all_four_combinations() {
    let a = sketch(0..1000);
    let b = sketch(500..1500);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc = a_not_b_ffi::new_array_of_doubles_a_not_b();

    assert_eq!(
        calc.compute_sketch_sketch(&a, &b, true).get_estimate(),
        500.0
    );
    assert_eq!(
        calc.compute_sketch_compact(&a, &cb, true).get_estimate(),
        500.0
    );
    assert_eq!(
        calc.compute_compact_sketch(&ca, &b, true).get_estimate(),
        500.0
    );
    assert_eq!(
        calc.compute_compact_compact(&ca, &cb, true).get_estimate(),
        500.0
    );
}

#[test]
fn mixed_type_overloads_preserve_operand_order() {
    // Asymmetric fixture: a - b estimates 500, b - a estimates 0.
    let a = sketch(0..1000);
    let b = sketch(0..500);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc = a_not_b_ffi::new_array_of_doubles_a_not_b();

    // (Sketch, Compact) path.
    assert_eq!(
        calc.compute_sketch_compact(&a, &cb, true).get_estimate(),
        500.0
    );
    assert_eq!(
        calc.compute_sketch_compact(&b, &ca, true).get_estimate(),
        0.0
    );

    // (Compact, Sketch) path.
    assert_eq!(
        calc.compute_compact_sketch(&ca, &b, true).get_estimate(),
        500.0
    );
    assert_eq!(
        calc.compute_compact_sketch(&cb, &a, true).get_estimate(),
        0.0
    );
}

#[test]
fn a_not_b_preserves_num_values_and_values() {
    let mut a =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
            .unwrap();
    a.pin_mut().update_u64(1, &[5.0, 6.0]);
    let b = sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 2)
        .unwrap();

    let calc = a_not_b_ffi::new_array_of_doubles_a_not_b();
    let result = calc.compute_sketch_sketch(&a, &b, true);
    assert_eq!(result.get_num_values(), 2);
    assert_eq!(result.get_num_retained(), 1);
    let values: Vec<f64> = result.entry_values().as_slice().to_vec();
    assert_eq!(values, vec![5.0, 6.0]);
}
