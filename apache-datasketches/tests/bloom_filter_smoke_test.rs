use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, DEFAULT_SEED};
use apache_datasketches::SketchError;

mod bloom_common;
use bloom_common::{assert_rejected, Image, DIRTY, EMPTY_FLAG};

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

// ---- Hardening: crafted serialized images -------------------------------
//
// Upstream's `internal_deserialize_or_wrap` (bloom_filter_impl.hpp:346-421,
// 5.3.0) trusts parts of the header; the shim's `validate_bloom_layout`
// rejects these before upstream sees them. Each case is checked through both
// `BloomFilter::deserialize` and `WrappedBloomFilter::wrap`.

#[test]
fn rejects_zero_hashes() {
    // Upstream checks num_hashes only on the empty path (in the constructor);
    // a non-empty image with 0 hashes made every query return true.
    assert_rejected(
        &Image::non_empty(2, 0, 7, &[3]).bytes(),
        "non-empty, 0 hashes",
    );
    assert_rejected(&Image::empty(2, 0, 7).bytes(), "empty, 0 hashes");
}

#[test]
fn rejects_max_hashes() {
    // Upstream's hashing loops use a uint16_t counter `i <= num_hashes_`
    // (bloom_filter_impl.hpp:650, :748, :845), which never ends for 65535:
    // any update hangs.
    assert_rejected(
        &Image::non_empty(2, u16::MAX, 7, &[3]).bytes(),
        "non-empty, 65535 hashes",
    );
    assert_rejected(&Image::empty(2, u16::MAX, 7).bytes(), "empty, 65535 hashes");
    assert!(matches!(
        BloomFilterBuilder::by_size(64, u16::MAX).build(),
        Err(SketchError::InvalidConfig(_))
    ));
    // One fewer works.
    let mut f = BloomFilterBuilder::by_size(64, u16::MAX - 1)
        .build()
        .unwrap();
    f.update_u64(1);
    assert!(f.query_u64(1));
    assert!(f.query_and_update_u64(1));
}

#[test]
fn rejects_zero_capacity() {
    // Non-empty with num_longs = 0: upstream builds a filter of capacity 0,
    // and update / query_and_update compute `hash % 0`.
    assert_rejected(
        &Image::non_empty(0, 3, 7, &[]).bytes(),
        "non-empty, 0 longs",
    );
    assert_rejected(&Image::empty(0, 3, 7).bytes(), "empty, 0 longs");
}

#[test]
fn rejects_num_longs_that_wrap_the_32_bit_shift() {
    // Upstream reads num_longs as uint32_t and evaluates `num_longs << 6` and
    // `num_longs << 3` in 32 bits (bloom_filter_impl.hpp:392, :398, :420).
    // These need no large buffer: num_bytes wraps to 0 or 8 too.
    for num_longs in [1u32 << 29, 0x8000_0000, 3 << 29] {
        // Capacity wraps to 0: the same `% 0` as num_longs = 0. 0x8000_0000
        // is also a negative int32_t, which is what upstream serializes.
        let mut image = Image::non_empty(1, 3, 7, &[]);
        image.num_longs = num_longs;
        image.bits.clear();
        assert_rejected(&image.bytes(), &format!("non-empty, {num_longs:#x} longs"));
    }
    // 2^29 + 1 reads as one word: capacity 64, misread rather than rejected.
    let mut image = Image::non_empty(1, 3, 7, &[5]);
    image.num_longs = (1 << 29) + 1;
    assert_rejected(&image.bytes(), "non-empty, 2^29 + 1 longs");
    // Empty images: 2^26 wraps to 0 bits, 2^26 + 1 to a 64-bit filter, and
    // the rest are far above the 2^26 - 1 word maximum too.
    for num_longs in [1u32 << 26, (1 << 26) + 1, 1 << 28, 0x8000_0000, u32::MAX] {
        assert_rejected(
            &Image::empty(num_longs, 3, 7).bytes(),
            &format!("empty, {num_longs:#x} longs"),
        );
    }
}

