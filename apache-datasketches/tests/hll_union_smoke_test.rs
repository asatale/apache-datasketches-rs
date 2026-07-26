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
