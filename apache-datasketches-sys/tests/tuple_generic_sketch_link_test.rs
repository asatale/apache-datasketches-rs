#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi, RawSummaryOps, RustSummary};
use std::any::Any;

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn summary(v: i64) -> RustSummary {
    RustSummary::new(Box::new(Sum(v)))
}

#[test]
fn construct_update_estimate() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &summary(1));
    }
    assert!((sketch.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(sketch.get_num_retained(), 1000);
    assert!(!sketch.is_empty());
}

#[test]
fn repeated_key_combines_rather_than_inserting() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for _ in 0..5 {
        sketch.pin_mut().update_u64(42, &summary(10));
    }
    assert_eq!(
        sketch.get_num_retained(),
        1,
        "same key must not insert twice"
    );
}

#[test]
fn invalid_lg_k_returns_err() {
    assert!(ffi::new_tuple_generic_sketch(4, 8, 1.0).is_err());
}

#[test]
fn invalid_resize_factor_returns_err() {
    assert!(ffi::new_tuple_generic_sketch(12, 3, 1.0).is_err());
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    sketch.pin_mut().update_u64(1, &summary(1));
    assert!(!sketch.is_empty());
    sketch.pin_mut().reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 0);
}

/// At `lg_k = 5` the table starts small (32 slots) and 500 distinct keys force
/// several resize/rehash cycles well past the initial capacity. Every rehash
/// move-constructs each retained `DynSummary`, so this exercises the invariant
/// that `engaged()` stays true and `get()` stays valid across that move (the
/// move constructor added while hardening `DynSummary` in Task 1) — not just
/// that the move compiles, but that the resulting sketch still reports the
/// correct estimate and count after the moves have actually happened.
#[test]
fn rehash_and_resize_preserve_summaries() {
    let mut sketch = ffi::new_tuple_generic_sketch(5, 2, 1.0).unwrap();
    for i in 0..500u64 {
        sketch.pin_mut().update_u64(i, &summary(1));
    }
    assert!(
        sketch.is_estimation_mode(),
        "500 keys at lg_k=5 must trigger sampling/estimation"
    );
    assert!((sketch.get_estimate() - 500.0).abs() / 500.0 < 0.2);
    assert!(sketch.get_num_retained() > 0);
    assert!(!sketch.is_empty());
}
