//! Port of upstream `filters/test/bloom_filter_test.cpp` (datasketches-cpp
//! 5.3.0): one Rust test per upstream `TEST_CASE`, with the same explicit
//! seeds, sizes and exact expected values. The filter is deterministic given
//! its seed, so the exact counts upstream pins hold here too.
//!
//! Not ported, because the safe API does not expose them:
//! - `initialize_by_size` / `initialize_by_accuracy` / `get_wrapped_memory` /
//!   `is_memory_owned` (caller-provided memory); the in-memory leg of
//!   "basic operations" is replaced by a second identically-built filter that
//!   must serialize to identical bytes.
//! - `writable_wrap` and every mutation through a wrapped filter (the
//!   wrapper is read-only by construction: there is no mutating method to
//!   call, so upstream's `std::logic_error` checks cannot be written).
//! - `std::istream` / `std::ostream` serialization (Rust uses byte slices).
//! - `bit_array_ops_test.cpp` (internal helpers) and
//!   `bloom_filter_allocation_test.cpp` (custom allocator) are not ported.
//!
//! Deliberate deviations from upstream, each commented where it occurs: sizes
//! above `2^32 - 64` bits are rejected (`1 << 60` still fails, with a
//! different message), builder arguments are validated in Rust, and
//! exceptions map to `SketchError::InvalidConfig` (arguments, set
//! operations) or `SketchError::Deserialization` (bytes).

use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};
use apache_datasketches::SketchError;

mod bloom_common;

fn assert_invalid_config<T>(r: Result<T, SketchError>, what: &str) {
    match r {
        Err(SketchError::InvalidConfig(_)) => {}
        Err(e) => panic!("{what}: expected InvalidConfig, got {e:?}"),
        Ok(_) => panic!("{what}: expected InvalidConfig, got Ok"),
    }
}

#[test]
fn invalid_constructor_args() {
    assert_invalid_config(BloomFilterBuilder::by_size(0, 4).build(), "0 bits");
    // Upstream rejects 1 << 60 as too large; so do we, via the 2^32 - 64 cap.
    assert_invalid_config(
        BloomFilterBuilder::by_size(1u64 << 60, 4).build(),
        "1 << 60 bits",
    );
    assert_invalid_config(BloomFilterBuilder::by_size(65535, 0).build(), "0 hashes");
}

#[test]
fn standard_constructors() {
    let num_items = 4000u64;
    let fpp = 0.01;

    let num_bits = BloomFilterBuilder::suggest_num_filter_bits(num_items, fpp).unwrap();
    let num_hashes = BloomFilterBuilder::suggest_num_hashes(num_items, num_bits).unwrap();
    let seed = 89023u64;

    let bf = BloomFilterBuilder::by_size(num_bits, num_hashes)
        .seed(seed)
        .build()
        .unwrap();
    let adjusted_num_bits = (num_bits + 63) & !0x3F; // round up to a multiple of 64
    assert_eq!(bf.capacity(), adjusted_num_bits);
    assert_eq!(bf.num_hashes(), num_hashes);
    assert_eq!(bf.seed(), seed);
    assert!(bf.is_empty());

    // should match above
    let bf = BloomFilterBuilder::by_accuracy(num_items, fpp)
        .seed(seed)
        .build()
        .unwrap();
    assert_eq!(bf.capacity(), adjusted_num_bits);
    assert_eq!(bf.num_hashes(), num_hashes);
    assert_eq!(bf.seed(), seed);
    assert!(bf.is_empty());
}

#[test]
fn basic_operations() {
    let num_items = 5000u64;
    let fpp = 0.01;
    let seed = 4897301548054u64;

    let build = || {
        BloomFilterBuilder::by_accuracy(num_items, fpp)
            .seed(seed)
            .build()
            .unwrap()
    };
    let mut bf = build();
    assert!(bf.is_empty());
    assert_eq!(bf.bits_used(), 0);

    for i in 0..num_items {
        bf.query_and_update_u64(i);
    }
    assert!(!bf.is_empty());
    // Exact because the seed is fixed; the approximate check is the
    // "expectation" (filter about half full at target capacity).
    assert_eq!(bf.bits_used(), 24793);
    let half = 0.5 * bf.capacity() as f64;
    assert!((bf.bits_used() as f64 - half).abs() <= 0.05 * half);

    let num_found = (num_items..bf.capacity())
        .filter(|&i| bf.query_u64(i))
        .count();
    assert_eq!(num_found, 423);
    let bytes = bf.serialize();

    // Upstream re-runs the sequence on a filter living in caller-provided
    // memory. The equivalent here: an identically built filter, updated with
    // plain `update_u64`, must agree exactly.
    let mut bf2 = build();
    assert!(bf2.is_empty());
    assert_eq!(bf2.bits_used(), 0);
    for i in 0..num_items {
        bf2.update_u64(i);
    }
    assert!(!bf2.is_empty());
    assert_eq!(bf2.bits_used(), bf.bits_used());
    let num_found2 = (num_items..bf2.capacity())
        .filter(|&i| bf2.query_u64(i))
        .count();
    assert_eq!(num_found, num_found2);
    let bytes2 = bf2.serialize();
    assert_eq!(bytes.len(), bytes2.len());
    assert_eq!(bytes, bytes2);

    // ensure the filters reset properly
    bf.reset();
    assert!(bf.is_empty());
    assert_eq!(bf.bits_used(), 0);
    bf2.reset();
    assert!(bf2.is_empty());
    assert_eq!(bf2.bits_used(), 0);
}

