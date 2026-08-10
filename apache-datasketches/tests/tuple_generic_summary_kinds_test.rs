//! Four summary shapes, each stressing something different, plus
//! estimation-mode coverage for every set operation from the outset.
//!
//! The per-component smoke tests all use one small `Sum`-like summary in exact
//! mode. This file is where the trait's range is actually exercised: a `Copy`
//! summary, a heap-owning one, an `Update` type unrelated to the summary type,
//! and one whose union and intersection semantics differ so a cross-wired
//! trampoline cannot hide. It also covers Jaccard's degenerate early exits and
//! the survival of its operands.

use apache_datasketches::tuple::generic::{
    tuple_jaccard_similarity, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnionBuilder,
};
use apache_datasketches::tuple::JaccardBounds;

/// 1. Trivial Copy summary.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Count(u64);

impl TupleSummary for Count {
    type Update = ();
    fn create(_: &()) -> Self {
        Count(1)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
}

/// 2. Heap-owning summary.
#[derive(Clone, Debug, PartialEq)]
struct Tags(Vec<String>);

impl TupleSummary for Tags {
    type Update = str;
    fn create(update: &str) -> Self {
        Tags(vec![update.to_string()])
    }
    fn union_combine(&mut self, other: &Self) {
        self.0.extend(other.0.iter().cloned());
        self.0.sort();
        self.0.dedup();
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0.retain(|t| other.0.contains(t));
    }
}

/// 3. Unsized Update that differs from the summary type.
#[derive(Clone, Debug, PartialEq)]
struct LenHistogram([u32; 4]);

impl TupleSummary for LenHistogram {
    type Update = str;
    fn create(update: &str) -> Self {
        let mut h = [0u32; 4];
        h[update.len().min(3)] = 1;
        LenHistogram(h)
    }
    fn union_combine(&mut self, other: &Self) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a += *b;
        }
    }
    fn intersection_combine(&mut self, other: &Self) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a = (*a).min(*b);
        }
    }
}

/// 4. Union and intersection semantics genuinely differ, so a cross-wired
///    trampoline is detectable.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SumOrMin(i64);

impl TupleSummary for SumOrMin {
    type Update = i64;
    fn create(update: &i64) -> Self {
        SumOrMin(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sum_or_min(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<SumOrMin> {
    let mut s: TupleSketch<SumOrMin> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

fn tagged(keys: std::ops::Range<u64>, tag: &str) -> TupleSketch<Tags> {
    let mut s: TupleSketch<Tags> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, tag);
    }
    s
}

#[test]
fn copy_summary_counts_occurrences() {
    let mut s: TupleSketch<Count> = TupleSketchBuilder::new().build().unwrap();
    for _ in 0..7 {
        s.update_u64(1, &());
    }
    let entries: Vec<(u64, Count)> = s.compact(true).entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, Count(7));
}

#[test]
fn heap_owning_summary_survives_round_trip() {
    let mut s: TupleSketch<Tags> = TupleSketchBuilder::new().build().unwrap();
    s.update_u64(1, "alpha");
    s.update_u64(1, "beta");
    s.update_u64(2, "gamma");
    let entries: Vec<(u64, Tags)> = s.compact(true).entries().collect();
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .any(|(_, t)| t.0 == vec!["alpha".to_string(), "beta".to_string()]),
        "expected an entry with [\"alpha\", \"beta\"]; got {entries:?}"
    );
    assert!(
        entries
            .iter()
            .any(|(_, t)| t.0 == vec!["gamma".to_string()]),
        "expected an entry with [\"gamma\"]; got {entries:?}"
    );
}

#[test]
fn unsized_update_type_works() {
    let mut s: TupleSketch<LenHistogram> = TupleSketchBuilder::new().build().unwrap();
    s.update_u64(1, "ab");
    s.update_u64(1, "xy");
    let entries: Vec<(u64, LenHistogram)> = s.compact(true).entries().collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one retained entry (both updates hit the same key); got {entries:?}"
    );
    assert_eq!(entries[0].1, LenHistogram([0, 0, 2, 0]));
}

#[test]
fn union_and_intersection_use_different_trampolines() {
    let a = sum_or_min(7..8, 10);
    let b = sum_or_min(7..8, 32);

    let mut u = TupleUnionBuilder::<SumOrMin>::new().build().unwrap();
    u.update(&a);
    u.update(&b);
    let unioned: Vec<(u64, SumOrMin)> = u.get_result(true).entries().collect();
    assert_eq!(unioned[0].1, SumOrMin(42), "union must sum");

    let mut i = TupleIntersection::<SumOrMin>::new();
    i.update(&a);
    i.update(&b);
    let intersected: Vec<(u64, SumOrMin)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(
        intersected[0].1,
        SumOrMin(10),
        "intersection must take the min"
    );
}

#[test]
fn union_in_estimation_mode() {
    let mut u = TupleUnionBuilder::<SumOrMin>::new().build().unwrap();
    u.update(&sum_or_min(0..30_000, 1));
    u.update(&sum_or_min(15_000..45_000, 1));
    let result = u.get_result(true);
    assert!(result.is_estimation_mode());
    let estimate = result.get_estimate();
    assert!(
        (estimate - 45_000.0).abs() < 45_000.0 * 0.03,
        "union estimate out of tolerance: {estimate}"
    );

    // Summary VALUES, not just cardinality: overlapping keys must carry the
    // combined SumOrMin(2), non-overlapping keys the untouched SumOrMin(1).
    // A cardinality-only assertion would pass even if union_combine never
    // fired, since the 15_000-key overlap changes no key.
    let values: Vec<i64> = result.entries().map(|(_, s)| s.0).collect();
    assert!(
        values.iter().all(|v| *v == 1 || *v == 2),
        "union must only ever produce 1 (no overlap) or 2 (combined); saw {values:?}"
    );
    assert!(
        values.contains(&2),
        "no overlapping key was retained with a combined value of 2 -- \
         union_combine may never have fired; values were {values:?}"
    );
}