#[test]
fn rejects_num_longs_at_the_maximum_boundary() {
    // The largest readable capacity is 2^26 - 1 words (2^32 - 64 bits). One
    // word more is rejected outright, even with a matching (empty) image.
    let mut image = Image::non_empty(1, 3, 7, &[]);
    image.num_longs = 1 << 26;
    image.bits.clear();
    assert_rejected(&image.bytes(), "non-empty, 2^26 longs");
    assert_rejected(&Image::empty(1 << 26, 3, 7).bytes(), "empty, 2^26 longs");
    // 2^26 - 1 words is in range, but a non-empty image needs 512 MiB of bit
    // array; a truncated buffer is rejected by the length check before
    // anything is allocated. (A genuine empty image of that size would be
    // accepted and allocate 512 MiB, so it is not exercised here.)
    let mut image = Image::non_empty(1, 3, 7, &[]);
    image.num_longs = (1 << 26) - 1;
    assert_rejected(&image.bytes(), "non-empty, 2^26 - 1 longs, truncated");
}

#[test]
fn rejects_truncated_bit_array() {
    let bytes = Image::non_empty(4, 3, 7, &[1, 100, 200]).bytes();
    for len in [24, 31, 32, 33, 40, bytes.len() - 1] {
        assert_rejected(&bytes[..len], &format!("truncated to {len}"));
    }
}

#[test]
fn rejects_wrong_bits_set_count() {
    let mut image = Image::non_empty(2, 3, 7, &[1, 2, 3]);
    for count in [0, 2, 4, 128, DIRTY - 1] {
        image.count = Some(count);
        assert_rejected(&image.bytes(), &format!("count {count}"));
    }
}

#[test]
fn rejects_unknown_flag_bits() {
    for flag in [1u8, 2, 8, 0x10, 0x80] {
        let mut image = Image::non_empty(2, 3, 7, &[1]);
        image.flags = flag;
        assert_rejected(&image.bytes(), &format!("non-empty, flags {flag:#x}"));
        let mut image = Image::empty(2, 3, 7);
        image.flags = EMPTY_FLAG | flag;
        assert_rejected(&image.bytes(), &format!("empty, flags {flag:#x}"));
    }
}

#[test]
fn rejects_preamble_flag_mismatch() {
    // Preamble 3 without the empty flag: upstream checks only 24 bytes are
    // present, then reads the bits-set count and bit array past them.
    let mut image = Image::non_empty(2, 3, 7, &[1]);
    image.preamble_longs = 3;
    assert_rejected(&image.bytes(), "preamble 3, non-empty");
    // Preamble 4 with the empty flag (upstream never writes it).
    let mut image = Image::empty(2, 3, 7);
    image.preamble_longs = 4;
    image.count = Some(0);
    assert_rejected(&image.bytes(), "preamble 4, empty");
    // Out-of-range preambles, version and family.
    for preamble_longs in [0u8, 1, 2, 5, 255] {
        let mut image = Image::non_empty(2, 3, 7, &[1]);
        image.preamble_longs = preamble_longs;
        assert_rejected(&image.bytes(), &format!("preamble {preamble_longs}"));
    }
    let mut image = Image::non_empty(2, 3, 7, &[1]);
    image.serial_version = 2;
    assert_rejected(&image.bytes(), "serial version 2");
    let mut image = Image::non_empty(2, 3, 7, &[1]);
    image.family = 20;
    assert_rejected(&image.bytes(), "family 20");
}

#[test]
fn accepts_genuine_crafted_images() {
    // An empty image with a large but valid capacity still deserializes and
    // allocates the whole bit array (2^20 bits here; see
    // `BloomFilter::deserialize` for the bound).
    let bytes = Image::empty(1 << 14, 3, 7).bytes();
    let mut f = BloomFilter::deserialize(&bytes).unwrap();
    assert!(f.is_empty());
    assert_eq!(f.capacity(), 1 << 20);
    f.update_u64(9);
    assert!(f.query_u64(9));
    let w = apache_datasketches::bloom::WrappedBloomFilter::wrap(&bytes).unwrap();
    assert_eq!(w.capacity(), 1 << 20);

    // Non-empty with the dirty sentinel, any seed, trailing bytes.
    for seed in [0, 7, u64::MAX] {
        let mut image = Image::non_empty(3, 65_534, seed, &[0, 64, 191]);
        image.count = Some(DIRTY);
        let mut bytes = image.bytes();
        bytes.extend_from_slice(&[0xAB; 5]);
        let mut f = BloomFilter::deserialize(&bytes).unwrap();
        assert_eq!(
            (f.capacity(), f.num_hashes(), f.seed(), f.bits_used()),
            (192, 65_534, seed, 3)
        );
        let w = apache_datasketches::bloom::WrappedBloomFilter::wrap(&bytes).unwrap();
        assert_eq!((w.capacity(), w.bits_used()), (192, 3));
    }

    // The largest accepted capacity, 2^26 - 1 words, as a non-empty image is
    // 512 MiB; check only the boundary arithmetic on a one-word image.
    let f = BloomFilter::deserialize(&Image::non_empty(1, 1, 0, &[63]).bytes()).unwrap();
    assert_eq!(f.capacity(), 64);
}

