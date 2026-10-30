use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};
use apache_datasketches::SketchError;

mod bloom_common;
use bloom_common::{assert_wrap_rejected, Image};

fn sample() -> BloomFilter {
    let mut f = BloomFilterBuilder::by_accuracy(1_000, 0.01)
        .build()
        .unwrap();
    for i in 0..500u64 {
        f.update_u64(i);
    }
    f.update_str("member");
    f
}

#[test]
fn wrapped_queries_match_owned() {
    let mut f = sample();
    let bytes = f.serialize();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    for i in 0..2_000u64 {
        assert_eq!(w.query_u64(i), f.query_u64(i), "{i}");
    }
    assert!(w.query_str("member"));
    assert_eq!(w.capacity(), f.capacity());
    assert_eq!(w.num_hashes(), f.num_hashes());
    assert_eq!(w.seed(), f.seed());
    assert_eq!(w.bits_used(), f.bits_used());
    assert_eq!(w.is_empty(), f.is_empty());
}

#[test]
fn to_owned_is_independent_and_mutable() {
    let f = sample();
    let bytes = f.serialize();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    let mut owned = w.to_owned();
    owned.update_u64(1_000_000);
    assert!(owned.query_u64(1_000_000));
    assert_eq!(bytes, f.serialize(), "wrapping must not modify the bytes");
}

#[test]
fn set_operations_accept_wrapped_inputs() {
    let f = sample();
    let bytes = f.serialize();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    let mut g = BloomFilterBuilder::by_accuracy(1_000, 0.01)
        .build()
        .unwrap();
    assert!(g.is_compatible(&w) && w.is_compatible(&g));
    g.union_with(&w).unwrap();
    assert!(g.query_u64(42));
    g.intersect(&w).unwrap();
    assert!(g.query_u64(42));
}

#[test]
fn empty_filter_wraps() {
    let f = BloomFilterBuilder::by_size(1024, 3).build().unwrap();
    let bytes = f.serialize();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    assert!(w.is_empty());
    assert!(!w.query_u64(1));
}

#[test]
fn garbage_bytes_fail() {
    assert!(matches!(
        WrappedBloomFilter::wrap(&[1, 2, 3]),
        Err(SketchError::Deserialization(_))
    ));
}

#[test]
fn preamble_3_non_empty_cannot_read_past_the_slice() {
    // Upstream requires only `prelongs * 8` = 24 bytes, then reads the
    // bits-set count at 24..32 and points the wrapped bit array at byte 32:
    // past the end of the borrowed slice. Here the bytes past the slice are
    // a consistent bit array, so unvalidated upstream would return a view
    // whose queries read memory the lifetime does not cover.
    let mut image = Image::non_empty(2, 3, 7, &[0, 1, 2, 3]);
    image.preamble_longs = 3;
    let bytes = image.bytes();
    assert_wrap_rejected(&bytes[..24], "preamble 3, sliced to 24 bytes");
    bloom_common::assert_deserialize_rejected(&bytes[..24], "preamble 3, sliced to 24 bytes");
}

#[test]
fn wrapped_crafted_capacity_zero_is_rejected() {
    // Capacity 0 from num_longs = 0 or a wrapped 32-bit shift. Queries on such
    // a view returned false early (`is_empty`) because the 0-byte bit array
    // counts 0 set bits, but `to_owned` would hand back an updatable filter.
    for num_longs in [0u32, 1 << 29, 0x8000_0000] {
        let mut image = Image::non_empty(1, 3, 7, &[]);
        image.num_longs = num_longs;
        image.bits.clear();
        assert_wrap_rejected(&image.bytes(), &format!("{num_longs:#x} longs"));
    }
}

#[test]
fn wrapped_valid_crafted_image_queries_and_converts() {
    let bytes = Image::non_empty(2, 2, 7, &[5, 77]).bytes();
    let w = WrappedBloomFilter::wrap(&bytes).unwrap();
    assert_eq!((w.capacity(), w.bits_used()), (128, 2));
    for i in 0..1_000u64 {
        let _ = w.query_u64(i);
    }
    let mut owned = w.to_owned();
    owned.update_u64(1);
    assert!(owned.query_and_update_u64(1));
}
