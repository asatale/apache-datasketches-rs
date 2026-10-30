//! Every row of the Bloom filter error table: bad builder and `suggest_*`
//! arguments and incompatible set operations are `InvalidConfig`; bad bytes
//! are `Deserialization`. Boundaries are probed on both sides without ever
//! allocating a maximum-size filter.

use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};
use apache_datasketches::SketchError;

mod bloom_common;
use bloom_common::{assert_rejected, Image};

/// Upstream's size cap as the binding documents it: 2^32 - 64 bits.
const MAX_BITS: u64 = (1 << 32) - 64;

fn invalid_config_message<T>(r: Result<T, SketchError>, what: &str) -> String {
    match r {
        Err(SketchError::InvalidConfig(m)) => m,
        Err(e) => panic!("{what}: expected InvalidConfig, got {e:?}"),
        Ok(_) => panic!("{what}: expected InvalidConfig, got Ok"),
    }
}

#[test]
fn max_distinct_items_zero_is_rejected() {
    invalid_config_message(BloomFilterBuilder::by_accuracy(0, 0.01).build(), "build");
    invalid_config_message(
        BloomFilterBuilder::suggest_num_filter_bits(0, 0.01),
        "suggest_num_filter_bits",
    );
    invalid_config_message(
        BloomFilterBuilder::suggest_num_hashes(0, 1024),
        "suggest_num_hashes",
    );
}

#[test]
fn target_fpp_outside_open_unit_interval_is_rejected() {
    for fpp in [
        0.0,
        1.0,
        -0.1,
        1.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        // Deviation: the binding requires the open interval (0, 1), so an fpp
        // of exactly 1.0 is rejected here.
        invalid_config_message(
            BloomFilterBuilder::by_accuracy(1000, fpp).build(),
            &format!("build {fpp}"),
        );
        invalid_config_message(
            BloomFilterBuilder::suggest_num_filter_bits(1000, fpp),
            &format!("suggest_num_filter_bits {fpp}"),
        );
        invalid_config_message(
            BloomFilterBuilder::suggest_num_hashes_for_fpp(fpp),
            &format!("suggest_num_hashes_for_fpp {fpp}"),
        );
    }
    BloomFilterBuilder::by_accuracy(1000, 0.5).build().unwrap();
    BloomFilterBuilder::suggest_num_hashes_for_fpp(0.5).unwrap();
}

#[test]
fn num_bits_boundaries() {
    let m = invalid_config_message(BloomFilterBuilder::by_size(0, 3).build(), "0 bits");
    assert!(m.contains("num_bits"), "{m}");
    for bits in [MAX_BITS + 1, u64::MAX] {
        let m = invalid_config_message(
            BloomFilterBuilder::by_size(bits, 3).build(),
            &format!("{bits} bits"),
        );
        assert!(m.contains("num_bits"), "{m}");
    }
    // One bit rounds up to one 64-bit word.
    let f = BloomFilterBuilder::by_size(1, 1).build().unwrap();
    assert_eq!(f.capacity(), 64);
    let f = BloomFilterBuilder::by_size(64, 1).build().unwrap();
    assert_eq!(f.capacity(), 64);
    let f = BloomFilterBuilder::by_size(65, 1).build().unwrap();
    assert_eq!(f.capacity(), 128);
}

#[test]
fn computed_num_bits_above_cap_is_rejected() {
    invalid_config_message(
        BloomFilterBuilder::by_accuracy(u64::MAX, 0.01).build(),
        "build",
    );
    invalid_config_message(
        BloomFilterBuilder::by_accuracy(1 << 40, 0.01).build(),
        "build 2^40 items",
    );
    invalid_config_message(
        BloomFilterBuilder::suggest_num_filter_bits(u64::MAX, 0.01),
        "suggest_num_filter_bits",
    );
}

#[test]
fn num_hashes_boundaries() {
    let m = invalid_config_message(BloomFilterBuilder::by_size(1024, 0).build(), "0 hashes");
    assert!(m.contains("num_hashes"), "{m}");
    // Upstream accepts 65535 but then hangs on the first update (its uint16_t
    // loop counter never exceeds the bound); the binding rejects it.
    let m = invalid_config_message(
        BloomFilterBuilder::by_size(1024, u16::MAX).build(),
        "65535 hashes",
    );
    assert!(m.contains("num_hashes"), "{m}");
    let mut f = BloomFilterBuilder::by_size(1024, 1).build().unwrap();
    f.update_u64(1);
    assert!(f.query_u64(1));
    let mut f = BloomFilterBuilder::by_size(1024, u16::MAX - 1)
        .build()
        .unwrap();
    f.update_u64(1);
    assert!(f.query_u64(1));
}

