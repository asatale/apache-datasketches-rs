use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, DEFAULT_SEED};
use apache_datasketches::SketchError;

fn filled(n: u64) -> BloomFilter {
    let mut f = BloomFilterBuilder::by_accuracy(10_000, 0.01)
        .build()
        .unwrap();
    for i in 0..n {
        f.update_u64(i);
    }
    f
}

#[test]
fn default_seed_and_override() {
    assert_eq!(
        BloomFilterBuilder::by_size(1024, 3).build().unwrap().seed(),
        DEFAULT_SEED
    );
    assert_eq!(
        BloomFilterBuilder::by_size(1024, 3)
            .seed(7)
            .build()
            .unwrap()
            .seed(),
        7
    );
}

#[test]
fn no_false_negatives() {
    let f = filled(5_000);
    for i in 0..5_000u64 {
        assert!(f.query_u64(i), "false negative for {i}");
    }
}

#[test]
fn every_overload_family_round_trips() {
    let mut f = BloomFilterBuilder::by_accuracy(1_000, 0.01)
        .build()
        .unwrap();
    f.update_u64(1);
    f.update_i64(-2);
    f.update_u32(3);
    f.update_i32(-4);
    f.update_u16(5);
    f.update_i16(-6);
    f.update_u8(7);
    f.update_i8(-8);
    f.update_f64(9.5);
    f.update_f32(10.5);
    f.update_str("eleven");
    f.update_bytes(b"twelve");
    assert!(f.query_u64(1) && f.query_i64(-2) && f.query_u32(3) && f.query_i32(-4));
    assert!(f.query_u16(5) && f.query_i16(-6) && f.query_u8(7) && f.query_i8(-8));
    assert!(
        f.query_f64(9.5) && f.query_f32(10.5) && f.query_str("eleven") && f.query_bytes(b"twelve")
    );
}

#[test]
fn query_and_update_reports_prior_membership() {
    let mut f = BloomFilterBuilder::by_accuracy(1_000, 0.01)
        .build()
        .unwrap();
    assert!(!f.query_and_update_u64(42));
    assert!(f.query_and_update_u64(42));
}

#[test]
fn empty_string_is_ignored() {
    let mut f = BloomFilterBuilder::by_size(1024, 3).build().unwrap();
    f.update_str("");
    assert!(f.is_empty());
    assert!(!f.query_str(""));
}

#[test]
fn union_intersect_invert_reset() {
    let mut a = BloomFilterBuilder::by_size(4096, 4).build().unwrap();
    let mut b = BloomFilterBuilder::by_size(4096, 4).build().unwrap();
    a.update_u64(1);
    b.update_u64(2);
    let mut u = a.clone();
    u.union_with(&b).unwrap();
    assert!(u.query_u64(1) && u.query_u64(2));
    let mut i = u.clone();
    i.intersect(&a).unwrap();
    assert!(i.query_u64(1));
    let mut inv = a.clone();
    let before = inv.bits_used();
    inv.invert();
    assert_eq!(inv.bits_used(), inv.capacity() - before);
    inv.reset();
    assert!(inv.is_empty());
}

#[test]
fn incompatible_filters_are_rejected() {
    let mut a = BloomFilterBuilder::by_size(4096, 4).build().unwrap();
    for other in [
        BloomFilterBuilder::by_size(4096, 4)
            .seed(1)
            .build()
            .unwrap(),
        BloomFilterBuilder::by_size(8192, 4).build().unwrap(),
        BloomFilterBuilder::by_size(4096, 5).build().unwrap(),
    ] {
        assert!(!a.is_compatible(&other));
        assert!(matches!(
            a.union_with(&other),
            Err(SketchError::InvalidConfig(_))
        ));
        assert!(matches!(
            a.intersect(&other),
            Err(SketchError::InvalidConfig(_))
        ));
    }
}

#[test]
fn invalid_builder_arguments() {
    for (n, p) in [
        (0u64, 0.01),
        (100, 0.0),
        (100, 1.0),
        (100, f64::NAN),
        (100, -0.1),
    ] {
        assert!(
            matches!(
                BloomFilterBuilder::by_accuracy(n, p).build(),
                Err(SketchError::InvalidConfig(_))
            ),
            "{n} {p}"
        );
    }
    assert!(matches!(
        BloomFilterBuilder::by_size(0, 3).build(),
        Err(SketchError::InvalidConfig(_))
    ));
    assert!(matches!(
        BloomFilterBuilder::by_size(64, 0).build(),
        Err(SketchError::InvalidConfig(_))
    ));
    assert!(matches!(
        BloomFilterBuilder::by_size(u64::MAX, 3).build(),
        Err(SketchError::InvalidConfig(_))
    ));
}

#[test]
fn serialize_roundtrip_and_capacity_rounding() {
    let mut f = filled(1_000);
    let restored = BloomFilter::deserialize(&f.serialize()).unwrap();
    assert_eq!(restored.capacity(), f.capacity());
    assert_eq!(restored.num_hashes(), f.num_hashes());
    assert_eq!(restored.seed(), f.seed());
    let mut restored = restored;
    assert_eq!(restored.bits_used(), f.bits_used());
    assert_eq!(
        BloomFilterBuilder::by_size(65, 3)
            .build()
            .unwrap()
            .capacity(),
        128
    );
    assert!(matches!(
        BloomFilter::deserialize(&[1, 2, 3]),
        Err(SketchError::Deserialization(_))
    ));
}

#[test]
fn suggestions() {
    assert!(BloomFilterBuilder::suggest_num_hashes(1_000, 9_586).unwrap() >= 1);
    assert!(BloomFilterBuilder::suggest_num_hashes_for_fpp(0.01).unwrap() >= 1);
    assert!(BloomFilterBuilder::suggest_num_filter_bits(1_000, 0.01).unwrap() > 1_000);
    assert!(matches!(
        BloomFilterBuilder::suggest_num_filter_bits(0, 0.01),
        Err(SketchError::InvalidConfig(_))
    ));
}

#[test]
fn false_positive_rate_is_near_target() {
    // Deterministic given the seed: 10k members, 100k non-members.
    let f = filled(10_000);
    let fp = (1_000_000u64..1_100_000)
        .filter(|&i| f.query_u64(i))
        .count();
    let rate = fp as f64 / 100_000.0;
    assert!(rate < 0.02, "fpp {rate} far above the 0.01 target");
}
