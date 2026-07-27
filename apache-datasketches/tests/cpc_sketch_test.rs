// apache-datasketches/tests/cpc_sketch_test.rs
//! Ported from cpc/test/cpc_sketch_test.cpp (tag 5.2.0). 15 of 26 upstream
//! cases ported; 11 excluded:
//! - "overflow bug" (100,000,000 updates) — impractically slow for a
//!   routine local test suite; the bug it regression-tests is a specific
//!   historical fix, not a public-API behavior distinct from the
//!   already-ported "many values"/large-scale tests.
//! - "serialize deserialize {empty,sparse,hybrid,pinned,sliding}" (the
//!   ostream-based variants) — duplicates of the byte-vector-based
//!   "..., bytes" variants below, which this crate ports instead since its
//!   public API only exposes the byte-vector serialize()/deserialize()
//!   pair (no ostream overload).
//! - "serializing deserialize sliding large" (ostream-only, n=3,000,000) —
//!   redundant with the "sliding" tier already covered at a smaller,
//!   faster n; the "sliding huge" case below covers the large-scale path.
//! - "copy" — tests C++ copy-constructor/assignment semantics; this
//!   crate's `CpcSketch` doesn't implement `Clone`.
//! - "serialize deserialize empty, custom seed" — no seed parameter is
//!   exposed in this crate's public API (every sketch uses upstream's
//!   `DEFAULT_SEED`).
//! - "validate fail" — `validate()` is not exposed (marked `@private`
//!   upstream, for internal debugging use only).
//! - "serialize both ways" — exercises the `header_size_bytes` parameter
//!   on `serialize()`, which this crate doesn't expose (PostgreSQL
//!   extension-specific, no use case here).
use apache_datasketches::cpc::{get_max_serialized_size_bytes, CpcSketch, CpcSketchBuilder};

const RELATIVE_ERROR_FOR_LG_K_11: f64 = 0.02;

#[test]
fn lg_k_limits() {
    assert!(CpcSketchBuilder::new().lg_k(4).build().is_ok());
    assert!(CpcSketchBuilder::new().lg_k(26).build().is_ok());
    assert!(CpcSketchBuilder::new().lg_k(3).build().is_err());
    assert!(CpcSketchBuilder::new().lg_k(27).build().is_err());
}

#[test]
fn empty() {
    let sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_estimate(), 0.0);
    assert_eq!(sketch.get_lower_bound(1).unwrap(), 0.0);
    assert_eq!(sketch.get_upper_bound(1).unwrap(), 0.0);
}

#[test]
fn one_value() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_u64(1);
    assert!(!sketch.is_empty());
    let estimate = sketch.get_estimate();
    assert!((estimate - 1.0).abs() < RELATIVE_ERROR_FOR_LG_K_11);
    assert!(estimate >= sketch.get_lower_bound(1).unwrap());
    assert!(estimate <= sketch.get_upper_bound(1).unwrap());
}

#[test]
fn many_values() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 10_000u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    assert!(!sketch.is_empty());
    let estimate = sketch.get_estimate();
    assert!((estimate - n as f64).abs() < n as f64 * RELATIVE_ERROR_FOR_LG_K_11);
    assert!(estimate >= sketch.get_lower_bound(1).unwrap());
    assert!(estimate <= sketch.get_upper_bound(1).unwrap());
}

fn round_trip(sketch: &CpcSketch) -> CpcSketch {
    let bytes = sketch.serialize();
    CpcSketch::deserialize(&bytes).unwrap()
}

#[test]
fn serialize_deserialize_empty() {
    let sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let bytes = sketch.serialize();
    let deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());
}

#[test]
fn serialize_deserialize_sparse() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 100u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    // updating again with the same values should not change the sketch
    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn serialize_deserialize_hybrid() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 200u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn serialize_deserialize_pinned() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 2_000u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn serialize_deserialize_sliding() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    let n = 20_000u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn serialize_deserialize_sliding_huge() {
    let mut sketch = CpcSketchBuilder::new().lg_k(26).build().unwrap();
    let n = 10_000_000u64;
    for i in 0..n {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - n as f64).abs() < n as f64 * 0.001);
    let bytes = sketch.serialize();
    let mut deserialized = round_trip(&sketch);
    assert_eq!(deserialized.is_empty(), sketch.is_empty());
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
    assert!(CpcSketch::deserialize(&bytes[..7]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..15]).is_err());
    assert!(CpcSketch::deserialize(&bytes[..bytes.len() - 1]).is_err());

    for i in 0..n {
        deserialized.update_u64(i);
    }
    assert_eq!(deserialized.get_estimate(), sketch.get_estimate());
}

#[test]
fn kappa_range() {
    let sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    assert_eq!(sketch.get_lower_bound(1).unwrap(), 0.0);
    assert_eq!(sketch.get_upper_bound(1).unwrap(), 0.0);
    assert_eq!(sketch.get_lower_bound(2).unwrap(), 0.0);
    assert_eq!(sketch.get_upper_bound(2).unwrap(), 0.0);
    assert_eq!(sketch.get_lower_bound(3).unwrap(), 0.0);
    assert_eq!(sketch.get_upper_bound(3).unwrap(), 0.0);
    assert!(sketch.get_lower_bound(4).is_err());
    assert!(sketch.get_upper_bound(4).is_err());
}

#[test]
fn update_int_equivalence() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_u64(u64::MAX);
    sketch.update_i64(-1);
    sketch.update_u32(u32::MAX);
    sketch.update_i32(-1);
    sketch.update_u16(u16::MAX);
    sketch.update_i16(-1);
    sketch.update_u8(u8::MAX);
    sketch.update_i8(-1);
    let estimate = sketch.get_estimate();
    assert!((estimate - 1.0).abs() < RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn update_float_equivalence() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_f32(1.0);
    sketch.update_f64(1.0);
    let estimate = sketch.get_estimate();
    assert!((estimate - 1.0).abs() < RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn update_string_equivalence() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    sketch.update_str("a");
    sketch.update_bytes(b"a");
    let estimate = sketch.get_estimate();
    assert!((estimate - 1.0).abs() < RELATIVE_ERROR_FOR_LG_K_11);
}

#[test]
fn max_serialized_size() {
    assert_eq!(get_max_serialized_size_bytes(4), 24 + 40);
    assert_eq!(
        get_max_serialized_size_bytes(26),
        ((0.6 * (1u64 << 26) as f64) as usize) + 40
    );
}
