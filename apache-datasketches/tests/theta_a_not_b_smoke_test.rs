use apache_datasketches::theta::{ThetaAnotB, ThetaSketchBuilder};

#[test]
fn a_not_b_two_theta_sketches() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    for i in 250..750u64 {
        b.update_u64(i);
    }

    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b, true);
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}

#[test]
fn a_not_b_mixed_input_types() {
    let mut a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..500u64 {
        a.update_u64(i);
    }
    let mut b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 250..750u64 {
        b.update_u64(i);
    }
    let b_compact = b.compact(true);

    let a_not_b = ThetaAnotB::new();
    let result = a_not_b.compute(&a, &b_compact, true);
    assert!((result.get_estimate() - 250.0).abs() < 20.0);
}
