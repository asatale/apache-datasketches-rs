//! Wire-format compatibility. C++ and Java share the REQ format, so bytes
//! matching upstream C++'s layout are readable by Java's `ReqSketch` (float)
//! and Python's float REQ sketch. Upstream ships no REQ `.sk` fixtures, so
//! these goldens are derived by hand from `req_sketch::serialize` and
//! `req_compactor::serialize`.
//!
//! Layout used below (little endian). Preamble, 8 bytes: preamble ints (2, or
//! 4 once there are several levels), serial version 1, family 17, flags, `k`
//! (u16), number of levels, number of raw items. Raw items (`n <= 4`) follow
//! directly. Otherwise each level is a compactor block: `state` (u64),
//! `section_size_raw` (f32), `lg_weight` (u8), `num_sections` (u8), 2 bytes of
//! padding, `num_items` (u32), then the items.
//!
//! Facts taken from `req_compactor_impl.hpp`: a new compactor is created by
//! `grow()` with `sorted = true`, `state = 0`, `section_size_raw = k` and
//! `num_sections = 3`; `append` clears `sorted` once the compactor holds more
//! than one item; and in high-rank-accuracy mode it fills the item buffer
//! from the end, so `begin()` (and hence the serialized order) is the reverse
//! of the insertion order, while in low-rank-accuracy mode it is the
//! insertion order.
//!
//! Upstream's `get_serialized_size_bytes` counts a compactor header even for
//! raw items, but `serialize` writes none, so for `n` in 2..=4 the buffer
//! ends in 20 zero bytes after the raw items.

use apache_datasketches::req::{RankAccuracy, ReqSketch};
use apache_datasketches::SketchError;

const FAMILY: u8 = 17;
const IS_EMPTY: u8 = 1 << 2;
const IS_HIGH_RANK: u8 = 1 << 3;
const RAW_ITEMS: u8 = 1 << 4;
const LEVEL_ZERO_SORTED: u8 = 1 << 5;

#[test]
fn golden_empty() {
    let high = [
        2,
        1,
        FAMILY,
        IS_EMPTY | IS_HIGH_RANK | RAW_ITEMS | LEVEL_ZERO_SORTED,
        12,
        0,
        0,
        0,
    ];
    let low = [
        2,
        1,
        FAMILY,
        IS_EMPTY | RAW_ITEMS | LEVEL_ZERO_SORTED,
        12,
        0,
        0,
        0,
    ];
    assert_eq!(ReqSketch::<f32>::default().serialize(), high);
    assert_eq!(ReqSketch::<f64>::default().serialize(), high);
    assert_eq!(
        ReqSketch::<f32>::new(12, RankAccuracy::Low)
            .unwrap()
            .serialize(),
        low
    );
}

#[test]
fn golden_single_item() {
    // n = 1 <= 4, so raw items; one item keeps level zero sorted.
    let header = [
        2u8,
        1,
        FAMILY,
        IS_HIGH_RANK | RAW_ITEMS | LEVEL_ZERO_SORTED,
        12,
        0,
        1,
        1,
    ];
    let mut f = ReqSketch::<f32>::default();
    f.update(1.0);
    let expected: Vec<u8> = header.iter().copied().chain(1.0f32.to_le_bytes()).collect();
    assert_eq!(f.serialize(), expected);
    let mut d = ReqSketch::<f64>::default();
    d.update(1.0);
    let expected: Vec<u8> = header.iter().copied().chain(1.0f64.to_le_bytes()).collect();
    assert_eq!(d.serialize(), expected);
}

#[test]
fn golden_raw_items_unsorted() {
    // HRA appends 3.0, 1.0, 2.0 at buffer positions cap-1, cap-2, cap-3, and
    // begin() reads from cap-3: the order is 2.0, 1.0, 3.0. Three items
    // clear the sorted flag. The 20 trailing zeros are upstream's over-count.
    let header = [2u8, 1, FAMILY, IS_HIGH_RANK | RAW_ITEMS, 12, 0, 1, 3];
    let mut s = ReqSketch::<f32>::default();
    for v in [3.0f32, 1.0, 2.0] {
        s.update(v);
    }
    let mut expected: Vec<u8> = header.to_vec();
    for v in [2.0f32, 1.0, 3.0] {
        expected.extend(v.to_le_bytes());
    }
    expected.extend([0u8; 20]);
    assert_eq!(s.serialize(), expected);

    // Low-rank accuracy keeps insertion order.
    let header = [2u8, 1, FAMILY, RAW_ITEMS, 12, 0, 1, 3];
    let mut s = ReqSketch::<f32>::new(12, RankAccuracy::Low).unwrap();
    for v in [3.0f32, 1.0, 2.0] {
        s.update(v);
    }
    let mut expected: Vec<u8> = header.to_vec();
    for v in [3.0f32, 1.0, 2.0] {
        expected.extend(v.to_le_bytes());
    }
    expected.extend([0u8; 20]);
    assert_eq!(s.serialize(), expected);
}

