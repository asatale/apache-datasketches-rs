use apache_datasketches::tuple::{
    ArrayOfDoublesIntersection, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder,
};
use apache_datasketches::SketchError;

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
fn intersection_half_overlap() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 500..1500, &[1.0]);
    let mut i = ArrayOfDoublesIntersection::new(1).unwrap();
    i.update(&a).unwrap();
    i.update(&b).unwrap();
    assert_eq!(i.get_result(true).unwrap().get_estimate(), 500.0);
}

#[test]
fn intersection_accepts_both_input_types_and_sums_values() {
    let a = sketch(2, 0..1, &[1.0, 10.0]);
    let b = sketch(2, 0..1, &[2.0, 20.0]).compact(true);
    let mut i = ArrayOfDoublesIntersection::new(2).unwrap();
    i.update(&a).unwrap();
    i.update(&b).unwrap();
    let entries: Vec<(u64, Vec<f64>)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, vec![3.0, 30.0]);
}

#[test]
fn get_result_before_update_is_empty_intersection_err() {
    let i = ArrayOfDoublesIntersection::new(1).unwrap();
    assert!(!i.has_result());
    assert!(matches!(
        i.get_result(true),
        Err(SketchError::EmptyIntersection)
    ));
}

#[test]
fn mismatched_num_values_is_err() {
    let a = sketch(3, 0..10, &[1.0, 2.0, 3.0]);
    let mut i = ArrayOfDoublesIntersection::new(2).unwrap();
    assert_eq!(i.get_num_values(), 2);
    assert!(i.update(&a).is_err());
    assert!(i.update(&a.compact(true)).is_err());
}

#[test]
fn num_values_zero_is_err() {
    assert!(ArrayOfDoublesIntersection::new(0).is_err());
}

#[test]
fn intersection_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<ArrayOfDoublesIntersection>();
}
