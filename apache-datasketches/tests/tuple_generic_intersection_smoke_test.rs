use apache_datasketches::tuple::generic::{
    TupleIntersection, TupleSketch, TupleSketchBuilder, TupleSummary,
};
use apache_datasketches::SketchError;

#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sketch(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<Sum> {
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

#[test]
fn intersection_half_overlap() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(0..1000, 1));
    i.update(&sketch(500..1500, 1));
    assert_eq!(i.get_result(true).unwrap().get_estimate(), 500.0);
}

// The operand order is load-bearing. `Sum::intersection_combine` is `min`, so
// the *second* operand must carry the smaller value for the assertion to
// separate all three outcomes:
//
//   Sum(32) -> `intersection_combine` never ran,
//   Sum(42) -> `union_combine` ran instead (the two trampolines are crossed),
//   Sum(10) -> `intersection_combine` ran, which is correct.
//
// Feeding 10 before 32 would collapse "correct" and "never ran" onto Sum(10).
#[test]
fn intersection_uses_intersection_semantics_not_union() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(7..8, 32));
    i.update(&sketch(7..8, 10));
    let entries: Vec<(u64, Sum)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].1,
        Sum(10),
        "min; Sum(42) would mean union semantics leaked in, Sum(32) that no combine ran"
    );
}

// A summary whose `intersection_combine` is deliberately NON-commutative, so
// the retained-vs-incoming operand order is observable. `min` cannot see it:
// swapping the two arguments of `min` changes nothing.
//
// Upstream calls the policy as `policy_(*result.first, entry)`, i.e. the
// summary already retained by the intersection is `self` and the summary from
// the incoming sketch is `other`
// (theta/include/theta_intersection_base_impl.hpp, the match branch).
#[derive(Clone, Debug, PartialEq)]
struct Trace(i64);

impl TupleSummary for Trace {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Trace(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0 * 100 + other.0;
    }
}

fn trace_sketch(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<Trace> {
    let mut s: TupleSketch<Trace> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

#[test]
fn intersection_combine_receives_retained_as_self_and_incoming_as_other() {
    let mut i: TupleIntersection<Trace> = TupleIntersection::new();
    i.update(&trace_sketch(7..8, 3));
    i.update(&trace_sketch(7..8, 4));
    let entries: Vec<(u64, Trace)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].1,
        Trace(304),
        "self must be the retained summary (3) and other the incoming one (4); \
         Trace(403) would mean the operands are swapped"
    );
}

#[test]
fn intersection_combine_chains_across_three_operands() {
    let mut i: TupleIntersection<Trace> = TupleIntersection::new();
    i.update(&trace_sketch(7..8, 1));
    i.update(&trace_sketch(7..8, 2));
    i.update(&trace_sketch(7..8, 3));
    let entries: Vec<(u64, Trace)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(entries.len(), 1);
    // (1 * 100 + 2) * 100 + 3
    assert_eq!(entries[0].1, Trace(10203));
}

// The COMPACT operand is second and carries the smaller value, matching
// `intersection_uses_intersection_semantics_not_union` above: with 32 seeding
// the table and 10 arriving from the compact operand, the three outcomes are
// distinct -- Sum(10) correct, Sum(32) no combine ran, Sum(42) union
// semantics leaked in. Equal values on both sides (as this test used to have)
// would make "correct" and "never ran" the same reading, so a value assertion
// alone would not have fixed it.
#[test]
fn intersection_accepts_both_input_types() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(0..100, 32));
    i.update(&sketch(50..150, 10).compact(true));
    let result = i.get_result(true).unwrap();
    assert_eq!(result.get_estimate(), 50.0);

    let values: Vec<Sum> = result.entries().map(|(_, s)| s).collect();
    assert_eq!(values.len(), 50);
    assert!(
        values.iter().all(|s| *s == Sum(10)),
        "every retained summary must be min(32, 10) = 10; Sum(32) would mean \
         no combine ran on the compact-operand path and Sum(42) that union \
         semantics leaked in; saw {values:?}"
    );
}

// Keys present in only one operand contribute nothing at all: upstream invokes
// the policy only on a match. Every retained summary here must be the combined
// value, and the count must be the overlap, not the union.
#[test]
fn non_matching_keys_are_dropped_without_combining() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(0..8, 32));
    i.update(&sketch(4..12, 10));
    let mut entries: Vec<(u64, Sum)> = i.get_result(true).unwrap().entries().collect();
    entries.sort_by_key(|(k, _)| *k);
    assert_eq!(entries.len(), 4);
    for (_, summary) in &entries {
        assert_eq!(*summary, Sum(10));
    }
}

// Disjoint operands are a DEFINED state whose result is empty -- distinct from
// the no-operand state below, which upstream treats as the undefined infinite
// "universe" and reports by throwing.
#[test]
fn disjoint_operands_give_a_defined_empty_result() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(0..100, 1));
    i.update(&sketch(100..200, 1));
    assert!(i.has_result());
    let result = i.get_result(true).unwrap();
    assert!(result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
    assert_eq!(result.get_estimate(), 0.0);
    assert_eq!(result.entries().count(), 0);
}

#[test]
fn get_result_before_update_is_empty_intersection_err() {
    let i: TupleIntersection<Sum> = TupleIntersection::new();
    assert!(!i.has_result());
    assert!(matches!(
        i.get_result(true),
        Err(SketchError::EmptyIntersection)
    ));
}

// Self-intersection would alias the stored and incoming summaries if
// `DynSummary`'s copy constructor were shallow: `DynIntersectionPolicy` hands
// the trampoline a `RustSummary&` and a `const RustSummary&`, which would be a
// `&mut` and a `&` to one Rust value. `DynSummary` deep-copies, so this must be
// sound and every summary must be combined with a copy of itself.
#[test]
fn self_intersection_deep_copies_summaries() {
    let a = trace_sketch(0..4, 5);
    let mut i: TupleIntersection<Trace> = TupleIntersection::new();
    i.update(&a);
    i.update(&a);
    let entries: Vec<(u64, Trace)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(entries.len(), 4);
    for (_, summary) in entries {
        assert_eq!(summary, Trace(505));
    }
}

#[test]
fn intersection_get_result_unordered_matches_ordered() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(0..100, 1));
    i.update(&sketch(50..150, 1));

    let ordered = i.get_result(true).unwrap();
    let unordered = i.get_result(false).unwrap();

    assert_eq!(ordered.get_num_retained(), unordered.get_num_retained());
    assert!(ordered.is_ordered());
    assert!(!unordered.is_ordered());
}

#[test]
fn intersection_is_send_and_default() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleIntersection<Sum>>();
    let i = TupleIntersection::<Sum>::default();
    assert!(!i.has_result());
}
