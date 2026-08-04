//! Standalone demo of the HLL (HyperLogLog) sketch and union APIs.
//!
//! Run with: `cargo run -p apache-datasketches --example hll`

use apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType};

fn main() {
    // A sketch estimates the number of distinct items seen, using bounded
    // memory regardless of how many items are added. `lg_config_k` (4..=21)
    // trades memory for accuracy: higher values are more accurate but use
    // more space.
    let mut sketch = HllSketch::new(12, TargetHllType::Hll4).expect("valid lg_config_k");

    for i in 0..10_000u64 {
        sketch.update_u64(i);
    }
    // Duplicates don't affect the distinct count.
    for i in 0..5_000u64 {
        sketch.update_u64(i);
    }
    sketch.update_str("some-key");
    sketch.update_bytes(b"raw bytes work too");

    println!("distinct count estimate: {:.1}", sketch.get_estimate());
    println!(
        "95% confidence interval: [{:.1}, {:.1}]",
        sketch.get_lower_bound(2).unwrap(),
        sketch.get_upper_bound(2).unwrap()
    );

    // Sketches can be serialized to bytes (e.g. to store or send over the
    // network) and reconstructed later.
    let bytes = sketch.serialize_compact();
    let restored = HllSketch::deserialize(&bytes).expect("valid sketch bytes");
    assert_eq!(sketch.get_estimate(), restored.get_estimate());
    println!(
        "serialized to {} bytes and restored successfully",
        bytes.len()
    );

    // HllUnion merges multiple sketches into one, e.g. combining
    // per-shard/per-day counts into a total distinct count.
    let mut shard_a = HllSketch::new(12, TargetHllType::Hll4).unwrap();
    for i in 0..10_000u64 {
        shard_a.update_u64(i);
    }
    let mut shard_b = HllSketch::new(12, TargetHllType::Hll4).unwrap();
    for i in 5_000..15_000u64 {
        shard_b.update_u64(i);
    }

    let mut union = HllUnion::new(12).expect("valid lg_max_k");
    union.update_sketch(&shard_a);
    union.update_sketch(&shard_b);

    let merged = union.get_result(TargetHllType::Hll4);
    println!(
        "merged distinct count across two overlapping shards (true count = 15000): {:.1}",
        merged.get_estimate()
    );
}