#[test]
fn inversion() {
    let num_bits = 8192u64;
    let mut bf = BloomFilterBuilder::by_size(num_bits, 3).build().unwrap();

    let n = 500u64;
    for i in 0..n {
        bf.update_u64(i);
    }
    let num_bits_set = bf.bits_used();
    bf.invert();
    assert_eq!(bf.bits_used(), num_bits - num_bits_set);

    // original items should be mostly not-present
    let num_found = (0..n).filter(|&i| bf.query_u64(i)).count() as u64;
    assert!(num_found < n / 10);

    // many other items should be "present"
    let num_found = (n..num_bits).filter(|&i| bf.query_u64(i)).count() as u64;
    assert!(num_found > n);
}

#[test]
fn incompatible_set_operations() {
    let num_bits = 32768u64;
    let num_hashes = 4u16;

    let mut bf1 = BloomFilterBuilder::by_size(num_bits, num_hashes)
        .build()
        .unwrap();

    // mismatched num bits
    let bf2 = BloomFilterBuilder::by_size(2 * num_bits, num_hashes)
        .build()
        .unwrap();
    assert_invalid_config(bf1.union_with(&bf2), "union, capacity");
    // Upstream's second case intersects with bf2 (its comment says "mismatched
    // num hashes" but it never uses bf3); mirror that and also try bf3.
    assert_invalid_config(bf1.intersect(&bf2), "intersect, capacity");

    // mismatched num hashes
    let bf3 = BloomFilterBuilder::by_size(num_bits, 2 * num_hashes)
        .build()
        .unwrap();
    assert_invalid_config(bf1.union_with(&bf3), "union, hashes");
    assert_invalid_config(bf1.intersect(&bf3), "intersect, hashes");

    // mismatched seed
    let bf4 = BloomFilterBuilder::by_size(num_bits, num_hashes)
        .seed(bf1.seed() + 1)
        .build()
        .unwrap();
    assert_invalid_config(bf1.union_with(&bf4), "union, seed");
    assert_invalid_config(bf1.intersect(&bf4), "intersect, seed");
}

#[test]
fn basic_union() {
    let num_bits = 12288u64;
    let mut bf1 = BloomFilterBuilder::by_size(num_bits, 4).build().unwrap();
    let mut bf2 = BloomFilterBuilder::by_size(num_bits, 4)
        .seed(bf1.seed())
        .build()
        .unwrap();

    let n = 1000u64;
    let max_item = 3 * n / 2 - 1;
    for i in 0..n {
        bf1.query_and_update_u64(i);
        bf2.update_u64(n / 2 + i);
    }

    bf1.union_with(&bf2).unwrap();
    for i in 0..max_item {
        assert!(bf1.query_u64(i), "{i}");
    }

    let num_found = (max_item..num_bits).filter(|&i| bf1.query_u64(i)).count() as u64;
    assert!(num_found < num_bits / 10); // not being super strict
}

#[test]
fn basic_intersection() {
    let num_bits = 8192u64;
    let mut bf1 = BloomFilterBuilder::by_size(num_bits, 5).build().unwrap();
    let mut bf2 = BloomFilterBuilder::by_size(num_bits, 5)
        .seed(bf1.seed())
        .build()
        .unwrap();

    let n = 1024u64;
    let max_item = 3 * n / 2 - 1;
    for i in 0..n {
        bf1.update_u64(i);
        bf2.update_u64(n / 2 + i);
    }

    bf1.intersect(&bf2).unwrap();
    // overlap bits should all be set
    for i in n / 2..n {
        assert!(bf1.query_u64(i), "{i}");
    }

    let num_found = (0..n / 2).filter(|&i| bf1.query_u64(i)).count()
        + (max_item..num_bits).filter(|&i| bf1.query_u64(i)).count();
    assert!((num_found as u64) < num_bits / 10); // not being super strict
}

