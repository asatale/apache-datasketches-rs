use apache_datasketches::theta::{CompactThetaSketch, ThetaSketchBuilder};

#[test]
fn compact_v3_round_trip() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    let restored = CompactThetaSketch::deserialize(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());
    assert_eq!(compact.get_num_retained(), restored.get_num_retained());
}

#[test]
fn compact_v4_round_trip() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compressed();
    let restored = CompactThetaSketch::deserialize_compressed(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());
    assert_eq!(compact.get_num_retained(), restored.get_num_retained());
}

#[test]
fn deserialize_garbage_is_err() {
    assert!(CompactThetaSketch::deserialize(&[0u8; 3]).is_err());
}
