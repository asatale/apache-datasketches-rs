//! Standalone demo of the CPC (Compressed Probabilistic Counting) sketch
//! and union APIs.
//!
//! Run with: `cargo run -p apache-datasketches --example cpc --features cpc`

use apache_datasketches::cpc::{CpcSketch, CpcSketchBuilder, CpcUnionBuilder};

fn main() {
    // A sketch estimates the number of distinct items seen, using bounded
    // memory regardless of how many items are added. `lg_k` (4..=26)
    // trades memory for accuracy: higher values are more accurate but use
    // more space.
    let mut visitors_day1 = CpcSketchBuilder::new()
        .lg_k(11)
        .build()
        .expect("valid lg_k");
    for id in 0..10_000u64 {
        visitors_day1.update_u64(id);
    }

    let mut visitors_day2 = CpcSketchBuilder::new()
        .lg_k(11)
        .build()
        .expect("valid lg_k");
    for id in 5_000..15_000u64 {
        visitors_day2.update_u64(id);
    }

    println!(
        "Day 1 unique visitors (estimate): {:.0}",
        visitors_day1.get_estimate()
    );
    println!(
        "Day 2 unique visitors (estimate): {:.0}",
        visitors_day2.get_estimate()
    );
    println!(
        "Day 1, 95% confidence interval: [{:.0}, {:.0}]",
        visitors_day1.get_lower_bound(2).unwrap(),
        visitors_day1.get_upper_bound(2).unwrap()
    );

    // CpcUnion merges multiple sketches into one, e.g. combining per-day
    // counts into a total distinct count across both days.
    let mut union = CpcUnionBuilder::new().lg_k(11).build().expect("valid lg_k");
    union.update(&visitors_day1);
    union.update(&visitors_day2);
    let total_unique = union.get_result();
    println!(
        "Total unique visitors across both days (true count = 15000): {:.0}",
        total_unique.get_estimate()
    );

    // Sketches can be serialized to bytes (e.g. to store or send over the
    // network) and reconstructed later. CPC's serialized form is always
    // compressed, so there's a single serialize()/deserialize() pair
    // (unlike Theta, which has separate compressed/uncompressed formats).
    let bytes = visitors_day1.serialize();
    let restored = CpcSketch::deserialize(&bytes).expect("valid sketch bytes");
    println!(
        "serialized day-1 sketch to {} bytes and restored successfully (estimate {:.0})",
        bytes.len(),
        restored.get_estimate()
    );
}
