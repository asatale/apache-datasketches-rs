#![cfg(feature = "bloom")]

use apache_datasketches_sys::bloom_filter::ffi;

const SEED: u64 = 9001;

#[test]
fn construct_update_query_roundtrip() {
    let mut f = ffi::bloom_new_by_accuracy(1000, 0.01, SEED).unwrap();
    assert!(f.is_empty());
    for i in 0..100u64 {
        f.pin_mut().update_u64(i);
    }
    f.pin_mut().update_str("hello");
    assert!(!f.is_empty());
    assert!(f.query_u64(42));
    assert!(f.query_str("hello"));
    assert_eq!(f.get_seed(), SEED);
    assert!(f.get_num_hashes() >= 1);
    assert_eq!(f.get_capacity() % 64, 0);
    let used = f.pin_mut().get_bits_used();
    assert!(used > 0 && used <= f.get_capacity());

    let mut bytes = vec![0u8; f.serialized_size_bytes() as usize];
    f.serialize_into(&mut bytes).unwrap();
    let mut too_short = vec![0u8; bytes.len() - 1];
    assert!(f.serialize_into(&mut too_short).is_err());
    let g = ffi::bloom_deserialize(&bytes).unwrap();
    assert!(g.query_u64(42));
    let w = ffi::bloom_wrap(&bytes).unwrap();
    assert!(w.query_u64(42));
    assert!(w.query_str("hello"));
    assert_eq!(w.get_bits_used(), used);
    assert!(f.is_compatible_with_wrapped(&w));

    let mut h = ffi::bloom_new_by_accuracy(1000, 0.01, SEED).unwrap();
    h.pin_mut().union_with_filter(&f).unwrap();
    assert!(h.query_u64(42));
    h.pin_mut().intersect_with_wrapped(&w).unwrap();
    assert!(h.query_u64(42));
    let c = h.clone_shim();
    assert!(c.query_u64(42));
}

#[test]
fn incompatible_set_operation_returns_err() {
    let mut a = ffi::bloom_new_by_size(1024, 3, SEED).unwrap();
    let b = ffi::bloom_new_by_size(1024, 3, SEED + 1).unwrap();
    assert!(!a.is_compatible_with_filter(&b));
    assert!(a.pin_mut().union_with_filter(&b).is_err());
}

#[test]
fn invalid_args_return_err() {
    assert!(ffi::bloom_new_by_size(0, 3, SEED).is_err());
    assert!(ffi::bloom_new_by_size(64, 0, SEED).is_err());
    assert!(ffi::bloom_new_by_accuracy(0, 0.01, SEED).is_err());
    assert!(ffi::bloom_deserialize(&[1, 2, 3]).is_err());
    assert!(ffi::bloom_wrap(&[1, 2, 3]).is_err());
}

#[test]
fn suggestions() {
    assert!(ffi::bloom_suggest_num_hashes(1000, 9586).unwrap() >= 1);
    assert!(ffi::bloom_suggest_num_hashes_for_fpp(0.01).unwrap() >= 1);
    assert!(ffi::bloom_suggest_num_filter_bits(1000, 0.01).unwrap() > 1000);
}