#[test]
fn deserialized_filters_update_and_query_after_validation() {
    // The operations that divide by the capacity, on accepted crafted bytes.
    let mut f = BloomFilter::deserialize(&Image::non_empty(1, 4, 7, &[0]).bytes()).unwrap();
    for i in 0..100u64 {
        f.update_u64(i);
        assert!(f.query_and_update_u64(i));
        assert!(f.query_u64(i));
    }
    let again = BloomFilter::deserialize(&f.serialize()).unwrap();
    assert_eq!(again.capacity(), 64);
}

#[test]
fn mixing_update_and_query_and_update_keeps_count_exact() {
    // Upstream's query_and_update added to a stale cached count after a plain
    // update, so bits_used under-reported and serialize wrote a count that
    // deserialize rejected. Deserialize recounts the bits and compares, so a
    // successful round trip pins the count.
    // The stale count could also be 0 with bits set, so the filter claimed to
    // be empty and every query answered false: a false negative.
    let mut f = BloomFilterBuilder::by_size(1024, 3).build().unwrap();
    f.update_u64(1);
    assert!(f.query_and_update_u64(1));
    assert!(!f.is_empty());
    assert!(f.query_u64(1), "no false negatives");
    assert_eq!(f.bits_used(), 3);

    let mut f = BloomFilterBuilder::by_size(1024, 3).build().unwrap();
    for i in 0..200u64 {
        if i % 3 == 0 {
            f.update_u64(i);
        } else {
            let _ = f.query_and_update_u64(i);
        }
        if i % 50 == 0 {
            let mut restored = BloomFilter::deserialize(&f.serialize()).unwrap();
            assert_eq!(restored.bits_used(), f.bits_used(), "after {i}");
        }
    }
    f.update_str("a");
    let _ = f.query_and_update_bytes(b"b");
    let mut restored = BloomFilter::deserialize(&f.serialize()).unwrap();
    assert_eq!(restored.bits_used(), f.bits_used());
}

// ---- suggest_* range guards ------------------------------------------------
//
// Upstream static_casts the suggestion (a double) to uint16_t / uint64_t
// unchecked (bloom_filter_builder_impl.hpp:48, :54, :61): out of range is
// undefined behavior, so the safe layer rejects it first.

/// 2^32 - 64 bits: the largest capacity upstream can read back, (2^26 - 1)
/// words. The builder caps sizes here so every filter round-trips.
const MAX_FILTER_SIZE_BITS: u64 = (1 << 32) - 64;

fn assert_size_rejected<T>(result: Result<T, SketchError>, what: &str) {
    match result {
        Err(SketchError::InvalidConfig(msg)) => assert!(
            msg.contains("2^32"),
            "{what}: message should give the reason: {msg:?}"
        ),
        Err(e) => panic!("{what}: expected InvalidConfig, got {e:?}"),
        Ok(_) => panic!("{what}: expected InvalidConfig, got Ok"),
    }
}

#[test]
fn filter_size_is_capped_so_every_filter_round_trips() {
    // by_size rounds up to a multiple of 64: anything above the cap (which
    // is a multiple of 64) rounds above it. Building exactly the cap would
    // allocate 512 MiB, so the accepted boundary is checked via
    // suggest_num_hashes, which applies the same range check to its bits.
    for bits in [
        MAX_FILTER_SIZE_BITS + 1,
        MAX_FILTER_SIZE_BITS + 63,
        1 << 32,
        1 << 33,
    ] {
        assert_size_rejected(BloomFilterBuilder::by_size(bits, 3).build(), "by_size");
        assert_size_rejected(
            BloomFilterBuilder::suggest_num_hashes(1_000_000_000, bits),
            "suggest_num_hashes",
        );
    }
    assert_eq!(
        BloomFilterBuilder::suggest_num_hashes(1_000_000_000, MAX_FILTER_SIZE_BITS).unwrap(),
        3
    );
    // by_accuracy: 500M items at 1% need about 4.8e9 bits.
    assert_size_rejected(
        BloomFilterBuilder::by_accuracy(500_000_000, 0.01).build(),
        "by_accuracy",
    );
    assert_size_rejected(
        BloomFilterBuilder::suggest_num_filter_bits(500_000_000, 0.01),
        "suggest_num_filter_bits",
    );
    // Rounding up to a multiple of 64 is what the cap check accounts for.
    let f = BloomFilterBuilder::by_size(64 * 1024 - 63, 2)
        .build()
        .unwrap();
    assert_eq!(f.capacity(), 64 * 1024);
    assert!(BloomFilter::deserialize(&f.serialize()).is_ok());
}

