//! The two validations this family adds in Rust because upstream C++ has no
//! equivalent check to delegate to. Both are safety-critical, not merely
//! ergonomic: upstream's update and combine policies index the supplied array
//! blindly for `i in 0..num_values`, so a mismatch would be an out-of-bounds
//! read or write rather than a graceful failure.

use apache_datasketches::tuple::{
    array_of_doubles_jaccard_similarity, ArrayOfDoublesAnotB, ArrayOfDoublesIntersection,
    ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder,
};
use apache_datasketches::SketchError;

fn sketch(num_values: u8) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    let values: Vec<f64> = vec![1.0; num_values as usize];
    for key in 0..10u64 {
        s.update_u64(key, &values).unwrap();
    }
    s
}

#[test]
fn update_with_too_few_values_is_invalid_config() {
    let mut s = ArrayOfDoublesSketchBuilder::new().num_values(3).build().unwrap();
    let err = s.update_u64(1, &[1.0, 2.0]).unwrap_err();
    assert!(matches!(err, SketchError::InvalidConfig(_)));
    // Nothing was added.
    assert!(s.is_empty());
}

#[test]
fn update_with_too_many_values_is_invalid_config() {
    let mut s = ArrayOfDoublesSketchBuilder::new().num_values(3).build().unwrap();
    assert!(matches!(
        s.update_u64(1, &[1.0, 2.0, 3.0, 4.0]).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(s.is_empty());
}

#[test]
fn update_with_empty_slice_is_invalid_config() {
    let mut s = ArrayOfDoublesSketchBuilder::new().num_values(1).build().unwrap();
    assert!(matches!(
        s.update_u64(1, &[]).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    // Nothing was added.
    assert!(s.is_empty());
}

#[test]
fn every_update_key_type_validates_length() {
    let mut s = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    let short: &[f64] = &[1.0];
    assert!(s.update_u64(1, short).is_err());
    assert!(s.update_i64(1, short).is_err());
    assert!(s.update_u32(1, short).is_err());
    assert!(s.update_i32(1, short).is_err());
    assert!(s.update_u16(1, short).is_err());
    assert!(s.update_i16(1, short).is_err());
    assert!(s.update_u8(1, short).is_err());
    assert!(s.update_i8(1, short).is_err());
    assert!(s.update_f64(1.0, short).is_err());
    assert!(s.update_str("k", short).is_err());
    assert!(s.update_bytes(b"k", short).is_err());
    assert!(s.is_empty());

    // And the correct length succeeds for each.
    let ok: &[f64] = &[1.0, 2.0];
    assert!(s.update_u64(1, ok).is_ok());
    assert!(s.update_i64(2, ok).is_ok());
    assert!(s.update_u32(3, ok).is_ok());
    assert!(s.update_i32(4, ok).is_ok());
    assert!(s.update_u16(5, ok).is_ok());
    assert!(s.update_i16(6, ok).is_ok());
    assert!(s.update_u8(7, ok).is_ok());
    assert!(s.update_i8(8, ok).is_ok());
    assert!(s.update_f64(9.0, ok).is_ok());
    assert!(s.update_str("k", ok).is_ok());
    assert!(s.update_bytes(b"k2", ok).is_ok());
    assert!(!s.is_empty());
}

#[test]
fn union_rejects_mismatched_num_values() {
    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    let wrong = sketch(3);
    assert!(matches!(
        u.update(&wrong).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(matches!(
        u.update(&wrong.compact(true)).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    // The matching width still works.
    assert!(u.update(&sketch(2)).is_ok());
}

#[test]
fn intersection_rejects_mismatched_num_values() {
    let mut i = ArrayOfDoublesIntersection::new(2).unwrap();
    let wrong = sketch(1);
    assert!(matches!(
        i.update(&wrong).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(matches!(
        i.update(&wrong.compact(true)).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(i.update(&sketch(2)).is_ok());
}

#[test]
fn a_not_b_rejects_mismatched_num_values() {
    let a = sketch(2);
    let b = sketch(4);
    let calc = ArrayOfDoublesAnotB::new();
    // `.err()` rather than `.unwrap_err()`: `CompactArrayOfDoublesSketch`
    // (the `Ok` type here) does not implement `Debug`, which `unwrap_err`
    // requires; `.err()` discards the `Ok` side without that bound.
    assert!(matches!(
        calc.compute(&a, &b, true).err(),
        Some(SketchError::InvalidConfig(_))
    ));
    assert!(matches!(
        calc.compute(&a.compact(true), &b.compact(true), true).err(),
        Some(SketchError::InvalidConfig(_))
    ));
    assert!(calc.compute(&a, &sketch(2), true).is_ok());
}

#[test]
fn jaccard_rejects_mismatched_num_values() {
    let a = sketch(2);
    let b = sketch(3);
    assert!(matches!(
        array_of_doubles_jaccard_similarity(&a, &b).unwrap_err(),
        SketchError::InvalidConfig(_)
    ));
    assert!(array_of_doubles_jaccard_similarity(&a, &sketch(2)).is_ok());
}

#[test]
fn zero_num_values_is_rejected_everywhere() {
    assert!(ArrayOfDoublesSketchBuilder::new().num_values(0).build().is_err());
    assert!(ArrayOfDoublesUnionBuilder::new().num_values(0).build().is_err());
    assert!(ArrayOfDoublesIntersection::new(0).is_err());
}
