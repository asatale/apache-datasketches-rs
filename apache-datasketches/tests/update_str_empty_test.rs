//! The empty-string no-op, asserted on every `update_str` path.
//!
//! Upstream places this guard inside its `update(const std::string&)`
//! overload, which returns early for an empty string. The shims do not call
//! that overload: constructing a `std::string` from a `rust::Str` heap-copies
//! bytes the caller already owns, so they forward the borrowed pointer and
//! length to the `(data, length)` overload instead. That overload does *not*
//! screen on a zero length — HLL screens on a null pointer, and a `rust::Str`
//! for `""` is non-null with length 0 — so an unguarded forward would hash a
//! zero-length payload and record an item where upstream records nothing.
//!
//! Each shim therefore replicates the guard explicitly, which moves the
//! invariant from upstream's responsibility to ours. These tests are what keep
//! it honest. `hll_sketch_test` and `hll_union_test` already cover the two HLL
//! paths as part of their upstream ports; the HLL cases are repeated here so
//! that all six paths carrying the replicated guard are asserted in one place,
//! and so that deleting a guard fails a test named after the guard.
//!
//! Every case also updates with a non-empty key afterwards, so that a guard
//! that over-triggers (screening out real input) fails just as loudly as one
//! that is missing.

use apache_datasketches::cpc::CpcSketchBuilder;
use apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType};
use apache_datasketches::theta::ThetaSketchBuilder;
use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};
use apache_datasketches::tuple::ArrayOfDoublesSketchBuilder;

#[derive(Clone, Debug, PartialEq)]
struct Doubles(Vec<f64>);

impl TupleSummary for Doubles {
    type Update = [f64];
    fn create(update: &[f64]) -> Self {
        Doubles(update.to_vec())
    }
    fn union_combine(&mut self, other: &Self) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a += *b;
        }
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.union_combine(other);
    }
}

#[test]
fn hll_sketch_update_str_ignores_empty_string() {
    let mut sketch = HllSketch::new(12, TargetHllType::Hll8).unwrap();
    sketch.update_str("");
    assert!(sketch.is_empty(), "empty string must not record an item");

    sketch.update_str("a");
    assert!(!sketch.is_empty(), "guard must not screen out a real key");
}

#[test]
fn hll_union_update_str_ignores_empty_string() {
    let mut union = HllUnion::new(12).unwrap();
    union.update_str("");
    assert!(union.is_empty(), "empty string must not record an item");

    union.update_str("a");
    assert!(!union.is_empty(), "guard must not screen out a real key");
}

#[test]
fn theta_sketch_update_str_ignores_empty_string() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    sketch.update_str("");
    assert!(sketch.is_empty(), "empty string must not record an item");

    sketch.update_str("a");
    assert!(!sketch.is_empty(), "guard must not screen out a real key");
}

#[test]
fn cpc_sketch_update_str_ignores_empty_string() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_str("");
    assert!(sketch.is_empty(), "empty string must not record an item");

    sketch.update_str("a");
    assert!(!sketch.is_empty(), "guard must not screen out a real key");
}

#[test]
fn array_of_doubles_sketch_update_str_ignores_empty_string() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    sketch.update_str("", &[1.0]).unwrap();
    assert!(sketch.is_empty(), "empty key must not record an entry");
    assert_eq!(sketch.get_num_retained(), 0);

    sketch.update_str("a", &[1.0]).unwrap();
    assert!(!sketch.is_empty(), "guard must not screen out a real key");
    assert_eq!(sketch.get_num_retained(), 1);
}

#[test]
fn tuple_generic_sketch_update_str_ignores_empty_string() {
    let mut sketch: TupleSketch<Doubles> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    sketch.update_str("", &[1.0]);
    assert!(sketch.is_empty(), "empty key must not record an entry");
    assert_eq!(sketch.get_num_retained(), 0);

    sketch.update_str("a", &[1.0]);
    assert!(!sketch.is_empty(), "guard must not screen out a real key");
    assert_eq!(sketch.get_num_retained(), 1);
}

/// The values slice is validated even when the key is empty.
///
/// The safe wrapper screens the slice length in Rust before crossing, and the
/// shim keeps its own `check_values_len` ahead of the empty-key guard as a
/// backstop. Both orderings matter: upstream validates the summary before its
/// string overload discards an empty key, so hoisting either check below the
/// guard would quietly start accepting a bad slice whenever the key is empty.
#[test]
fn array_of_doubles_still_validates_values_for_an_empty_key() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new()
        .num_values(2)
        .build()
        .unwrap();
    assert!(
        sketch.update_str("", &[1.0]).is_err(),
        "a 1-value slice against num_values=2 must be rejected even for an empty key"
    );
}
