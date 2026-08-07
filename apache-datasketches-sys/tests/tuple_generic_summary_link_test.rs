#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{RawSummaryOps, RustSummary};
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

static CLONES: AtomicUsize = AtomicUsize::new(0);
static DROPS: AtomicUsize = AtomicUsize::new(0);

/// A summary that sums on union and takes the minimum on intersection, so the
/// two combine trampolines are distinguishable if they are ever cross-wired.
#[derive(Debug)]
struct TestSummary(i64);

impl Drop for TestSummary {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

impl RawSummaryOps for TestSummary {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        CLONES.fetch_add(1, Ordering::SeqCst);
        Box::new(TestSummary(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        let other = other.as_any().downcast_ref::<TestSummary>().unwrap();
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        let other = other.as_any().downcast_ref::<TestSummary>().unwrap();
        self.0 = self.0.min(other.0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn summary(v: i64) -> RustSummary {
    RustSummary::new(Box::new(TestSummary(v)))
}

fn value_of(s: &RustSummary) -> i64 {
    s.ops().as_any().downcast_ref::<TestSummary>().unwrap().0
}

#[test]
fn clone_produces_an_independent_copy() {
    let a = summary(7);
    let b = apache_datasketches_sys::tuple_generic::clone_for_test(&a);
    assert_eq!(value_of(&b), 7);
    // Mutating the clone must not affect the original.
    let mut b = b;
    apache_datasketches_sys::tuple_generic::union_for_test(&mut b, &summary(3));
    assert_eq!(value_of(&b), 10);
    assert_eq!(value_of(&a), 7);
}

#[test]
fn union_and_intersection_are_distinct() {
    let mut u = summary(4);
    apache_datasketches_sys::tuple_generic::union_for_test(&mut u, &summary(6));
    assert_eq!(value_of(&u), 10, "union should sum");

    let mut i = summary(4);
    apache_datasketches_sys::tuple_generic::intersection_for_test(&mut i, &summary(6));
    assert_eq!(value_of(&i), 4, "intersection should take the minimum");
}

#[test]
fn clones_and_drops_balance() {
    CLONES.store(0, Ordering::SeqCst);
    DROPS.store(0, Ordering::SeqCst);
    {
        let a = summary(1);
        let _b = apache_datasketches_sys::tuple_generic::clone_for_test(&a);
        let _c = apache_datasketches_sys::tuple_generic::clone_for_test(&a);
    }
    assert_eq!(CLONES.load(Ordering::SeqCst), 2);
    assert_eq!(DROPS.load(Ordering::SeqCst), 3, "original plus two clones");
}
