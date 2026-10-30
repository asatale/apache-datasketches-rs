//! Wire-format tests for the Bloom filter. The expected bytes are written out
//! by hand from upstream's `bloom_filter_alloc::serialize`
//! (`filters/include/bloom_filter_impl.hpp`, datasketches-cpp 5.3.0), not
//! captured from the binding. Little-endian layout:
//!
//! | bytes | field |
//! |---|---|
//! | 0 | preamble longs: 3 when empty, 4 otherwise |
//! | 1 | serial version, 1 |
//! | 2 | family id, 21 |
//! | 3 | flags: 4 (empty) or 0 |
//! | 4-5 | `num_hashes` (u16) |
//! | 6-7 | unused, 0 |
//! | 8-15 | seed (u64) |
//! | 16-19 | `num_longs` = capacity / 64 (i32) |
//! | 20-23 | unused, 0 |
//! | 24-31 | bits set (u64), non-empty only |
//! | 32.. | bit array, `num_longs * 8` bytes, non-empty only |
//!
//! Upstream writes `is_dirty_ ? u64::MAX : num_bits_set_` in the bits-set
//! field. The binding routes every `update_*` through upstream's
//! `query_and_update` (which keeps the count exact, never dirty), and
//! `invert`, `union_with`, `intersect` and `reset` also store an exact count,
//! so a serialized image from this binding always carries the true count.

use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};

mod bloom_common;
use bloom_common::DIRTY;

/// 9001 (the default seed) little-endian.
const SEED_9001: [u8; 8] = [0x29, 0x23, 0, 0, 0, 0, 0, 0];
/// 12345 little-endian.
const SEED_12345: [u8; 8] = [0x39, 0x30, 0, 0, 0, 0, 0, 0];

fn empty_golden(num_hashes: u8, seed: [u8; 8], num_longs: u8) -> Vec<u8> {
    let mut v = vec![3, 1, 21, 4, num_hashes, 0, 0, 0];
    v.extend_from_slice(&seed);
    v.extend_from_slice(&[num_longs, 0, 0, 0, 0, 0, 0, 0]);
    v
}

/// A non-empty golden: 2 longs, 3 hashes, bits 0, 70 and 127 set (3 bits).
fn non_empty_golden(seed: [u8; 8], count: [u8; 8]) -> Vec<u8> {
    let mut v = vec![4, 1, 21, 0, 3, 0, 0, 0];
    v.extend_from_slice(&seed);
    v.extend_from_slice(&[2, 0, 0, 0, 0, 0, 0, 0]);
    v.extend_from_slice(&count);
    let mut bits = [0u8; 16];
    bits[0] = 0x01; // bit 0
    bits[8] = 0x40; // bit 70 = 64 + 6
    bits[15] = 0x80; // bit 127 = 120 + 7
    v.extend_from_slice(&bits);
    v
}

