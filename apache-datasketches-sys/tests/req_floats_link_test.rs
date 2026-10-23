#![cfg(feature = "req")]

use apache_datasketches_sys::req_floats::ffi;

#[test]
fn construct_update_query_roundtrip() {
    let mut sketch = ffi::req_floats_new(12, true).unwrap();
    for i in 1..=10u8 {
        sketch.pin_mut().update(f32::from(i));
    }
    assert_eq!(sketch.get_n(), 10);
    assert_eq!(sketch.get_k(), 12);
    assert!(sketch.is_hra());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_num_retained(), 10);
    assert_eq!(sketch.get_min_item().unwrap(), 1.0f32);
    assert_eq!(sketch.get_max_item().unwrap(), 10.0f32);
    assert_eq!(sketch.get_rank(5.0f32, true).unwrap(), 0.5);
    assert_eq!(
        sketch.get_cdf(&[5.0f32], true).unwrap().as_slice(),
        &[0.5, 1.0]
    );
    assert!(sketch.get_rank_lower_bound(0.5, 2) <= 0.5);
    assert!(sketch.get_rank_upper_bound(0.5, 2) >= 0.5);

    let restored = ffi::req_floats_deserialize(sketch.serialize().as_slice()).unwrap();
    assert_eq!(restored.get_n(), 10);

    let view = sketch.sorted_view();
    assert_eq!(view.size(), 10);
    assert_eq!(view.items().as_slice()[0], 1.0f32);
    assert_eq!(view.cumulative_weights().as_slice()[9], 10);

    let mut copy = sketch.clone_shim();
    copy.pin_mut().merge(&sketch).unwrap();
    assert_eq!(copy.get_n(), 20);
}

#[test]
fn merging_hra_into_lra_returns_err() {
    let mut hra = ffi::req_floats_new(12, true).unwrap();
    let mut lra = ffi::req_floats_new(12, false).unwrap();
    hra.pin_mut().update(1.0f32);
    lra.pin_mut().update(1.0f32);
    assert!(lra.pin_mut().merge(&hra).is_err());
}

#[test]
fn query_on_empty_returns_err() {
    let sketch = ffi::req_floats_new(12, true).unwrap();
    assert!(sketch.get_quantile(0.5, true).is_err());
}

#[test]
fn out_of_range_rank_returns_err() {
    let mut sketch = ffi::req_floats_new(12, true).unwrap();
    for i in 1..=10u8 {
        sketch.pin_mut().update(f32::from(i));
    }
    let view = sketch.sorted_view();
    for rank in [f64::NAN, -0.1, 1.1, f64::INFINITY] {
        assert!(
            sketch.get_quantile(rank, true).is_err(),
            "sketch rank {rank}"
        );
        assert!(view.get_quantile(rank, true).is_err(), "view rank {rank}");
    }
    for rank in [0.0, 1.0] {
        assert!(sketch.get_quantile(rank, true).is_ok());
        assert!(view.get_quantile(rank, true).is_ok());
    }
}

#[test]
fn a_priori_rank_error_is_finite_and_positive() {
    let err = ffi::req_floats_a_priori_rank_error(12, 0.5, true, 1000);
    assert!(err.is_finite() && err > 0.0, "error was {err}");
}

#[test]
fn empty_sketch_retained_items_and_weights_are_empty() {
    let sketch = ffi::req_floats_new(12, true).unwrap();
    assert!(sketch.retained_items().as_slice().is_empty());
    assert!(sketch.retained_weights().as_slice().is_empty());
}
