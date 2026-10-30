//! Standalone demo of the Bloom filter.
//!
//! Run with: `cargo run -p apache-datasketches --example bloom --features bloom`

use apache_datasketches::bloom::{BloomFilterBuilder, WrappedBloomFilter};

fn main() {
    // Size for 100 000 distinct items at a 1% false-positive target.
    let mut filter = BloomFilterBuilder::by_accuracy(100_000, 0.01)
        .build()
        .expect("valid configuration");
    println!(
        "filter: {} bits, {} hashes, seed {}",
        filter.capacity(),
        filter.num_hashes(),
        filter.seed()
    );

    // Add 50 000 user IDs. Members are always found: no false negatives.
    for id in 0..50_000u64 {
        filter.update_u64(id);
    }
    let found = (0..50_000u64).filter(|&id| filter.query_u64(id)).count();
    println!("members found: {found} of 50000");
    assert_eq!(found, 50_000);

    // Non-members are usually rejected; the ones that are not are false
    // positives. Measure the rate over 100 000 IDs that were never added.
    let false_positives = (1_000_000..1_100_000u64)
        .filter(|&id| filter.query_u64(id))
        .count();
    println!(
        "false-positive rate over 100000 non-members: {:.4}%",
        false_positives as f64 / 100_000.0 * 100.0
    );

    // `query_and_update` tests and adds in one call, which deduplicates a
    // stream: it returns whether the item may already have been present.
    let mut seen = BloomFilterBuilder::by_accuracy(1_000, 0.01)
        .build()
        .expect("valid configuration");
    let stream = ["a", "b", "a", "c", "b", "a"];
    let unique: Vec<&str> = stream
        .iter()
        .copied()
        .filter(|item| !seen.query_and_update_str(item))
        .collect();
    println!("stream {stream:?} deduplicated to {unique:?}");

    // Union two shard filters. They share the default seed, so they are
    // compatible.
    let mut shard_a = BloomFilterBuilder::by_size(4096, 3).build().unwrap();
    let mut shard_b = BloomFilterBuilder::by_size(4096, 3).build().unwrap();
    shard_a.update_str("alice");
    shard_b.update_str("bob");
    shard_a.union_with(&shard_b).expect("compatible filters");
    println!(
        "after union: alice={}, bob={}",
        shard_a.query_str("alice"),
        shard_a.query_str("bob")
    );

    // A filter with a different seed is not compatible.
    let other_seed = BloomFilterBuilder::by_size(4096, 3)
        .seed(12345)
        .build()
        .unwrap();
    match shard_a.union_with(&other_seed) {
        Ok(()) => println!("unexpected: union with a different seed succeeded"),
        Err(e) => println!("union with a different seed fails: {e}"),
    }

    // Serialize, then query the bytes in place without a full deserialize.
    let bytes = filter.serialize();
    let view = WrappedBloomFilter::wrap(&bytes).expect("valid image");
    println!(
        "wrapped {} bytes: member 42 -> {}, bits used {}",
        bytes.len(),
        view.query_u64(42),
        view.bits_used()
    );

    // Copy the view into an owned, mutable filter.
    let mut owned = view.to_owned();
    owned.update_u64(2_000_000);
    println!(
        "owned copy: capacity {}, bits used {}, contains 2000000 -> {}",
        owned.capacity(),
        owned.bits_used(),
        owned.query_u64(2_000_000)
    );
}
