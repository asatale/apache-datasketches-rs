#![cfg(feature = "kll")]

use apache_datasketches_sys::kll_doubles::ffi;

#[test]
fn construct_update_query_roundtrip() {
    let mut sketch = ffi::kll_doubles_new(200).unwrap();
    for i in 1..=100 {
        sketch.pin_mut().update(f64::from(i));
    }
    assert_eq!(sketch.get_n(), 100);
    assert_eq!(sketch.get_k(), 200);
    assert_eq!(sketch.get_num_retained(), 100);
    assert_eq!(sketch.get_min_item().unwrap(), 1.0);
    assert_eq!(sketch.get_max_item().unwrap(), 100.0);
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), 50.0);
    assert_eq!(sketch.get_rank(50.0, true).unwrap(), 0.5);
    assert_eq!(
        sketch.get_cdf(&[50.0], true).unwrap().as_slice(),
        &[0.5, 1.0]
    );

    let restored = ffi::kll_doubles_deserialize(sketch.serialize().as_slice()).unwrap();
    assert_eq!(restored.get_n(), 100);

    let view = sketch.sorted_view();
    assert_eq!(view.size(), 100);
    assert_eq!(view.items().as_slice()[0], 1.0);
    assert_eq!(view.cumulative_weights().as_slice()[99], 100);

    let copy = sketch.clone_shim();
    assert_eq!(sketch.ks_delta(&copy), 0.0);
}

#[test]
fn invalid_k_returns_err() {
    assert!(ffi::kll_doubles_new(7).is_err());
}

#[test]
fn query_on_empty_returns_err() {
    let sketch = ffi::kll_doubles_new(200).unwrap();
    assert!(sketch.get_quantile(0.5, true).is_err());
}

#[test]
fn static_rank_error_matches_instance() {
    let sketch = ffi::kll_doubles_new(200).unwrap();
    assert_eq!(
        ffi::kll_doubles_normalized_rank_error(200, false),
        sketch.get_normalized_rank_error(false)
    );
}

#[test]
fn out_of_range_rank_returns_err() {
    let mut sketch = ffi::kll_doubles_new(200).unwrap();
    for i in 1..=100u8 {
        sketch.pin_mut().update(f64::from(i));
    }
    let view = sketch.sorted_view();
    for r in [f64::NAN, -0.1, 1.1, f64::INFINITY] {
        assert!(sketch.get_quantile(r, true).is_err(), "sketch rank {r}");
        assert!(view.get_quantile(r, true).is_err(), "view rank {r}");
    }
    for r in [0.0, 1.0] {
        assert!(sketch.get_quantile(r, true).is_ok(), "sketch rank {r}");
        assert!(view.get_quantile(r, true).is_ok(), "view rank {r}");
    }
}
