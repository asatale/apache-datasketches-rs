use apache_datasketches::tuple::generic::{
    TupleAnotB, TupleSketch, TupleSketchBuilder, TupleSummary,
};

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

// The operands are asymmetric on purpose (a - b = 500, b - a = 0), so a
// transposed overload changes an asserted estimate. They also carry DISTINCT
// summary values so each forward result can be checked by VALUE: a-not-b
// copies operand a's summaries through `DynSummary`'s copy constructor and
// the clone trampoline, and every retained entry must therefore read Sum(17),
// never b's Sum(3). Two of the four forward calls have a COMPACT operand a,
// which is a distinct upstream iteration path.
#[test]
fn all_four_combinations_preserve_operand_order() {
    let a = sketch(0..1000, 17);
    let b = sketch(0..500, 3);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc: TupleAnotB<Sum> = TupleAnotB::new();

    for (label, result) in [
        ("sketch/sketch", calc.compute(&a, &b, true)),
        ("sketch/compact", calc.compute(&a, &cb, true)),
        ("compact/sketch", calc.compute(&ca, &b, true)),
        ("compact/compact", calc.compute(&ca, &cb, true)),
    ] {
        assert_eq!(result.get_estimate(), 500.0, "{label}");
        let values: Vec<Sum> = result.entries().map(|(_, s)| s).collect();
        assert_eq!(values.len(), 500, "{label}");
        assert!(
            values.iter().all(|s| *s == Sum(17)),
            "{label}: every retained summary must be a copy of operand a's \
             Sum(17); saw {values:?}"
        );
    }

    assert_eq!(calc.compute(&b, &a, true).get_estimate(), 0.0);
    assert_eq!(calc.compute(&b, &ca, true).get_estimate(), 0.0);
    assert_eq!(calc.compute(&cb, &a, true).get_estimate(), 0.0);
    assert_eq!(calc.compute(&cb, &ca, true).get_estimate(), 0.0);
}

// A-not-b has no combine callback -- there is no policy on the C++ type at
// all -- so what there is to pin is that a's summary arrives unchanged AND
// that `a` still owns its own copy afterwards. The second assertion is what
// distinguishes a deep copy from a move out of `a`'s table.
#[test]
fn result_copies_operand_a_summaries_and_leaves_a_intact() {
    let calc: TupleAnotB<Sum> = TupleAnotB::new();
    let a = sketch(0..1, 17);
    let result = calc.compute(&a, &sketch(100..101, 3), true);
    let entries: Vec<(u64, Sum)> = result.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, Sum(17));

    let a_after: Vec<(u64, Sum)> = a.compact(true).entries().collect();
    assert_eq!(a_after.len(), 1);
    assert_eq!(
        a_after[0].1,
        Sum(17),
        "the entry was copied out of a, not moved out of it"
    );
}

// Disjoint, both non-empty: the result is all of `a`, and `ordered` decides
// the result's ordering because an update sketch is never ordered.
#[test]
fn disjoint_operands_yield_all_of_a_and_honour_ordered() {
    let a = sketch(0..100, 17);
    let b = sketch(200..300, 3);
    let calc: TupleAnotB<Sum> = TupleAnotB::new();

    let unordered = calc.compute(&a, &b, false);
    assert!(!unordered.is_ordered(), "ordered=false must be honoured");
    assert_eq!(unordered.get_num_retained(), 100);
    assert!(unordered.entries().all(|(_, s)| s == Sum(17)));

    let ordered = calc.compute(&a, &b, true);
    assert!(ordered.is_ordered());
    assert_eq!(ordered.get_num_retained(), 100);
    let hashes: Vec<u64> = ordered.entries().map(|(h, _)| h).collect();
    assert!(hashes.windows(2).all(|w| w[0] <= w[1]));
    assert!(ordered.entries().all(|(_, s)| s == Sum(17)));
}

#[test]
fn empty_b_yields_all_of_a_with_its_summaries() {
    let calc: TupleAnotB<Sum> = TupleAnotB::new();
    let result = calc.compute(&sketch(0..3, 17), &sketch(0..0, 3), true);
    assert_eq!(result.get_num_retained(), 3);
    let values: Vec<Sum> = result.entries().map(|(_, s)| s).collect();
    assert_eq!(values, vec![Sum(17), Sum(17), Sum(17)]);
}

#[test]
fn empty_a_yields_empty() {
    let calc: TupleAnotB<Sum> = TupleAnotB::new();
    let result = calc.compute(&sketch(0..0, 17), &sketch(0..100, 3), true);
    assert!(result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
}

#[test]
fn a_not_b_self_is_empty() {
    let a = sketch(0..100, 1);
    let calc: TupleAnotB<Sum> = TupleAnotB::new();
    let result = calc.compute(&a, &a, true);
    assert_eq!(result.get_num_retained(), 0);
    assert!(result.is_empty());
}

#[test]
fn a_not_b_is_send_and_default() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleAnotB<Sum>>();
    let _ = TupleAnotB::<Sum>::default();
}
