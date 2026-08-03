use apache_datasketches::tuple::ArrayOfDoublesSketchBuilder;

#[test]
fn construct_update_estimate() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i, &[1.0]).unwrap();
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
    assert_eq!(sketch.get_num_values(), 1);
}

#[test]
fn invalid_lg_k_is_err() {
    assert!(ArrayOfDoublesSketchBuilder::new().lg_k(4).build().is_err());
}

#[test]
fn num_values_zero_is_err() {
    assert!(ArrayOfDoublesSketchBuilder::new().num_values(0).build().is_err());
}

#[test]
fn wrong_length_values_is_err() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    assert!(sketch.update_u64(1, &[1.0]).is_err());
    assert!(sketch.update_u64(1, &[1.0, 2.0, 3.0]).is_err());
    assert!(sketch.update_u64(1, &[1.0, 2.0]).is_ok());
}

#[test]
fn entries_yields_hash_and_values() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().num_values(3).build().unwrap();
    sketch.update_u64(7, &[1.0, 2.0, 3.0]).unwrap();
    sketch.update_u64(7, &[1.0, 2.0, 3.0]).unwrap();
    let entries: Vec<(u64, Vec<f64>)> = sketch.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, vec![2.0, 4.0, 6.0]);
}

#[test]
fn sketch_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<apache_datasketches::tuple::ArrayOfDoublesSketch>();
}
