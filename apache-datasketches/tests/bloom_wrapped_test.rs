use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};
use apache_datasketches::SketchError;

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
