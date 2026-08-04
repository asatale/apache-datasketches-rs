//! Demonstrates the Theta sketch family: cardinality estimation, set
//! operations (union, intersection, a-not-b), and Jaccard similarity.
//!
//! Run with:
//!   cargo run --example theta --features theta

use apache_datasketches::theta::{
    jaccard_similarity, ThetaAnotB, ThetaIntersection, ThetaSketchBuilder, ThetaUnionBuilder,
};

fn main() {
    // Build two sketches representing two overlapping sets of user IDs.
    let mut visitors_day1 = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for id in 0..10_000u64 {
        visitors_day1.update_u64(id);
    }

    let mut visitors_day2 = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
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

    // Union: total unique visitors across both days.
    let mut union = ThetaUnionBuilder::new().lg_k(12).build().unwrap();
    union.update(&visitors_day1);
    union.update(&visitors_day2);
    let total_unique = union.get_result(true);
    println!(
        "Total unique visitors (union estimate): {:.0}",
        total_unique.get_estimate()
    );

    // Intersection: visitors who came back on day 2.
    let mut intersection = ThetaIntersection::new();
    intersection.update(&visitors_day1);
    intersection.update(&visitors_day2);
    match intersection.get_result(true) {
        Ok(returning) => println!(
            "Returning visitors (intersection estimate): {:.0}",
            returning.get_estimate()
        ),
        Err(e) => println!("No intersection result: {e}"),
    }

    // A-not-b: visitors who only came on day 1.
    let a_not_b = ThetaAnotB::new();
    let day1_only = a_not_b.compute(&visitors_day1, &visitors_day2, true);
    println!(
        "Day-1-only visitors (a-not-b estimate): {:.0}",
        day1_only.get_estimate()
    );

    // Jaccard similarity: how similar are the two days' visitor sets?
    let similarity = jaccard_similarity(&visitors_day1, &visitors_day2);
    println!(
        "Jaccard similarity: {:.3} (range [{:.3}, {:.3}])",
        similarity.estimate, similarity.lower_bound, similarity.upper_bound
    );

    // Serialize a compact sketch for storage/transmission, then restore it.
    let compact = visitors_day1.compact(true);
    let bytes = compact.serialize_compact();
    println!("Serialized day-1 sketch: {} bytes", bytes.len());
    let restored = apache_datasketches::theta::CompactThetaSketch::deserialize(&bytes).unwrap();
    println!("Restored estimate: {:.0}", restored.get_estimate());
}
