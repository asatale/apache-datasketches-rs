// apache-datasketches/tests/tuple_seed_hash_test.rs
//! Seed-hash handling for serialized ArrayOfDoubles images.
//!
//! Upstream's `compact_array_tuple_sketch::deserialize` checks the seed hash
//! only when the image is flagged `HAS_ENTRIES` (array_tuple_sketch_impl.hpp
//! :156 and :199 at tag 5.3.0). An image that is not flagged empty but has
//! zero entries (theta < 1) and carries a foreign seed hash therefore
//! deserialized successfully. The set operations do check it, and throw "seed
//! hash mismatch" from bridge functions that are not declared `Result`, so
//! passing such a sketch to a-not-b (as A), union, intersection, or Jaccard
//! aborted the process. This crate never exposes a seed (every sketch uses
//! upstream's `DEFAULT_SEED`), so the deserialize shim now rejects any
//! non-empty image whose seed hash is not the default one. These tests pin
//! that, and pin that genuine images (including empty images, whose seed hash
//! is meaningless and is 0 in 5.3.0) are still accepted.
//!
//! The crafted images are built by hand-editing the seed-hash bytes (offset
//! 6..8, little-endian u16) of a genuine serialized sketch. 0x05b6 is
//! `compute_seed_hash(123)`, a foreign seed's hash.
use apache_datasketches::error::SketchError;
use apache_datasketches::tuple::{
    array_of_doubles_jaccard_similarity, ArrayOfDoublesAnotB, ArrayOfDoublesIntersection,
    ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder,
    CompactArrayOfDoublesSketch,
};

const FOREIGN_SEED_HASH: u16 = 0x05b6;
const SEED_HASH_OFFSET: std::ops::Range<usize> = 6..8;

fn sketch(keys: std::ops::Range<u64>) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for k in keys {
        s.update_u64(k, &[1.0]).unwrap();
    }
    s
}

/// A genuine compact sketch that is not empty but retains zero entries: one
/// update with sampling probability 1e-6, so theta < 1 and the only hash is
/// above it.
fn non_empty_zero_entries() -> CompactArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new().p(1e-6).build().unwrap();
    s.update_u64(1, &[1.0]).unwrap();
    assert!(!s.is_empty());
    assert_eq!(s.get_num_retained(), 0);
    assert!(s.get_theta() < 1.0);
    s.compact(true)
}

fn with_seed_hash(mut bytes: Vec<u8>, seed_hash: u16) -> Vec<u8> {
    bytes[SEED_HASH_OFFSET].copy_from_slice(&seed_hash.to_le_bytes());
    bytes
}

fn assert_deserialization_err(bytes: &[u8]) {
    match CompactArrayOfDoublesSketch::deserialize(bytes) {
        Err(SketchError::Deserialization(_)) => {}
        Err(e) => panic!("expected SketchError::Deserialization, got {e:?}"),
        Ok(_) => panic!("expected SketchError::Deserialization, got Ok"),
    }
}

#[test]
fn non_empty_zero_entries_with_foreign_seed_hash_is_rejected() {
    let bytes = non_empty_zero_entries().serialize();
    assert_eq!(bytes.len(), 16, "preamble long plus theta, no entries");
    assert_deserialization_err(&with_seed_hash(bytes, FOREIGN_SEED_HASH));
}

#[test]
fn non_empty_zero_entries_with_zero_seed_hash_is_rejected() {
    // 0 is what 5.3.0 writes for an empty sketch; it is still foreign for a
    // non-empty one.
    let bytes = non_empty_zero_entries().serialize();
    assert_deserialization_err(&with_seed_hash(bytes, 0));
}

#[test]
fn non_empty_with_entries_and_foreign_seed_hash_is_rejected() {
    // Upstream already rejects this case itself; pinned so the validator's
    // rule ("not flagged empty") covers both sub-cases.
    let bytes = sketch(0..100).compact(true).serialize();
    assert_deserialization_err(&with_seed_hash(bytes, FOREIGN_SEED_HASH));
}

#[test]
fn empty_image_with_foreign_seed_hash_is_accepted() {
    let bytes = sketch(0..0).compact(true).serialize();
    assert_eq!(bytes.len(), 16);
    assert_eq!(
        u16::from_le_bytes([bytes[6], bytes[7]]),
        0,
        "5.3.0 serializes an empty sketch with seed hash 0"
    );
    let s = CompactArrayOfDoublesSketch::deserialize(&with_seed_hash(bytes, FOREIGN_SEED_HASH))
        .unwrap();
    assert!(s.is_empty());
}

#[test]
fn genuine_images_round_trip_and_work_in_set_operations() {
    let mut estimation = ArrayOfDoublesSketchBuilder::new().lg_k(10).build().unwrap();
    for k in 0..20_000u64 {
        estimation.update_u64(k, &[1.0]).unwrap();
    }
    assert!(estimation.is_estimation_mode());
    let originals = [
        sketch(0..0).compact(true),
        sketch(0..100).compact(true),
        estimation.compact(true),
        non_empty_zero_entries(),
    ];
    let other = sketch(50..150);
    for original in &originals {
        let restored = CompactArrayOfDoublesSketch::deserialize(&original.serialize()).unwrap();
        assert_eq!(restored.is_empty(), original.is_empty());
        assert_eq!(restored.get_num_retained(), original.get_num_retained());
        assert_eq!(restored.get_estimate(), original.get_estimate());

        let a_not_b = ArrayOfDoublesAnotB::new();
        a_not_b.compute(&restored, &other, true).unwrap();
        a_not_b.compute(&other, &restored, true).unwrap();

        let mut union = ArrayOfDoublesUnionBuilder::new().build().unwrap();
        union.update(&restored).unwrap();
        union.update(&other).unwrap();
        union.get_result(true);

        let mut intersection = ArrayOfDoublesIntersection::new(1).unwrap();
        intersection.update(&restored).unwrap();
        intersection.update(&other).unwrap();
        intersection.get_result(true).unwrap();

        array_of_doubles_jaccard_similarity(&restored, &other).unwrap();
    }
}

// The ArrayOfDoubles counterpart of theta_a_not_b_test.rs's
// `a_not_b_empty_b_with_different_seed_is_ignored`, ported from upstream's
// TEST_CASE("theta a-not-b: empty B with different seed must be ignored") at
// tag 5.3.0. Upstream's fix lives in the shared theta_set_difference_base
// (theta_set_difference_base_impl.hpp:44), so it covers Tuple too. A must be
// non-empty with zero entries: if A had entries, compute() would return early
// at :39 and never reach the B seed-hash check.
#[test]
fn a_not_b_empty_b_with_different_seed_is_ignored() {
    let a = non_empty_zero_entries();

    let b_bytes = with_seed_hash(sketch(0..0).compact(true).serialize(), FOREIGN_SEED_HASH);
    let b = CompactArrayOfDoublesSketch::deserialize(&b_bytes).unwrap();
    assert!(b.is_empty());

    let result = ArrayOfDoublesAnotB::new().compute(&a, &b, true).unwrap();
    assert!(!result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
    assert_eq!(result.get_estimate(), a.get_estimate());
}