#[test]
fn empty_serialization() {
    let mut bf = BloomFilterBuilder::by_size(32769, 7).build().unwrap();
    let bytes = bf.serialize();
    // Empty images carry no bit array: 3 preamble longs.
    assert_eq!(bytes.len(), 24);

    let de = BloomFilter::deserialize(&bytes).unwrap();
    assert_eq!(bf.capacity(), de.capacity());
    assert_eq!(bf.seed(), de.seed());
    assert_eq!(bf.num_hashes(), de.num_hashes());
    assert!(de.is_empty());

    // read-only wrap should work
    let wrap = WrappedBloomFilter::wrap(&bytes).unwrap();
    assert_eq!(bf.capacity(), wrap.capacity());
    assert_eq!(bf.seed(), wrap.seed());
    assert_eq!(bf.num_hashes(), wrap.num_hashes());
    assert!(wrap.is_empty());
    assert_eq!(wrap.bits_used(), 0);
    assert_eq!(bf.bits_used(), 0);
}

#[test]
fn non_empty_serialization() {
    let num_bits = 32768u64;
    let mut bf = BloomFilterBuilder::by_size(num_bits, 5).build().unwrap();
    let n = 1000u64;
    for i in 0..n {
        bf.update_f64(0.5 + i as f64); // testing floats
    }

    // Query more items without updating, assuming some false positives, so we
    // can check that every restored copy reports the same ones.
    let fp_count = (n..num_bits)
        .filter(|&i| bf.query_f64(0.5 + i as f64))
        .count();

    let bytes = bf.serialize();
    // 4 preamble longs plus the bit array.
    assert_eq!(bytes.len() as u64, 32 + num_bits / 8);

    let de = BloomFilter::deserialize(&bytes).unwrap();
    assert_eq!(bf.capacity(), de.capacity());
    assert_eq!(bf.seed(), de.seed());
    assert_eq!(bf.num_hashes(), de.num_hashes());
    assert!(!de.is_empty());
    let mut fp_count_bytes = 0;
    for i in 0..num_bits {
        let val = de.query_f64(0.5 + i as f64);
        if i < n {
            assert!(val, "{i}");
        } else if val {
            fp_count_bytes += 1;
        }
    }
    assert_eq!(fp_count_bytes, fp_count);

    // read-only wrap
    let wrap = WrappedBloomFilter::wrap(&bytes).unwrap();
    assert_eq!(bf.capacity(), wrap.capacity());
    assert_eq!(bf.seed(), wrap.seed());
    assert_eq!(bf.num_hashes(), wrap.num_hashes());
    assert!(!wrap.is_empty());
    let mut fp_count_wrap = 0;
    for i in 0..num_bits {
        let val = wrap.query_f64(0.5 + i as f64);
        if i < n {
            assert!(val, "{i}");
        } else if val {
            fp_count_wrap += 1;
        }
    }
    assert_eq!(fp_count_wrap, fp_count);

    // Upstream's writable_wrap leg updates the shared bytes and checks the
    // read-only view sees it; the wrapper has no mutators, so the nearest
    // equivalent is that a value never added stays absent, and an owned copy
    // can take it without touching the borrowed bytes.
    assert!(!wrap.query_f64(-1.0));
    let mut owned = wrap.to_owned();
    owned.update_f64(-1.0);
    assert!(owned.query_f64(-1.0));
    assert!(!wrap.query_f64(-1.0));
    assert_eq!(bytes, bf.serialize());
}

#[test]
fn inconsistent_num_bits_set_is_rejected() {
    const NUM_BITS_SET_OFFSET: usize = 24;

    let mut bf = BloomFilterBuilder::by_accuracy(100, 0.01).build().unwrap();
    bf.update_str("apple");
    bf.update_str("banana");
    let actual = bf.bits_used();
    assert!(actual > 1);

    // The honest image is accepted by both entry points.
    let good = bf.serialize();
    assert_eq!(
        u64::from_le_bytes(
            good[NUM_BITS_SET_OFFSET..NUM_BITS_SET_OFFSET + 8]
                .try_into()
                .unwrap()
        ),
        actual
    );
    BloomFilter::deserialize(&good).unwrap();
    WrappedBloomFilter::wrap(&good).unwrap();

    for count in [0, actual - 1, actual + 1] {
        let mut bytes = good.clone();
        bytes[NUM_BITS_SET_OFFSET..NUM_BITS_SET_OFFSET + 8].copy_from_slice(&count.to_le_bytes());
        bloom_common::assert_rejected(&bytes, &format!("count {count}"));
    }
    // (Upstream's writable_wrap and stream-deserialize legs have no counterpart here.)
}