#[test]
fn intersection_in_estimation_mode() {
    let mut i = TupleIntersection::<SumOrMin>::new();
    i.update(&sum_or_min(0..30_000, 1));
    i.update(&sum_or_min(15_000..45_000, 1));
    let result = i.get_result(true).unwrap();
    assert!(result.is_estimation_mode());
    let estimate = result.get_estimate();
    assert!(
        (estimate - 15_000.0).abs() < 15_000.0 * 0.05,
        "intersection estimate out of tolerance: {estimate}"
    );

    // Both operands use value 1, so min(1, 1) = 1 but sum(1, 1) = 2: a
    // trampoline cross-wired to union_combine (or made a no-op that keeps
    // the create()d value of 1 -- so this specifically needs the sum case)
    // is directly visible in the retained values. Estimation-mode
    // intersection values are otherwise unchecked anywhere in this suite.
    let values: Vec<i64> = result.entries().map(|(_, s)| s.0).collect();
    assert!(!values.is_empty(), "intersection retained no entries");
    assert!(
        values.iter().all(|v| *v == 1),
        "intersection must take the min (1), not the sum (2); saw {values:?} -- \
         a cross-wired trampoline would produce 2 here"
    );
}

#[test]
fn a_not_b_in_estimation_mode() {
    let calc = TupleAnotB::<SumOrMin>::new();
    let result = calc.compute(
        &sum_or_min(0..30_000, 1),
        &sum_or_min(15_000..45_000, 1),
        true,
    );
    assert!(result.is_estimation_mode());
    let estimate = result.get_estimate();
    assert!(
        (estimate - 15_000.0).abs() < 15_000.0 * 0.05,
        "a-not-b estimate out of tolerance: {estimate}"
    );

    // a-not-b invokes no summary policy at all (per shared context): it
    // copies operand a's summaries unchanged. Asserting the retained values
    // still closes the "summary reachable but unchecked" gap the review
    // flagged, even though there is no combine callback to catch here.
    let values: Vec<i64> = result.entries().map(|(_, s)| s.0).collect();
    assert!(!values.is_empty(), "a-not-b retained no entries");
    assert!(
        values.iter().all(|v| *v == 1),
        "a-not-b copies operand a's summaries unchanged; saw {values:?}"
    );
}

#[test]
fn jaccard_in_estimation_mode() {
    let bounds =
        tuple_jaccard_similarity(&sum_or_min(0..30_000, 1), &sum_or_min(15_000..45_000, 1));
    assert!(
        (bounds.estimate - 1.0 / 3.0).abs() < 0.05,
        "jaccard estimate out of tolerance: {bounds:?}"
    );
    // Non-degenerate interval: this holds only in estimation mode.
    assert!(
        bounds.lower_bound < bounds.upper_bound,
        "expected a non-degenerate interval in estimation mode: {bounds:?}"
    );
}

/// Both of jaccard's scratch objects deep-copy each summary, so a
/// move-instead-of-copy regression in the clone trampoline would leave the
/// operands' summaries disengaged. Heap-owning `Tags` makes that maximally
/// visible, and the assertion is on the summary VALUES, not just the count.
#[test]
fn jaccard_leaves_both_operands_intact() {
    let a = tagged(0..500, "alpha");
    let b = tagged(250..750, "beta");

    let bounds = tuple_jaccard_similarity(&a, &b);
    assert!((bounds.estimate - 1.0 / 3.0).abs() < 0.01);

    let ea: Vec<(u64, Tags)> = a.compact(true).entries().collect();
    assert_eq!(ea.len(), 500);
    assert!(
        ea.iter().all(|(_, t)| t.0 == vec!["alpha".to_string()]),
        "operand a's summaries were not left intact: {ea:?}"
    );

    let eb: Vec<(u64, Tags)> = b.compact(true).entries().collect();
    assert_eq!(eb.len(), 500);
    assert!(
        eb.iter().all(|(_, t)| t.0 == vec!["beta".to_string()]),
        "operand b's summaries were not left intact: {eb:?}"
    );
}

/// Upstream's `sketch_a.is_empty() && sketch_b.is_empty()` early exit.
#[test]
fn jaccard_of_two_empty_sketches_is_fully_similar() {
    let a: TupleSketch<SumOrMin> = TupleSketchBuilder::new().build().unwrap();
    let b: TupleSketch<SumOrMin> = TupleSketchBuilder::new().build().unwrap();
    assert_eq!(
        tuple_jaccard_similarity(&a, &b),
        JaccardBounds {
            lower_bound: 1.0,
            estimate: 1.0,
            upper_bound: 1.0,
        }
    );
}

/// Upstream's `sketch_a.is_empty() || sketch_b.is_empty()` early exit, in both
/// argument orders.
#[test]
fn jaccard_with_one_empty_operand_is_zero() {
    let empty: TupleSketch<SumOrMin> = TupleSketchBuilder::new().build().unwrap();
    let full = sum_or_min(0..1_000, 1);
    let zero = JaccardBounds {
        lower_bound: 0.0,
        estimate: 0.0,
        upper_bound: 0.0,
    };
    assert_eq!(tuple_jaccard_similarity(&empty, &full), zero);
    assert_eq!(tuple_jaccard_similarity(&full, &empty), zero);
}