#[test]
fn empty_filter_golden_bytes() {
    // by_size(128, 3): num_longs = 128 / 64 = 2, 24 bytes.
    let f = BloomFilterBuilder::by_size(128, 3).build().unwrap();
    let expected = [
        3, 1, 21, 4, 3, 0, 0, 0, 0x29, 0x23, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
    ];
    assert_eq!(f.serialize(), expected);
    assert_eq!(f.serialize(), empty_golden(3, SEED_9001, 2));

    let f = BloomFilterBuilder::by_size(128, 3)
        .seed(12345)
        .build()
        .unwrap();
    assert_eq!(f.serialize(), empty_golden(3, SEED_12345, 2));

    // Rounding up: 129 bits is 3 longs; num_hashes is two bytes wide.
    let f = BloomFilterBuilder::by_size(129, 258).build().unwrap();
    let mut expected = vec![3, 1, 21, 4, 2, 1, 0, 0];
    expected.extend_from_slice(&SEED_9001);
    expected.extend_from_slice(&[3, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(f.serialize(), expected);
}

#[test]
fn reset_filter_serializes_as_empty() {
    let mut f = BloomFilterBuilder::by_size(128, 3).build().unwrap();
    f.update_u64(1);
    assert_eq!(f.serialize().len(), 32 + 16);
    f.reset();
    assert_eq!(f.serialize(), empty_golden(3, SEED_9001, 2));
}

#[test]
fn golden_non_empty_image_round_trips_byte_for_byte() {
    for seed in [SEED_9001, SEED_12345] {
        let golden = non_empty_golden(seed, [3, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(golden.len(), 32 + 2 * 8);

        let mut f = BloomFilter::deserialize(&golden).unwrap();
        assert!(!f.is_empty());
        assert_eq!(f.capacity(), 128);
        assert_eq!(f.num_hashes(), 3);
        assert_eq!(f.seed(), u64::from_le_bytes(seed));
        assert_eq!(f.bits_used(), 3);
        assert_eq!(f.serialize(), golden);

        let w = WrappedBloomFilter::wrap(&golden).unwrap();
        assert_eq!(
            (w.capacity(), w.num_hashes(), w.seed(), w.bits_used()),
            (128, 3, u64::from_le_bytes(seed), 3)
        );
        assert_eq!(w.to_owned().serialize(), golden);
    }
}

#[test]
fn dirty_sentinel_is_accepted_and_replaced_by_the_true_count() {
    // An image written by a filter that was updated and not recounted carries
    // u64::MAX; upstream recounts the bit array on read and, since it then
    // treats the count as exact, writes the true count back out.
    let dirty = non_empty_golden(SEED_9001, DIRTY.to_le_bytes());
    let exact = non_empty_golden(SEED_9001, [3, 0, 0, 0, 0, 0, 0, 0]);

    let mut f = BloomFilter::deserialize(&dirty).unwrap();
    assert_eq!(f.bits_used(), 3);
    assert_eq!(f.serialize(), exact);

    let w = WrappedBloomFilter::wrap(&dirty).unwrap();
    assert_eq!(w.bits_used(), 3);
    assert_eq!(w.to_owned().serialize(), exact);
}

#[test]
fn updated_filter_header_and_count_are_exact() {
    for seed in [9001u64, 12345] {
        let mut f = BloomFilterBuilder::by_size(1024, 5)
            .seed(seed)
            .build()
            .unwrap();
        for i in 0..40u64 {
            f.update_u64(i);
        }
        // No bits_used() call first: the count must already be exact.
        let bytes = f.serialize();
        assert_eq!(bytes.len(), 32 + 16 * 8);
        assert_eq!(
            &bytes[0..4],
            &[4, 1, 21, 0],
            "preamble, version, family, flags"
        );
        assert_eq!(&bytes[4..8], &[5, 0, 0, 0], "num_hashes and unused");
        assert_eq!(&bytes[8..16], &seed.to_le_bytes());
        assert_eq!(
            &bytes[16..24],
            &[16, 0, 0, 0, 0, 0, 0, 0],
            "num_longs and unused"
        );

        let stored = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        let popcount: u64 = bytes[32..].iter().map(|b| u64::from(b.count_ones())).sum();
        assert_ne!(stored, DIRTY);
        assert_eq!(stored, popcount);
        assert!(popcount > 0 && popcount <= 40 * 5);
        assert_eq!(stored, f.bits_used());

        // Asking for the count changes nothing.
        assert_eq!(f.serialize(), bytes);
    }
}

#[test]
fn set_operations_and_invert_keep_the_stored_count_exact() {
    let mut a = BloomFilterBuilder::by_size(512, 3).build().unwrap();
    let mut b = BloomFilterBuilder::by_size(512, 3).build().unwrap();
    for i in 0..30u64 {
        a.update_u64(i);
        b.update_u64(i + 15);
    }
    let check = |f: &BloomFilter| {
        let bytes = f.serialize();
        let stored = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        let popcount: u64 = bytes[32..].iter().map(|x| u64::from(x.count_ones())).sum();
        assert_eq!(stored, popcount);
    };
    let mut u = a.clone();
    u.union_with(&b).unwrap();
    check(&u);
    let mut i = a.clone();
    i.intersect(&b).unwrap();
    check(&i);
    let mut inv = a.clone();
    inv.invert();
    check(&inv);
    assert_eq!(inv.bits_used(), 512 - a.bits_used());
}

#[test]
fn serialization_is_deterministic() {
    let build = || {
        let mut f = BloomFilterBuilder::by_accuracy(500, 0.01)
            .seed(77)
            .build()
            .unwrap();
        for i in 0..300u64 {
            f.update_u64(i);
        }
        f.update_str("x");
        f.update_f64(1.5);
        f
    };
    let (a, b) = (build(), build());
    assert_eq!(a.serialize(), b.serialize());
    assert_eq!(a.serialize(), a.serialize());
    assert_eq!(a.clone().serialize(), a.serialize());
    // update and query_and_update set the same bits.
    let mut c = BloomFilterBuilder::by_accuracy(500, 0.01)
        .seed(77)
        .build()
        .unwrap();
    for i in 0..300u64 {
        c.query_and_update_u64(i);
    }
    c.query_and_update_str("x");
    c.query_and_update_f64(1.5);
    assert_eq!(c.serialize(), a.serialize());
}

#[test]
fn wrap_and_deserialize_agree_on_every_query() {
    let mut images: Vec<Vec<u8>> = vec![
        empty_golden(3, SEED_9001, 2),
        empty_golden(3, SEED_12345, 2),
        non_empty_golden(SEED_9001, [3, 0, 0, 0, 0, 0, 0, 0]),
        non_empty_golden(SEED_12345, DIRTY.to_le_bytes()),
    ];
    let mut f = BloomFilterBuilder::by_accuracy(300, 0.02)
        .seed(12345)
        .build()
        .unwrap();
    for i in 0..300u64 {
        f.update_u64(i);
    }
    images.push(f.serialize());

    for (n, bytes) in images.iter().enumerate() {
        let mut d = BloomFilter::deserialize(bytes).unwrap();
        let w = WrappedBloomFilter::wrap(bytes).unwrap();
        assert_eq!(d.capacity(), w.capacity(), "image {n}");
        assert_eq!(d.num_hashes(), w.num_hashes(), "image {n}");
        assert_eq!(d.seed(), w.seed(), "image {n}");
        assert_eq!(d.is_empty(), w.is_empty(), "image {n}");
        assert_eq!(d.bits_used(), w.bits_used(), "image {n}");
        for i in 0..2_000u64 {
            assert_eq!(d.query_u64(i), w.query_u64(i), "image {n} item {i}");
        }
        assert_eq!(d.query_str("x"), w.query_str("x"));
        assert_eq!(d.query_f64(1.5), w.query_f64(1.5));
        assert_eq!(w.to_owned().serialize(), d.serialize(), "image {n}");
    }
}
