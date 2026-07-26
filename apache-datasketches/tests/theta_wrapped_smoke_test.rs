use apache_datasketches::theta::{ThetaSketchBuilder, WrappedCompactThetaSketch};

#[test]
fn wrap_bytes_and_query() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();

    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), wrapped.get_estimate());
}

#[test]
fn wrap_garbage_is_err() {
    assert!(WrappedCompactThetaSketch::wrap(&[0u8; 2]).is_err());
}
