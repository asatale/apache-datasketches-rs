//! Standalone demo of the REQ (Relative Error Quantiles) sketch.
//!
//! Run with: `cargo run -p apache-datasketches --example req --features req`

use apache_datasketches::req::{RankAccuracy, ReqSketch};

fn main() {
    // REQ keeps its error relative to the rank, so with high-rank accuracy the
    // extreme tail (p99.9, p99.99) stays accurate even though the sketch is
    // small. `k` (even, 4..=254 here) trades memory for accuracy.
    let mut latencies = ReqSketch::<f64>::new(12, RankAccuracy::High).expect("valid k");
    for i in 0..1_000_000u32 {
        // A heavy-tailed "latency" distribution, in milliseconds.
        let x = f64::from(i % 10_000) / 10_000.0;
        latencies.update(1.0 + 1.0 / (1.0 - x * 0.9999));
    }

    println!("requests seen: {}", latencies.get_n());
    for rank in [0.5, 0.99, 0.999, 0.9999] {
        let q = latencies.get_quantile(rank, true).unwrap();
        let lo = latencies.get_rank_lower_bound(rank, 2).unwrap();
        let hi = latencies.get_rank_upper_bound(rank, 2).unwrap();
        println!(
            "p{:<6} = {q:>9.2} ms   (true rank in [{lo:.5}, {hi:.5}] at 2 std devs)",
            rank * 100.0
        );
    }

    // A-priori error estimate for planning a k.
    let err =
        ReqSketch::<f64>::a_priori_rank_error(12, 0.999, RankAccuracy::High, 1_000_000).unwrap();
    println!("expected 1-sigma rank error at p99.9 with k = 12: {err:.6}");

    // Merging requires the same RankAccuracy.
    let mut other = ReqSketch::<f64>::new(12, RankAccuracy::High).expect("valid k");
    other.update(5_000.0);
    latencies.merge(&other).expect("same accuracy merges");
    let low = ReqSketch::<f64>::new(12, RankAccuracy::Low).expect("valid k");
    println!(
        "merging a low-accuracy sketch fails: {}",
        latencies.merge(&low).is_err()
    );

    // A sorted view answers the same queries and can be shared across threads.
    let view = std::sync::Arc::new(latencies.sorted_view());
    println!("sorted view holds {} retained items", view.len());

    // f32 bytes are compatible with Java's ReqSketch; f64 bytes are C++-only.
    let bytes = latencies.serialize();
    let restored = ReqSketch::<f64>::deserialize(&bytes).expect("valid bytes");
    println!(
        "serialized to {} bytes; restored n = {}",
        bytes.len(),
        restored.get_n()
    );
}
