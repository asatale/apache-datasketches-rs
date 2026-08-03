use apache_datasketches::tuple::{
    ArrayOfDoublesAnotB, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder,
};

fn sketch(num_values: u8, keys: std::ops::Range<u64>, values: &[f64]) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    for key in keys {
        s.update_u64(key, values).unwrap();
    }
    s
}

#[test]
fn a_not_b_half_overlap_all_four_combinations() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 500..1500, &[1.0]);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc = ArrayOfDoublesAnotB::new();

    assert_eq!(calc.compute(&a, &b, true).unwrap().get_estimate(), 500.0);
    assert_eq!(calc.compute(&a, &cb, true).unwrap().get_estimate(), 500.0);
    assert_eq!(calc.compute(&ca, &b, true).unwrap().get_estimate(), 500.0);
    assert_eq!(calc.compute(&ca, &cb, true).unwrap().get_estimate(), 500.0);
}

#[test]
fn mixed_type_overloads_preserve_operand_order() {
    // Asymmetric fixture: a - b estimates 500, b - a estimates 0.
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 0..500, &[1.0]);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc = ArrayOfDoublesAnotB::new();

    // (Sketch, Compact) path.
    assert_eq!(calc.compute(&a, &cb, true).unwrap().get_estimate(), 500.0);
    assert_eq!(calc.compute(&b, &ca, true).unwrap().get_estimate(), 0.0);

    // (Compact, Sketch) path.
    assert_eq!(calc.compute(&ca, &b, true).unwrap().get_estimate(), 500.0);
    assert_eq!(calc.compute(&cb, &a, true).unwrap().get_estimate(), 0.0);
}

#[test]
fn a_not_b_preserves_values() {
    let a = sketch(2, 0..1, &[5.0, 6.0]);
    let b = sketch(2, 100..101, &[1.0, 1.0]);
    let calc = ArrayOfDoublesAnotB::new();
    let result = calc.compute(&a, &b, true).unwrap();
    let entries: Vec<(u64, Vec<f64>)> = result.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, vec![5.0, 6.0]);
    assert_eq!(result.get_num_values(), 2);
}

#[test]
fn mismatched_num_values_is_err() {
    let a = sketch(2, 0..10, &[1.0, 2.0]);
    let b = sketch(3, 0..10, &[1.0, 2.0, 3.0]);
    let calc = ArrayOfDoublesAnotB::new();
    assert!(calc.compute(&a, &b, true).is_err());
    assert!(calc.compute(&b, &a, true).is_err());
}

#[test]
fn a_not_b_is_send_and_default() {
    fn assert_send<T: Send>() {}
    assert_send::<ArrayOfDoublesAnotB>();
    let _ = ArrayOfDoublesAnotB::default();
}
