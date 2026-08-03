use apache_datasketches::tuple::{
    ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnion,
    ArrayOfDoublesUnionBuilder,
};

fn sketch(num_values: u8, keys: std::ops::Range<u64>) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    let values: Vec<f64> = (0..num_values).map(|i| (i + 1) as f64).collect();
    for key in keys {
        s.update_u64(key, &values).unwrap();
    }
    s
}

#[test]
fn union_half_overlap() {
    let a = sketch(1, 0..1000);
    let b = sketch(1, 500..1500);
    let mut u = ArrayOfDoublesUnionBuilder::new().lg_k(12).build().unwrap();
    u.update(&a).unwrap();
    u.update(&b).unwrap();
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);
}

#[test]
fn union_accepts_both_input_types() {
    let a = sketch(2, 0..100);
    let b = sketch(2, 50..150).compact(true);
    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    u.update(&a).unwrap();
    u.update(&b).unwrap();
    let result = u.get_result(true);
    assert_eq!(result.get_estimate(), 150.0);
    assert_eq!(result.get_num_values(), 2);
}

#[test]
fn union_sums_values_on_collision() {
    let mut a = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    a.update_u64(1, &[1.0, 10.0]).unwrap();
    let mut b = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    b.update_u64(1, &[2.0, 20.0]).unwrap();

    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    u.update(&a).unwrap();
    u.update(&b).unwrap();
    let entries: Vec<(u64, Vec<f64>)> = u.get_result(true).entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, vec![3.0, 30.0]);
}

#[test]
fn union_reset_empties_result() {
    let a = sketch(1, 0..100);
    let mut u = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    u.update(&a).unwrap();
    assert!(!u.get_result(true).is_empty());
    u.reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn mismatched_num_values_is_err() {
    let a = sketch(3, 0..10);
    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    assert_eq!(u.get_num_values(), 2);
    assert!(u.update(&a).is_err());
    assert!(u.update(&a.compact(true)).is_err());
}

#[test]
fn invalid_config_is_err() {
    assert!(ArrayOfDoublesUnionBuilder::new().lg_k(4).build().is_err());
    assert!(ArrayOfDoublesUnionBuilder::new().num_values(0).build().is_err());
}

#[test]
fn union_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<ArrayOfDoublesUnion>();
}
