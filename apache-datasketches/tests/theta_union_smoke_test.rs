use apache_datasketches::theta::{ThetaSketchBuilder, ThetaUnionBuilder};

#[test]
fn union_two_theta_sketches() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    for i in 250..750u64 {
        b.update_u64(i);
    }

    let mut union_ = ThetaUnionBuilder::new().lg_k(12).build().unwrap();
    union_.update(&a);
    union_.update(&b);

    let result = union_.get_result(true);
    assert!((result.get_estimate() - 750.0).abs() < 20.0);
}

#[test]
fn union_mixed_input_types() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    let a_compact = a.compact(true);
    let a_bytes = a_compact.serialize_compact();
    let a_wrapped = apache_datasketches::theta::WrappedCompactThetaSketch::wrap(&a_bytes).unwrap();

    let mut union_ = ThetaUnionBuilder::new().lg_k(12).build().unwrap();
    union_.update(&a);
    union_.update(&a_compact);
    union_.update(&a_wrapped);

    let result = union_.get_result(true);
    assert!((result.get_estimate() - 500.0).abs() < 10.0);
}
