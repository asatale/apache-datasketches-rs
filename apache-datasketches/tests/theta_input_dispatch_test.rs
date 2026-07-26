// apache-datasketches/tests/theta_input_dispatch_test.rs
//! New, non-upstream tests: systematically exercise every ThetaInput
//! dispatch combination (Sketch/Compact/Wrapped x Sketch/Compact/Wrapped)
//! for each set operation and for jaccard_similarity, since production
//! code contains a hand-written match arm per combination (see
//! ThetaUnion::update, ThetaIntersection::update, ThetaAnotB::compute,
//! and jaccard_similarity) that would not otherwise be caught by a
//! missing/mismatched arm.
use apache_datasketches::theta::{
    jaccard_similarity, ThetaAnotB, ThetaIntersection, ThetaSketchBuilder, ThetaUnionBuilder,
    WrappedCompactThetaSketch,
};

fn fixture() -> (
    apache_datasketches::theta::ThetaSketch,
    apache_datasketches::theta::CompactThetaSketch,
    Vec<u8>,
) {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    (sketch, compact, bytes)
}

#[test]
fn union_accepts_all_nine_combinations() {
    let (sketch, compact, bytes) = fixture();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    for _ in 0..1 {
        // Exercise all 3 inputs as the *first* update, and all 3 as the
        // *second*, covering the 3x3 matrix across two calls per union.
        let combos: [&dyn Fn(&mut apache_datasketches::theta::ThetaUnion); 3] = [
            &|u| u.update(&sketch),
            &|u| u.update(&compact),
            &|u| u.update(&wrapped),
        ];
        for first in &combos {
            for second in &combos {
                let mut union_ = ThetaUnionBuilder::new().lg_k(12).build().unwrap();
                first(&mut union_);
                second(&mut union_);
                let result = union_.get_result(true);
                assert!((result.get_estimate() - 500.0).abs() < 20.0);
            }
        }
    }
}

#[test]
fn intersection_accepts_all_nine_combinations() {
    let (sketch, compact, bytes) = fixture();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    let combos: [&dyn Fn(&mut ThetaIntersection); 3] = [
        &|u| u.update(&sketch),
        &|u| u.update(&compact),
        &|u| u.update(&wrapped),
    ];
    for first in &combos {
        for second in &combos {
            let mut isect = ThetaIntersection::new();
            first(&mut isect);
            second(&mut isect);
            let result = isect.get_result(true).unwrap();
            assert!((result.get_estimate() - 500.0).abs() < 20.0);
        }
    }
}

#[test]
fn a_not_b_accepts_all_nine_combinations() {
    let (sketch, compact, bytes) = fixture();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    // a-not-b of a set with itself, across every (a, b) type combination,
    // is always empty regardless of which concrete types are used.
    let a_not_b = ThetaAnotB::new();
    assert!(a_not_b.compute(&sketch, &sketch, true).is_empty());
    assert!(a_not_b.compute(&sketch, &compact, true).is_empty());
    assert!(a_not_b.compute(&sketch, &wrapped, true).is_empty());
    assert!(a_not_b.compute(&compact, &sketch, true).is_empty());
    assert!(a_not_b.compute(&compact, &compact, true).is_empty());
    assert!(a_not_b.compute(&compact, &wrapped, true).is_empty());
    assert!(a_not_b.compute(&wrapped, &sketch, true).is_empty());
    assert!(a_not_b.compute(&wrapped, &compact, true).is_empty());
    assert!(a_not_b.compute(&wrapped, &wrapped, true).is_empty());
}

#[test]
fn jaccard_accepts_all_nine_combinations() {
    let (sketch, compact, bytes) = fixture();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    // jaccard_similarity of a set with itself is always exactly 1.0,
    // regardless of which concrete types are used for the two arguments.
    let pairs_estimate = [
        jaccard_similarity(&sketch, &sketch).estimate,
        jaccard_similarity(&sketch, &compact).estimate,
        jaccard_similarity(&sketch, &wrapped).estimate,
        jaccard_similarity(&compact, &sketch).estimate,
        jaccard_similarity(&compact, &compact).estimate,
        jaccard_similarity(&compact, &wrapped).estimate,
        jaccard_similarity(&wrapped, &sketch).estimate,
        jaccard_similarity(&wrapped, &compact).estimate,
        jaccard_similarity(&wrapped, &wrapped).estimate,
    ];
    for estimate in pairs_estimate {
        assert!((estimate - 1.0).abs() < 1e-9);
    }
}