#[test]
fn suggest_argument_errors() {
    // num_filter_bits out of range
    for bits in [0, MAX_BITS + 1, u64::MAX] {
        let m = invalid_config_message(
            BloomFilterBuilder::suggest_num_hashes(1000, bits),
            &format!("{bits}"),
        );
        assert!(m.contains("num_filter_bits"), "{m}");
    }
    // suggested hash count above the 16-bit limit
    invalid_config_message(
        BloomFilterBuilder::suggest_num_hashes(1, MAX_BITS),
        "too many hashes",
    );
    // in-range arguments work
    assert_eq!(
        BloomFilterBuilder::suggest_num_hashes(1000, 1000).unwrap(),
        1
    );
    assert!(BloomFilterBuilder::suggest_num_filter_bits(1000, 0.01).unwrap() > 1000);
    assert!(BloomFilterBuilder::suggest_num_hashes_for_fpp(0.01).unwrap() >= 1);
}

#[test]
fn each_incompatibility_names_its_parameter() {
    let base = || BloomFilterBuilder::by_size(1024, 3).seed(1);
    let mut f = base().build().unwrap();

    let cases: [(BloomFilter, &str); 3] = [
        (
            BloomFilterBuilder::by_size(2048, 3)
                .seed(1)
                .build()
                .unwrap(),
            "capacity",
        ),
        (
            BloomFilterBuilder::by_size(1024, 4)
                .seed(1)
                .build()
                .unwrap(),
            "hashes",
        ),
        (
            BloomFilterBuilder::by_size(1024, 3)
                .seed(2)
                .build()
                .unwrap(),
            "seed",
        ),
    ];
    for (other, word) in &cases {
        assert!(!f.is_compatible(other));
        let m = invalid_config_message(f.union_with(other), "union");
        assert!(m.contains(word), "union: {m}");
        let m = invalid_config_message(f.intersect(other), "intersect");
        assert!(m.contains(word), "intersect: {m}");

        // The same through a wrapped argument.
        let bytes = other.serialize();
        let w = WrappedBloomFilter::wrap(&bytes).unwrap();
        assert!(!f.is_compatible(&w));
        let m = invalid_config_message(f.union_with(&w), "union wrapped");
        assert!(m.contains(word), "union wrapped: {m}");
        let m = invalid_config_message(f.intersect(&w), "intersect wrapped");
        assert!(m.contains(word), "intersect wrapped: {m}");
    }

    // A failed operation leaves the filter untouched.
    assert!(f.is_empty());
    let same = base().build().unwrap();
    assert!(f.is_compatible(&same));
    f.union_with(&same).unwrap();
    f.intersect(&same).unwrap();
}

#[test]
fn bad_bytes_are_deserialization_errors() {
    assert_rejected(&[], "empty slice");
    assert_rejected(&[3, 1, 21], "short header");

    let good = Image::non_empty(2, 3, 9001, &[0, 70, 127]);
    BloomFilter::deserialize(&good.bytes()).unwrap();

    let mut bad = Image::non_empty(2, 3, 9001, &[0]);
    bad.num_hashes = 0;
    assert_rejected(&bad.bytes(), "0 hashes");
    bad.num_hashes = u16::MAX;
    assert_rejected(&bad.bytes(), "65535 hashes");

    let mut bad = Image::non_empty(2, 3, 9001, &[0]);
    bad.num_longs = 0;
    assert_rejected(&bad.bytes(), "0 longs");
    let mut bad = Image::empty(1 << 26, 3, 9001);
    assert_rejected(&bad.bytes(), "2^26 longs");
    bad.num_longs = (1 << 26) - 1;
    BloomFilter::deserialize(&bad.bytes()).unwrap();

    let mut bad = Image::non_empty(2, 3, 9001, &[0]);
    bad.flags = 0x80;
    assert_rejected(&bad.bytes(), "unknown flag bit");

    let mut bad = Image::non_empty(2, 3, 9001, &[0]);
    bad.flags = bloom_common::EMPTY_FLAG;
    assert_rejected(&bad.bytes(), "empty flag on a 4-long preamble");
    let mut bad = Image::empty(2, 3, 9001);
    bad.flags = 0;
    assert_rejected(&bad.bytes(), "3-long preamble without the empty flag");

    let mut truncated = good.bytes();
    truncated.pop();
    assert_rejected(&truncated, "truncated bit array");

    let mut bad = Image::non_empty(2, 3, 9001, &[0]);
    bad.count = Some(5);
    assert_rejected(&bad.bytes(), "wrong count");
}
