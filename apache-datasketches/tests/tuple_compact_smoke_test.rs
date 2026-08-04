use apache_datasketches::tuple::{ArrayOfDoublesSketchBuilder, CompactArrayOfDoublesSketch};

fn build_sketch(
    num_values: u8,
    keys: std::ops::Range<u64>,
) -> apache_datasketches::tuple::ArrayOfDoublesSketch {
    let mut sketch = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    let values: Vec<f64> = (0..num_values).map(|i| (i + 1) as f64).collect();
    for key in keys {
        sketch.update_u64(key, &values).unwrap();
    }
    sketch
}

#[test]
fn compact_preserves_estimate_and_num_values() {
    let sketch = build_sketch(2, 0..1000);
    let compact = sketch.compact(true);
    assert!((compact.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(compact.get_num_values(), 2);
    assert_eq!(compact.get_num_retained(), 1000);
    assert!(compact.is_ordered());
    assert!(!compact.is_empty());
    assert!(!compact.is_estimation_mode());
    assert_eq!(compact.get_theta(), 1.0);
    assert!(compact.get_lower_bound(1).unwrap() <= compact.get_estimate());
    assert!(compact.get_upper_bound(1).unwrap() >= compact.get_estimate());
}

#[test]
fn serialize_deserialize_round_trip() {
    let sketch = build_sketch(3, 0..500);
    let compact = sketch.compact(true);
    let bytes = compact.serialize();
    let restored = CompactArrayOfDoublesSketch::deserialize(&bytes).unwrap();
    assert_eq!(restored.get_num_values(), 3);
    assert_eq!(restored.get_num_retained(), compact.get_num_retained());
    let before: Vec<(u64, Vec<f64>)> = compact.entries().collect();
    let after: Vec<(u64, Vec<f64>)> = restored.entries().collect();
    assert_eq!(before, after);
}

#[test]
fn deserialize_garbage_is_err() {
    assert!(CompactArrayOfDoublesSketch::deserialize(&[0u8; 8]).is_err());
}

#[test]
fn ordered_entries_are_sorted_by_hash() {
    let sketch = build_sketch(1, 0..200);
    let compact = sketch.compact(true);
    let hashes: Vec<u64> = compact.entries().map(|(h, _)| h).collect();
    assert_eq!(hashes.len(), 200);
    let mut sorted = hashes.clone();
    sorted.sort_unstable();
    assert_eq!(hashes, sorted);
}

#[test]
fn compact_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<CompactArrayOfDoublesSketch>();
}
