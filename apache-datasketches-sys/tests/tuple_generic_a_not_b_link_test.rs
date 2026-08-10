#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi as sketch_ffi, RawSummaryOps, RustSummary};
use apache_datasketches_sys::tuple_generic_a_not_b::ffi as anb_ffi;
use std::any::Any;

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn sketch(
    keys: std::ops::Range<u64>,
    v: i64,
) -> cxx::UniquePtr<sketch_ffi::TupleGenericSketchShim> {
    let mut s = sketch_ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for key in keys {
        s.pin_mut()
            .update_u64(key, &RustSummary::new(Box::new(Sum(v))));
    }
    s
}

fn value_at(c: &sketch_ffi::CompactTupleGenericSketchShim, i: u32) -> i64 {
    c.entry_summary(i)
        .unwrap()
        .ops()
        .as_any()
        .downcast_ref::<Sum>()
        .unwrap()
        .0
}

fn values(c: &sketch_ffi::CompactTupleGenericSketchShim) -> Vec<i64> {
    (0..c.entry_count()).map(|i| value_at(c, i)).collect()
}

/// Asymmetric on purpose: a - b estimates 500, b - a estimates 0, so a
/// transposed mixed-type overload changes an asserted value.
#[test]
fn all_four_overloads_preserve_operand_order() {
    let a = sketch(0..1000, 1);
    let b = sketch(0..500, 1);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc = anb_ffi::new_tuple_generic_a_not_b();

    assert_eq!(
        calc.compute_sketch_sketch(&a, &b, true).get_estimate(),
        500.0
    );
    assert_eq!(
        calc.compute_sketch_compact(&a, &cb, true).get_estimate(),
        500.0
    );
    assert_eq!(
        calc.compute_compact_sketch(&ca, &b, true).get_estimate(),
        500.0
    );
    assert_eq!(
        calc.compute_compact_compact(&ca, &cb, true).get_estimate(),
        500.0
    );

    // Reversed: every combination must now be empty.
    assert_eq!(calc.compute_sketch_sketch(&b, &a, true).get_estimate(), 0.0);
    assert_eq!(
        calc.compute_sketch_compact(&b, &ca, true).get_estimate(),
        0.0
    );
    assert_eq!(
        calc.compute_compact_sketch(&cb, &a, true).get_estimate(),
        0.0
    );
    assert_eq!(
        calc.compute_compact_compact(&cb, &ca, true).get_estimate(),
        0.0
    );
}

// A-not-B has no policy at all (`tuple_a_not_b.hpp:29-33` -- the class takes
// only <Summary, Allocator>, and `theta_set_difference_base::compute` never
// invokes a callback). The retained entries are COPY-constructed out of `a`
// (`theta_set_difference_base_impl.hpp:72` forwards a const lvalue when, as
// here, `a` arrives as a const reference), which routes through DynSummary's
// copy ctor and so through `rust_summary_clone`.
//
// So the two things worth pinning are: a's value arrives unchanged, and `a`
// still owns its own summary afterwards. Had the entry been moved out instead
// of copied, a's DynSummary would be left disengaged and reading it back would
// throw std::logic_error from DynSummary::get().
#[test]
fn result_copies_operand_a_summaries_and_leaves_a_intact() {
    let a = sketch(0..1, 17);
    let b = sketch(100..101, 3);
    let calc = anb_ffi::new_tuple_generic_a_not_b();
    let result = calc.compute_sketch_sketch(&a, &b, true);
    assert_eq!(result.entry_count(), 1);
    assert_eq!(
        value_at(&result, 0),
        17,
        "a's summary must pass through unchanged"
    );

    let a_after = a.compact(true);
    assert_eq!(a_after.entry_count(), 1);
    assert_eq!(
        value_at(&a_after, 0),
        17,
        "the entry was copied out of a, not moved out of it"
    );
}

// `ordered` must reach upstream rather than being hard-wired. Both operands
// are non-empty and disjoint, so this takes the scan path
// (`theta_set_difference_base_impl.hpp:55-77`) and lands on line 81's
// `a.is_ordered() || ordered`; an update sketch is never ordered, so the flag
// alone decides.
#[test]
fn ordered_flag_reaches_upstream() {
    let a = sketch(0..100, 17);
    let b = sketch(200..300, 3);
    let calc = anb_ffi::new_tuple_generic_a_not_b();

    let unordered = calc.compute_sketch_sketch(&a, &b, false);
    assert!(!unordered.is_ordered(), "ordered=false must be honoured");
    assert_eq!(unordered.get_num_retained(), 100);
    assert!(values(&unordered).iter().all(|&v| v == 17));

    let ordered = calc.compute_sketch_sketch(&a, &b, true);
    assert!(ordered.is_ordered());
    assert_eq!(ordered.get_num_retained(), 100);
    assert!(values(&ordered).iter().all(|&v| v == 17));
}

// An empty `b` short-circuits at `theta_set_difference_base_impl.hpp:40` into
// `CompactSketch(a, ordered)` -- a different code path from the scan above, so
// it gets its own summary-value assertion.
#[test]
fn empty_b_yields_all_of_a_with_its_summaries() {
    let a = sketch(0..3, 17);
    let b = sketch(0..0, 3);
    let calc = anb_ffi::new_tuple_generic_a_not_b();
    let result = calc.compute_sketch_sketch(&a, &b, true);
    assert_eq!(result.get_num_retained(), 3);
    assert_eq!(values(&result), vec![17, 17, 17]);
}

#[test]
fn empty_a_yields_empty() {
    let a = sketch(0..0, 17);
    let b = sketch(0..100, 3);
    let calc = anb_ffi::new_tuple_generic_a_not_b();
    let result = calc.compute_sketch_sketch(&a, &b, true);
    assert!(result.is_empty());
    assert_eq!(result.get_num_retained(), 0);
}

// Identical operands cancel exactly. `get_num_retained() == 0` is asserted
// alongside `is_empty()` because upstream only sets the empty flag when the
// entry list is empty AND theta is still MAX_THETA
// (`theta_set_difference_base_impl.hpp:79`).
#[test]
fn a_not_b_with_identical_operands_is_empty() {
    let a = sketch(0..100, 17);
    let calc = anb_ffi::new_tuple_generic_a_not_b();
    let result = calc.compute_sketch_sketch(&a, &a, true);
    assert_eq!(result.get_num_retained(), 0);
    assert!(result.is_empty());
}
