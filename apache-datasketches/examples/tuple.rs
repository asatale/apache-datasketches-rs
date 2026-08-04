//! Demonstrates the ArrayOfDoubles Tuple sketch family: cardinality
//! estimation where each distinct key also carries a fixed-width array of
//! `f64` values, summed on collision — plus set operations (union,
//! intersection, a-not-b) and Jaccard similarity.
//!
//! Run with:
//!   cargo run --example tuple --features tuple

use apache_datasketches::tuple::{
    array_of_doubles_jaccard_similarity, ArrayOfDoublesAnotB, ArrayOfDoublesIntersection,
    ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder, CompactArrayOfDoublesSketch,
};

fn main() {
    // Two sketches of user IDs, each carrying [sessions, revenue] per user.
    let mut day1 = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();
    for id in 0..10_000u64 {
        day1.update_u64(id, &[1.0, 2.50]).unwrap();
    }

    let mut day2 = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();
    for id in 5_000..15_000u64 {
        day2.update_u64(id, &[1.0, 4.00]).unwrap();
    }

    println!("Day 1 unique users (estimate): {:.0}", day1.get_estimate());
    println!("Day 2 unique users (estimate): {:.0}", day2.get_estimate());
    println!("Values per entry: {}", day1.get_num_values());

    // Union: unique users across both days, with per-user values summed for
    // anyone who appeared on both.
    let mut union = ArrayOfDoublesUnionBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();
    union.update(&day1).unwrap();
    union.update(&day2).unwrap();
    let combined = union.get_result(true);
    println!(
        "Total unique users (union estimate): {:.0}",
        combined.get_estimate()
    );

    // Per-entry access is what distinguishes Tuple sketches from HLL/Theta/CPC:
    // scale the retained sample's revenue back up by 1/theta to estimate the
    // full population total.
    let retained_revenue: f64 = combined.entries().map(|(_, values)| values[1]).sum();
    println!(
        "Estimated total revenue: {:.2} (from {} retained entries, theta = {:.4})",
        retained_revenue / combined.get_theta(),
        combined.get_num_retained(),
        combined.get_theta()
    );

    // Intersection: users who came back on day 2.
    let mut intersection = ArrayOfDoublesIntersection::new(2).unwrap();
    intersection.update(&day1).unwrap();
    intersection.update(&day2).unwrap();
    match intersection.get_result(true) {
        Ok(returning) => println!(
            "Returning users (intersection estimate): {:.0}",
            returning.get_estimate()
        ),
        Err(e) => println!("No intersection result: {e}"),
    }

    // A-not-b: users who only came on day 1.
    let a_not_b = ArrayOfDoublesAnotB::new();
    let day1_only = a_not_b.compute(&day1, &day2, true).unwrap();
    println!(
        "Day-1-only users (a-not-b estimate): {:.0}",
        day1_only.get_estimate()
    );

    // Jaccard similarity of the two days' audiences.
    let similarity = array_of_doubles_jaccard_similarity(&day1, &day2).unwrap();
    println!(
        "Jaccard similarity: {:.3} (range [{:.3}, {:.3}])",
        similarity.estimate, similarity.lower_bound, similarity.upper_bound
    );

    // Serialize a compact sketch for storage/transmission, then restore it.
    let compact = day1.compact(true);
    let bytes = compact.serialize();
    println!("Serialized day-1 sketch: {} bytes", bytes.len());
    let restored = CompactArrayOfDoublesSketch::deserialize(&bytes).unwrap();
    println!(
        "Restored estimate: {:.0} ({} values per entry)",
        restored.get_estimate(),
        restored.get_num_values()
    );
}
