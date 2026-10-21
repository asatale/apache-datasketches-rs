//! Wire-format compatibility. The C++ and Java libraries share the KLL
//! format, so bytes that match upstream C++'s layout are readable by Java and
//! Python too.

use apache_datasketches::kll::KllSketch;
use apache_datasketches::SketchError;

const FIXTURE_FLOAT_ONE_ITEM_V1: &[u8] = include_bytes!("data/kll_sketch_float_one_item_v1.sk");

/// preamble_ints=2, serial_version=1, family=15, flags=EMPTY, k=200 (LE),
/// m=8, unused.
const EMPTY_K200: [u8; 8] = [2, 1, 15, 1, 200, 0, 8, 0];

// Port of upstream SECTION("deserialize one item v1").
#[test]
fn upstream_fixture_float_one_item_v1() {
    let sketch = KllSketch::<f32>::deserialize(FIXTURE_FLOAT_ONE_ITEM_V1).unwrap();
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 1);
    assert_eq!(sketch.get_num_retained(), 1);
    assert_eq!(sketch.get_min_item(), Some(1.0));
    assert_eq!(sketch.get_max_item(), Some(1.0));
}

#[test]
fn golden_empty() {
    assert_eq!(KllSketch::<f64>::default().serialize(), EMPTY_K200);
    assert_eq!(KllSketch::<f32>::default().serialize(), EMPTY_K200);
}

#[test]
fn golden_single_item() {
    // serial_version=2, flags=SINGLE_ITEM, then just the item.
    let header = [2u8, 2, 15, 4, 200, 0, 8, 0];

    let mut d = KllSketch::<f64>::default();
    d.update(1.0);
    let expected: Vec<u8> = header.iter().copied().chain(1.0f64.to_le_bytes()).collect();
    assert_eq!(d.serialize(), expected);

    let mut f = KllSketch::<f32>::default();
    f.update(1.0);
    let expected: Vec<u8> = header.iter().copied().chain(1.0f32.to_le_bytes()).collect();
    assert_eq!(f.serialize(), expected);
}

// The empty preamble carries no item type, so this is correct, not a leak.
#[test]
fn empty_bytes_deserialize_as_either_type() {
    assert!(KllSketch::<f64>::deserialize(&EMPTY_K200)
        .unwrap()
        .is_empty());
    assert!(KllSketch::<f32>::deserialize(&EMPTY_K200)
        .unwrap()
        .is_empty());
}

fn f32_bytes(n: u32) -> Vec<u8> {
    let mut s = KllSketch::<f32>::default();
    for i in 0..n {
        s.update(i as f32);
    }
    s.serialize()
}

fn f64_bytes(n: u32) -> Vec<u8> {
    let mut s = KllSketch::<f64>::default();
    for i in 0..n {
        s.update(f64::from(i));
    }
    s.serialize()
}

#[test]
fn non_empty_bytes_of_the_other_type_are_rejected() {
    // 1 = single item, 3 = n <= k, 10_000 = estimation mode.
    for n in [1, 3, 10_000] {
        assert!(
            matches!(
                KllSketch::<f64>::deserialize(&f32_bytes(n)),
                Err(SketchError::Deserialization(_))
            ),
            "f32 bytes with n={n} accepted as f64"
        );
        assert!(
            matches!(
                KllSketch::<f32>::deserialize(&f64_bytes(n)),
                Err(SketchError::Deserialization(_))
            ),
            "f64 bytes with n={n} accepted as f32"
        );
    }
    assert!(matches!(
        KllSketch::<f64>::deserialize(FIXTURE_FLOAT_ONE_ITEM_V1),
        Err(SketchError::Deserialization(_))
    ));
}

#[test]
fn deterministic_sketch_bytes_are_stable() {
    // n <= k never compacts, so the bytes cannot depend on the random bit.
    assert_eq!(f64_bytes(150), f64_bytes(150));
    assert_eq!(f32_bytes(150), f32_bytes(150));
}
