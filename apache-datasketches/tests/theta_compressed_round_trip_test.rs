//! New, non-upstream tests: v4 (compressed) serialize/deserialize
//! round-trips across the three structurally distinct size tiers of the
//! theta compact sketch format (LIST, SET, and large/estimation-mode).
//! This substitutes for theta/test/bit_packing_test.cpp, which is not
//! portable: it tests internal bit-packing helpers with no public-API
//! surface (see the Test Inventory section's "Not ported" note).
use apache_datasketches::theta::{CompactThetaSketch, ThetaSketchBuilder};

fn round_trip_compressed(num_updates: u64) {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..num_updates {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compressed();
    let restored = CompactThetaSketch::deserialize_compressed(&bytes).unwrap();

    assert_eq!(compact.get_estimate(), restored.get_estimate());
    assert_eq!(compact.get_num_retained(), restored.get_num_retained());
    assert_eq!(compact.is_empty(), restored.is_empty());
    assert_eq!(compact.is_estimation_mode(), restored.is_estimation_mode());
    assert_eq!(compact.get_theta(), restored.get_theta());
}

#[test]
fn empty_sketch_compressed_round_trip() {
    round_trip_compressed(0);
}

#[test]
fn list_mode_compressed_round_trip() {
    // Very few entries: upstream's LIST representation (no hash table).
    round_trip_compressed(3);
}

#[test]
fn set_mode_exact_compressed_round_trip() {
    // Enough entries to move from LIST to SET representation, but still
    // exact (no estimation): well under 2^lg_k = 4096 entries.
    round_trip_compressed(100);
}

#[test]
fn set_mode_large_estimation_compressed_round_trip() {
    // Well past 2^lg_k = 4096 entries: forces estimation mode, theta < 1,
    // and a fuller/resized hash table.
    round_trip_compressed(1_000_000);
}

#[test]
fn deserialize_compressed_rejects_v3_uncompressed_only_if_actually_invalid() {
    // deserialize_compressed and deserialize both call the same
    // auto-detecting upstream routine (Design resolution #3), so v3
    // uncompressed bytes ARE accepted by deserialize_compressed too --
    // this documents that behavior rather than asserting it must fail.
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let v3_bytes = compact.serialize_compact();
    let restored = CompactThetaSketch::deserialize_compressed(&v3_bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());
}
