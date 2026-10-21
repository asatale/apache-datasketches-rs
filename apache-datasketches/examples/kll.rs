//! Standalone demo of the KLL quantiles sketch.
//!
//! Run with: `cargo run -p apache-datasketches --example kll --features kll`

use apache_datasketches::kll::{kolmogorov_smirnov_test, KllSketch};

fn main() {
    // A KLL sketch summarizes a stream of numbers so you can ask for ranks
    // and quantiles (p50, p99, ...) afterwards, in bounded memory. `k` trades
    // memory for accuracy; 200 gives about 1.33% rank error (1.65% for PMF).
    let mut monday = KllSketch::<f64>::new(200).expect("valid k");
    for i in 0..100_000u32 {
        // A skewed "latency" distribution, in milliseconds.
        let x = f64::from(i % 1000) / 1000.0;
        monday.update(5.0 + 200.0 * x * x * x);
    }

    println!("requests seen: {}", monday.get_n());
    println!(
        "p50 = {:.1} ms, p99 = {:.1} ms, max = {:.1} ms",
        monday.get_quantile(0.50, true).unwrap(),
        monday.get_quantile(0.99, true).unwrap(),
        monday.get_max_item().unwrap()
    );
    println!(
        "fraction of requests under 50 ms: {:.3}",
        monday.get_rank(50.0, true).unwrap()
    );

    // PMF: the share of requests falling into each bucket.
    let buckets = [10.0, 50.0, 100.0];
    let pmf = monday.get_pmf(&buckets, true).unwrap();
    println!(
        "<=10ms {:.3}, <=50ms {:.3}, <=100ms {:.3}, >100ms {:.3}",
        pmf[0], pmf[1], pmf[2], pmf[3]
    );

    // Sketches merge, e.g. per-server sketches into a fleet-wide one.
    let mut tuesday = KllSketch::<f64>::new(200).expect("valid k");
    for i in 0..100_000u32 {
        let x = f64::from(i % 1000) / 1000.0;
        tuesday.update(5.0 + 400.0 * x * x * x);
    }
    let mut both = monday.clone();
    both.merge(&tuesday);
    println!(
        "both days, p99 = {:.1} ms",
        both.get_quantile(0.99, true).unwrap()
    );

    // Did the distribution change between the two days?
    let changed = kolmogorov_smirnov_test(&monday, &tuesday, 0.05).unwrap();
    println!("distribution changed (p = 0.05): {changed}");

    // A sorted view answers the same queries and can be shared across threads.
    let view = std::sync::Arc::new(both.sorted_view());
    println!("sorted view holds {} retained items", view.len());

    // Bytes are compatible with the Java and Python KllDoublesSketch.
    let bytes = monday.serialize();
    let restored = KllSketch::<f64>::deserialize(&bytes).expect("valid bytes");
    println!(
        "serialized to {} bytes; restored n = {}",
        bytes.len(),
        restored.get_n()
    );
}