#[test]
fn suggest_num_hashes_rejects_results_above_the_maximum() {
    // With one item, ceil(bits * ln 2): 94_545 bits -> 65_534, the most a
    // filter can use; 94_546 -> 65_535 (a hang upstream, see
    // `rejects_max_hashes`), 94_548 -> 65_536 (UB in upstream's cast).
    assert_eq!(
        BloomFilterBuilder::suggest_num_hashes(1, 94_545).unwrap(),
        65_534
    );
    for (n, bits) in [
        (1u64, 94_546u64),
        (1, 94_548),
        (1, MAX_FILTER_SIZE_BITS),
        (1_000, 1 << 34),
    ] {
        assert!(
            matches!(
                BloomFilterBuilder::suggest_num_hashes(n, bits),
                Err(SketchError::InvalidConfig(_))
            ),
            "{n} {bits}"
        );
    }
    assert_eq!(
        BloomFilterBuilder::suggest_num_hashes(u64::MAX, 1).unwrap(),
        1
    );
    assert!(matches!(
        BloomFilterBuilder::suggest_num_hashes(1, 0),
        Err(SketchError::InvalidConfig(_))
    ));
}

#[test]
fn suggest_num_filter_bits_rejects_results_above_the_maximum_size() {
    // At fpp 0.5, ceil(n / ln 2): 2_977_044_427 items need exactly the
    // maximum filter size, 2_977_044_428 one bit more. (Only suggested, not
    // built: building the maximum allocates 512 MiB.)
    assert_eq!(
        BloomFilterBuilder::suggest_num_filter_bits(2_977_044_427, 0.5).unwrap(),
        MAX_FILTER_SIZE_BITS
    );
    for (n, p) in [
        (2_977_044_428u64, 0.5),
        (u64::MAX, 0.01),
        (u64::MAX, 0.999),
        (1 << 40, f64::MIN_POSITIVE),
    ] {
        assert!(
            matches!(
                BloomFilterBuilder::suggest_num_filter_bits(n, p),
                Err(SketchError::InvalidConfig(_))
            ),
            "{n} {p}"
        );
        // build() calls the same upstream suggestion internally.
        assert!(
            matches!(
                BloomFilterBuilder::by_accuracy(n, p).build(),
                Err(SketchError::InvalidConfig(_))
            ),
            "build {n} {p}"
        );
    }
}

#[test]
fn suggest_num_hashes_for_fpp_always_fits() {
    // ceil(-ln p / ln 2) peaks at about 1074 for the smallest positive f64.
    let most = BloomFilterBuilder::suggest_num_hashes_for_fpp(f64::from_bits(1)).unwrap();
    assert!((1074..=1075).contains(&most), "{most}");
    assert_eq!(
        BloomFilterBuilder::suggest_num_hashes_for_fpp(0.999_999).unwrap(),
        1
    );
    assert_eq!(
        BloomFilterBuilder::suggest_num_hashes_for_fpp(0.5).unwrap(),
        1
    );
}

#[test]
fn incompatibility_errors_name_the_parameter() {
    let mut a = BloomFilterBuilder::by_size(4096, 4).build().unwrap();
    for (other, word) in [
        (
            BloomFilterBuilder::by_size(8192, 4).build().unwrap(),
            "capacity",
        ),
        (
            BloomFilterBuilder::by_size(4096, 5).build().unwrap(),
            "number of hashes",
        ),
        (
            BloomFilterBuilder::by_size(4096, 4)
                .seed(1)
                .build()
                .unwrap(),
            "seed",
        ),
    ] {
        for result in [a.union_with(&other), a.intersect(&other)] {
            match result {
                Err(SketchError::InvalidConfig(msg)) => {
                    assert!(msg.contains(word), "{msg:?} should name the {word}")
                }
                other => panic!("expected InvalidConfig, got {other:?}"),
            }
        }
    }
}
