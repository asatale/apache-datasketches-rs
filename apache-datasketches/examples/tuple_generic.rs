//! Demonstrates generic Tuple sketches: cardinality estimation where each
//! distinct key carries a summary of a type you define in Rust.
//!
//! Run with:
//!   cargo run --example tuple_generic --features tuple

use apache_datasketches::tuple::generic::{
    tuple_jaccard_similarity, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnionBuilder,
};

/// Per-user session statistics. This is the sort of summary the fixed
/// `f64`-array shape of `ArrayOfDoublesSketch` cannot express: it mixes
/// counters with a max and a set of strings.
#[derive(Clone, Debug)]
struct Activity {
    sessions: u32,
    revenue_cents: u64,
    largest_order_cents: u64,
    countries: Vec<String>,
}

/// What a single event contributes.
struct Event<'a> {
    revenue_cents: u64,
    country: &'a str,
}

impl TupleSummary for Activity {
    // `Update` is a plain associated type with no lifetime parameter, so an
    // impl cannot name a borrowed lifetime here: `Event<'static>` is how an
    // update type holds a borrowed field.
    type Update = Event<'static>;

    fn create(event: &Event<'static>) -> Self {
        Activity {
            sessions: 1,
            revenue_cents: event.revenue_cents,
            largest_order_cents: event.revenue_cents,
            countries: vec![event.country.to_string()],
        }
    }

    fn union_combine(&mut self, other: &Self) {
        self.sessions += other.sessions;
        self.revenue_cents += other.revenue_cents;
        self.largest_order_cents = self.largest_order_cents.max(other.largest_order_cents);
        self.countries.extend(other.countries.iter().cloned());
        self.countries.sort();
        self.countries.dedup();
    }

    fn intersection_combine(&mut self, other: &Self) {
        // For an intersection we want only what both sides saw.
        self.sessions = self.sessions.min(other.sessions);
        self.revenue_cents = self.revenue_cents.min(other.revenue_cents);
        self.largest_order_cents = self.largest_order_cents.min(other.largest_order_cents);
        self.countries.retain(|c| other.countries.contains(c));
    }
}

fn main() {
    let mut january: TupleSketch<Activity> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    for user in 0..10_000u64 {
        january.update_u64(
            user,
            &Event {
                revenue_cents: 250 + (user % 100),
                country: if user % 2 == 0 { "GB" } else { "US" },
            },
        );
    }

    let mut february: TupleSketch<Activity> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    for user in 5_000..15_000u64 {
        february.update_u64(
            user,
            &Event {
                revenue_cents: 400,
                country: "US",
            },
        );
    }

    println!("January unique users:  {:.0}", january.get_estimate());
    println!("February unique users: {:.0}", february.get_estimate());

    // Union: everyone who appeared in either month, with their activity merged.
    let mut union = TupleUnionBuilder::<Activity>::new()
        .lg_k(12)
        .build()
        .unwrap();
    union.update(&january);
    union.update(&february);
    let combined = union.get_result(true);
    println!("Users across both months: {:.0}", combined.get_estimate());

    // Per-entry summaries are the point of a Tuple sketch. Scale the retained
    // sample back up by 1/theta to estimate population totals.
    let retained_revenue: u64 = combined.entries().map(|(_, a)| a.revenue_cents).sum();
    let biggest_order = combined
        .entries()
        .map(|(_, a)| a.largest_order_cents)
        .max()
        .unwrap_or(0);
    println!(
        "Estimated total revenue: {:.2} (from {} retained entries, theta = {:.4})",
        (retained_revenue as f64 / combined.get_theta()) / 100.0,
        combined.get_num_retained(),
        combined.get_theta()
    );
    println!(
        "Largest single order seen: {:.2}",
        biggest_order as f64 / 100.0
    );

    // Intersection: users active in both months.
    let mut intersection = TupleIntersection::<Activity>::new();
    intersection.update(&january);
    intersection.update(&february);
    match intersection.get_result(true) {
        Ok(returning) => println!("Returning users: {:.0}", returning.get_estimate()),
        Err(e) => println!("No intersection result: {e}"),
    }

    // A-not-b: users who churned after January.
    let churned = TupleAnotB::<Activity>::new().compute(&january, &february, true);
    println!("Churned after January: {:.0}", churned.get_estimate());

    // Jaccard similarity of the two months' audiences.
    let similarity = tuple_jaccard_similarity(&january, &february);
    println!(
        "Audience overlap (Jaccard): {:.3} (range [{:.3}, {:.3}])",
        similarity.estimate, similarity.lower_bound, similarity.upper_bound
    );
}
