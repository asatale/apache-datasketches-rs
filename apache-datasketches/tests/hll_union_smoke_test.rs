use apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType};

#[test]
fn union_two_overlapping_sketches() {
    let num = 10_000u64;
    let overlap = num / 2;

    let mut sketch1 = HllSketch::new(11, TargetHllType::Hll4).unwrap();
    for key in 0..num {
        sketch1.update_u64(key);
    }

    let mut sketch2 = HllSketch::new(11, TargetHllType::Hll4).unwrap();
    for key in overlap..(num + overlap) {
        sketch2.update_u64(key);
    }

    let mut u = HllUnion::new(11).unwrap();
    u.update_sketch(&sketch1);
    u.update_sketch(&sketch2);

    let result = u.get_result(TargetHllType::Hll4);
    let expected = num as f64 * 1.5;
    assert!(
        (result.get_estimate() - expected).abs() < expected * 0.05,
        "estimate was {}",
        result.get_estimate()
    );
}

#[test]
fn union_serialize_matches_get_result() {
    let mut u = HllUnion::new(11).unwrap();
    let mut sketch = HllSketch::new(11, TargetHllType::Hll4).unwrap();
    for key in 0..5_000u64 {
        sketch.update_u64(key);
    }
    u.update_sketch(&sketch);

    for &tgt_type in &[TargetHllType::Hll4, TargetHllType::Hll6, TargetHllType::Hll8] {
        let compact_bytes = u.serialize_compact(tgt_type);
        let updatable_bytes = u.serialize_updatable(tgt_type);

        let from_compact = HllSketch::deserialize(&compact_bytes).unwrap();
        let from_updatable = HllSketch::deserialize(&updatable_bytes).unwrap();
        let direct = u.get_result(tgt_type);

        assert_eq!(from_compact.get_estimate(), direct.get_estimate());
        assert_eq!(from_updatable.get_estimate(), direct.get_estimate());
    }
}
