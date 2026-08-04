//! Ported 1:1 from upstream datasketches-cpp's
//! `tuple/test/array_of_doubles_sketch_test.cpp`. Upstream covers the sketch,
//! union, intersection, and a-not-b for this family from that single file
//! (unlike Theta, which has one test file per class), so this file does too.
//!
//! Deviations from upstream, and why:
//!
//! - Upstream has three separate serialization test cases ("stream serialize
//!   deserialize", "bytes to stream serialize deserialize", and "bytes
//!   serialize deserialize") because C++ exposes both `std::ostream` and
//!   byte-vector overloads and both must round-trip through each other. These
//!   bindings expose exactly one byte-oriented `serialize()`/`deserialize()`
//!   pair (there is no stream API to bind — `&[u8]`/`Vec<u8>` is the idiomatic
//!   Rust equivalent and the wire format is identical either way), so the
//!   three cases collapse into the single `serialize_deserialize_estimation_mode`
//!   test below.
//! - Upstream's `builder(2)` relies on implicit conversion from `int` to the
//!   update policy; here that is the explicit `.num_values(2)` builder setter.
//! - Upstream iterates entries with `begin()`/`end()` and reads
//!   `entry.second[i]`; here `entries()` yields owned `(u64, Vec<f64>)` pairs
//!   (cxx cannot hand back a live C++ iterator).
//! - `update()` returns `Result` here because the values-length check has no
//!   C++ exception to delegate to; upstream's equivalent call is infallible.

use apache_datasketches::tuple::{
    ArrayOfDoublesAnotB, ArrayOfDoublesIntersection, ArrayOfDoublesSketch,
    ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder, CompactArrayOfDoublesSketch,
};

/// Upstream: `TEST_CASE("aod sketch: reset")`.
#[test]
fn sketch_reset() {
    let mut sketch = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    sketch.update_i32(1, &[1.0]).unwrap();
    assert!(!sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 1);

    sketch.reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 0);
}

/// Upstream: `TEST_CASE("aod sketch: stream serialize deserialize - estimation
/// mode")`, merged with the two byte-oriented serialization cases (see the
/// module comment).
#[test]
fn serialize_deserialize_estimation_mode() {
    let mut update_sketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    for i in 0..8192i32 {
        update_sketch.update_i32(i, &[1.0, 2.0]).unwrap();
    }
    assert!(!update_sketch.is_empty());
    assert!(update_sketch.is_estimation_mode());
    assert_eq!(update_sketch.get_num_values(), 2);

    let compact_sketch = update_sketch.compact(true);
    let bytes = compact_sketch.serialize();
    let deserialized = CompactArrayOfDoublesSketch::deserialize(&bytes).unwrap();

    assert_eq!(deserialized.get_num_values(), compact_sketch.get_num_values());
    assert_eq!(deserialized.is_empty(), compact_sketch.is_empty());
    assert_eq!(deserialized.is_ordered(), compact_sketch.is_ordered());
    assert_eq!(
        deserialized.is_estimation_mode(),
        compact_sketch.is_estimation_mode()
    );
    assert_eq!(deserialized.get_num_retained(), compact_sketch.get_num_retained());
    assert_eq!(deserialized.get_theta(), compact_sketch.get_theta());
    assert_eq!(deserialized.get_estimate(), compact_sketch.get_estimate());
    assert_eq!(
        deserialized.get_lower_bound(1).unwrap(),
        compact_sketch.get_lower_bound(1).unwrap()
    );
    assert_eq!(
        deserialized.get_upper_bound(1).unwrap(),
        compact_sketch.get_upper_bound(1).unwrap()
    );

    // Upstream compares the two sketches entry by entry via parallel
    // iteration, checking hash, value[0], and value[1].
    let expected: Vec<(u64, Vec<f64>)> = compact_sketch.entries().collect();
    let actual: Vec<(u64, Vec<f64>)> = deserialized.entries().collect();
    assert_eq!(expected.len(), compact_sketch.get_num_retained() as usize);
    assert_eq!(expected, actual);
    for (_, values) in &expected {
        assert_eq!(values.as_slice(), &[1.0, 2.0]);
    }

    // Upstream also iterates the update sketch and the compact sketch
    // together; the compact one is ordered, so compare against the update
    // sketch's entries sorted by hash.
    let mut from_update: Vec<(u64, Vec<f64>)> = update_sketch.entries().collect();
    from_update.sort_by_key(|(hash, _)| *hash);
    assert_eq!(from_update, expected);
}

/// Upstream: `TEST_CASE("aod union: half overlap")`.
#[test]
fn union_half_overlap() {
    let mut sketch1 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 0..1000i32 {
        sketch1.update_i32(i, &[1.0]).unwrap();
    }
    let mut sketch2 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 500..1500i32 {
        sketch2.update_i32(i, &[1.0]).unwrap();
    }

    let mut u = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    u.update(&sketch1).unwrap();
    u.update(&sketch2).unwrap();
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);

    u.reset();
    assert!(u.get_result(true).is_empty());
}

/// Upstream: `TEST_CASE("aod intersection: half overlap")`. Upstream notes
/// there is no default intersection policy and picks the union's sum policy
/// for testing; these bindings make that same choice permanently for v1.
#[test]
fn intersection_half_overlap() {
    let mut sketch1 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 0..1000i32 {
        sketch1.update_i32(i, &[1.0]).unwrap();
    }
    let mut sketch2 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 500..1500i32 {
        sketch2.update_i32(i, &[1.0]).unwrap();
    }

    let mut intersection = ArrayOfDoublesIntersection::new(1).unwrap();
    intersection.update(&sketch1).unwrap();
    intersection.update(&sketch2).unwrap();
    assert_eq!(intersection.get_result(true).unwrap().get_estimate(), 500.0);
}

/// Upstream: `TEST_CASE("aod a-not-b: half overlap")`.
#[test]
fn a_not_b_half_overlap() {
    let mut sketch1 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 0..1000i32 {
        sketch1.update_i32(i, &[1.0]).unwrap();
    }
    let mut sketch2 = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 500..1500i32 {
        sketch2.update_i32(i, &[1.0]).unwrap();
    }

    let a_not_b = ArrayOfDoublesAnotB::new();
    let result = a_not_b.compute(&sketch1, &sketch2, true).unwrap();
    assert_eq!(result.get_estimate(), 500.0);
}

/// Not from upstream: confirms an empty sketch round-trips, which the
/// serialized format handles via its IS_EMPTY flag byte.
#[test]
fn empty_sketch_round_trips() {
    let sketch: ArrayOfDoublesSketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    let compact = sketch.compact(true);
    assert!(compact.is_empty());
    assert_eq!(compact.get_estimate(), 0.0);

    let restored = CompactArrayOfDoublesSketch::deserialize(&compact.serialize()).unwrap();
    assert!(restored.is_empty());
    assert_eq!(restored.get_num_values(), 2);
    assert_eq!(restored.get_num_retained(), 0);
    assert_eq!(restored.entries().count(), 0);
}