#[test]
fn golden_one_level_exact() {
    // n = 5 > 4: not raw. One level, still below the nominal capacity
    // (2 * 3 * 12 = 72), so no compaction and the preamble stays at 2 ints.
    // Five items clear the sorted flag; HRA order is the reverse of 1..=5.
    let mut s = ReqSketch::<f32>::default();
    for v in 1..=5 {
        s.update(v as f32);
    }
    let mut expected: Vec<u8> = vec![2, 1, FAMILY, IS_HIGH_RANK, 12, 0, 1, 0];
    expected.extend(0u64.to_le_bytes()); // state
    expected.extend(12.0f32.to_le_bytes()); // section_size_raw
    expected.push(0); // lg_weight
    expected.push(3); // num_sections
    expected.extend([0u8, 0]); // padding
    expected.extend(5u32.to_le_bytes()); // num_items
    for v in [5.0f32, 4.0, 3.0, 2.0, 1.0] {
        expected.extend(v.to_le_bytes());
    }
    assert_eq!(s.serialize(), expected);
    assert_eq!(expected.len(), 8 + 20 + 5 * 4);
}

#[test]
fn empty_bytes_deserialize_as_either_type() {
    let bytes = ReqSketch::<f32>::default().serialize();
    let f = ReqSketch::<f32>::deserialize(&bytes).unwrap();
    let d = ReqSketch::<f64>::deserialize(&bytes).unwrap();
    assert!(f.is_empty() && d.is_empty());
    assert_eq!(f.get_k(), 12);
    assert_eq!(d.get_k(), 12);
    assert_eq!(f.rank_accuracy(), RankAccuracy::High);
}

#[test]
fn non_empty_bytes_of_the_other_type_are_rejected() {
    for n in [1u32, 3, 5, 10_000] {
        let mut f = ReqSketch::<f32>::default();
        let mut d = ReqSketch::<f64>::default();
        for i in 0..n {
            f.update(i as f32);
            d.update(f64::from(i));
        }
        assert!(
            matches!(
                ReqSketch::<f64>::deserialize(&f.serialize()),
                Err(SketchError::Deserialization(_))
            ),
            "f32 bytes as f64, n = {n}"
        );
        assert!(
            matches!(
                ReqSketch::<f32>::deserialize(&d.serialize()),
                Err(SketchError::Deserialization(_))
            ),
            "f64 bytes as f32, n = {n}"
        );
        // And each type reads its own.
        assert_eq!(
            ReqSketch::<f32>::deserialize(&f.serialize())
                .unwrap()
                .get_n(),
            u64::from(n)
        );
        assert_eq!(
            ReqSketch::<f64>::deserialize(&d.serialize())
                .unwrap()
                .get_n(),
            u64::from(n)
        );
    }
}

#[test]
fn roundtrip_preserves_accuracy_and_k() {
    for accuracy in [RankAccuracy::High, RankAccuracy::Low] {
        for k in [4, 12, 254] {
            let mut s = ReqSketch::<f32>::new(k, accuracy).unwrap();
            for i in 0..2_000 {
                s.update(i as f32);
            }
            let back = ReqSketch::<f32>::deserialize(&s.serialize()).unwrap();
            assert_eq!(back.get_k(), k);
            assert_eq!(back.rank_accuracy(), accuracy);
            assert_eq!(back.get_n(), 2_000);
            assert_eq!(back.get_num_retained(), s.get_num_retained());

            // Empty sketches keep both too.
            let empty = ReqSketch::<f32>::new(k, accuracy).unwrap();
            let back = ReqSketch::<f32>::deserialize(&empty.serialize()).unwrap();
            assert_eq!(back.get_k(), k);
            assert_eq!(back.rank_accuracy(), accuracy);
        }
    }
}

#[test]
fn nonempty_bytes_with_large_k_keep_their_k() {
    // Raw single-item bytes: preamble with k = 1024, then the item.
    let mut bytes = vec![2u8, 1, FAMILY, IS_HIGH_RANK | RAW_ITEMS | LEVEL_ZERO_SORTED];
    bytes.extend(1024u16.to_le_bytes());
    bytes.extend([1, 1]);
    bytes.extend(1.0f32.to_le_bytes());
    let s = ReqSketch::<f32>::deserialize(&bytes).unwrap();
    assert_eq!(s.get_k(), 1024);
    assert_eq!(s.get_n(), 1);

    // An empty sketch is rebuilt through the constructor, which truncates k
    // to 8 bits, so such bytes are rejected.
    let mut empty = vec![
        2u8,
        1,
        FAMILY,
        IS_EMPTY | IS_HIGH_RANK | RAW_ITEMS | LEVEL_ZERO_SORTED,
    ];
    empty.extend(1024u16.to_le_bytes());
    empty.extend([0, 0]);
    assert!(matches!(
        ReqSketch::<f32>::deserialize(&empty),
        Err(SketchError::Deserialization(_))
    ));
}
