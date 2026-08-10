use apache_datasketches::tuple::generic::{
    TupleSketch, TupleSketchBuilder, TupleSummary, TupleUnion, TupleUnionBuilder,
};
use apache_datasketches::tuple::ResizeFactor;

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
fn union_half_overlap() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().lg_k(12).build().unwrap();
    u.update(&sketch(0..1000, 1));
    u.update(&sketch(500..1500, 1));
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);
}

// The two operands carry DISTINCT values so the compact operand's summaries
// are observable, not just its keys. Hash order does not let a key be mapped
// back from an entry, so the assertion is on the multiset of values:
//
//   50 x Sum(42) -- the overlap, union_combine(10, 32)
//   50 x Sum(10) -- keys 0..50, only in the sketch operand
//   50 x Sum(32) -- keys 100..150, only in the COMPACT operand
//
// The 32-group is the part that proves the compact operand's summaries
// crossed the boundary at all. Honest limitation: on the overlap, a dead
// combine and a cross-wire to `Sum::intersection_combine` (min) both read 10,
// so this catches both but does not distinguish them.
#[test]
fn union_accepts_both_input_types() {
    let a = sketch(0..100, 10);
    let b = sketch(50..150, 32).compact(true);
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&a);
    u.update(&b);
    let result = u.get_result(true);
    assert_eq!(result.get_estimate(), 150.0);

    let mut got: Vec<i64> = result.entries().map(|(_, s)| s.0).collect();
    assert_eq!(got.len(), 150);
    got.sort_unstable();
    let mut expected: Vec<i64> = [vec![10i64; 50], vec![32; 50], vec![42; 50]].concat();
    expected.sort_unstable();
    assert_eq!(
        got, expected,
        "expected 50 untouched sketch-operand summaries (10), 50 untouched \
         compact-operand summaries (32) and 50 union-combined ones (42)"
    );
}

#[test]
fn union_sums_summaries_on_collision() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&sketch(7..8, 10));
    u.update(&sketch(7..8, 32));
    let entries: Vec<(u64, Sum)> = u.get_result(true).entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, Sum(42));
}

#[test]
fn union_reset_empties_result() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&sketch(0..100, 1));
    assert!(!u.get_result(true).is_empty());
    u.reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn invalid_config_is_err() {
    assert!(TupleUnionBuilder::<Sum>::new().lg_k(4).build().is_err());
    assert!(TupleUnionBuilder::<Sum>::new().p(1.5).build().is_err());
}

#[test]
fn union_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleUnion<Sum>>();
}

// `DynUnionPolicy::operator()` hands the stored summary to the trampoline as
// `RustSummary&` and the incoming one as `const RustSummary&`. If those ever
// aliased the same underlying `rust::Box` (e.g. a shallow-copying
// `DynSummary` copy constructor), unioning a sketch into itself would create
// a `&mut` and a `&` to one Rust value: undefined behaviour. `DynSummary`
// deep-copies today, so this must be sound, and every key's summary must be
// the union-combine of its value with itself (doubled, for `Sum`).
#[test]
fn union_self_union_deep_copies_summaries() {
    let a = sketch(0..4, 7);
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().lg_k(12).build().unwrap();
    u.update(&a);
    u.update(&a);
    let mut entries: Vec<(u64, Sum)> = u.get_result(true).entries().collect();
    entries.sort_by_key(|(k, _)| *k);
    assert_eq!(entries.len(), 4);
    for (_, summary) in entries {
        assert_eq!(summary, Sum(14));
    }
}

#[test]
fn union_empty_get_result_is_empty() {
    let u: TupleUnion<Sum> = TupleUnionBuilder::new().lg_k(12).build().unwrap();
    let result = u.get_result(true);
    assert!(result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
    assert_eq!(result.get_estimate(), 0.0);
}

#[test]
fn union_get_result_unordered_matches_ordered() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().lg_k(12).build().unwrap();
    u.update(&sketch(0..100, 1));
    u.update(&sketch(50..150, 1));

    let ordered = u.get_result(true);
    let unordered = u.get_result(false);

    assert_eq!(ordered.get_num_retained(), unordered.get_num_retained());
    assert!(ordered.is_ordered());
    assert!(!unordered.is_ordered());
}

#[test]
fn union_resize_factor_is_honored_in_a_real_union() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new()
        .lg_k(12)
        .resize_factor(ResizeFactor::X1)
        .build()
        .unwrap();
    u.update(&sketch(0..1000, 1));
    u.update(&sketch(500..1500, 1));
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);
}
