use apache_datasketches::theta::{ThetaIntersection, ThetaSketchBuilder};
use apache_datasketches::SketchError;

#[test]
fn intersect_two_theta_sketches() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    for i in 250..750u64 {
        b.update_u64(i);
    }

    let mut isect = ThetaIntersection::new();
    isect.update(&a);
    isect.update(&b);

    let result = isect.get_result(true).unwrap();
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}

#[test]
fn get_result_before_update_is_empty_intersection_error() {
    let isect = ThetaIntersection::new();
    match isect.get_result(true) {
        Err(SketchError::EmptyIntersection) => {}
        Err(other) => panic!("expected EmptyIntersection, got {:?}", other),
        Ok(_) => panic!("expected EmptyIntersection, got Ok"),
    }
}
