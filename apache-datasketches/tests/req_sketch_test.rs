//! Port of upstream `req/test/req_sketch_test.cpp`.
//!
//! Every `TEST_CASE` is ported except these, skipped for the reason given:
//!
//! - `stream serialize-deserialize empty`, `... single item`, `... exact
//!   mode`, `... estimation mode` and `serialize deserialize stream and bytes
//!   equivalence`: they exercise the `std::istream`/`std::ostream` API, which
//!   the binding does not expose.
//! - `type conversion - empty`, `... several levels` and `... custom types`:
//!   the binding has no conversion constructor between item types.
//! - the commented-out `for manual comparison with Java` case.
//!
//! In the byte round-trip cases, upstream's `bytes.size() ==
//! get_serialized_size_bytes()` assertions have no counterpart (the binding
//! exposes no size query); they are replaced by checking that
//! re-serializing the deserialized sketch reproduces the original bytes.
//!
//! Upstream's `req_sketch<int>` in `get_rank equivalence` becomes the two
//! float types; `REQUIRE(x == Approx(v).epsilon(e))` (Catch2 v2, scale 0)
//! becomes `|x - v| <= e * |v|`.

#[macro_use]
mod req_common;

use apache_datasketches::req::{RankAccuracy, ReqSketch};
use apache_datasketches::SketchError;
use req_common::TestItem;

fn sketch_of<T: TestItem>(k: u16, accuracy: RankAccuracy) -> ReqSketch<T> {
    ReqSketch::<T>::new(k, accuracy).unwrap()
}

fn fill<T: TestItem>(s: &mut ReqSketch<T>, range: std::ops::Range<u32>) {
    for i in range {
        s.update(T::from_f64(f64::from(i)));
    }
}

fn near(a: f64, expected: f64, margin: f64) -> bool {
    (a - expected).abs() <= margin
}

fn near_rel(a: f64, expected: f64, epsilon: f64) -> bool {
    (a - expected).abs() <= epsilon * expected.abs()
}

fn empty<T: TestItem>() {
    let s = sketch_of::<T>(12, RankAccuracy::High);
    assert_eq!(s.get_k(), 12);
    assert_eq!(s.rank_accuracy(), RankAccuracy::High);
    assert!(s.is_empty());
    assert!(!s.is_estimation_mode());
    assert_eq!(s.get_n(), 0);
    assert_eq!(s.get_num_retained(), 0);
    assert!(s.get_min_item().is_none());
    assert!(s.get_max_item().is_none());
    assert!(matches!(
        s.get_rank(T::from_f64(0.0), true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        s.get_quantile(0.0, true),
        Err(SketchError::EmptySketch)
    ));
    let split_points = [T::from_f64(0.0)];
    assert!(matches!(
        s.get_cdf(&split_points, true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        s.get_pmf(&split_points, true),
        Err(SketchError::EmptySketch)
    ));
}
for_both_types!(empty_t, empty);

fn single_value_lra<T: TestItem>() {
    let mut s = sketch_of::<T>(12, RankAccuracy::Low);
    s.update(T::from_f64(1.0));
    assert_eq!(s.rank_accuracy(), RankAccuracy::Low);
    assert!(!s.is_empty());
    assert!(!s.is_estimation_mode());
    assert_eq!(s.get_n(), 1);
    assert_eq!(s.get_num_retained(), 1);
    assert_eq!(s.get_rank(T::from_f64(1.0), false).unwrap(), 0.0);
    assert_eq!(s.get_rank(T::from_f64(1.0), true).unwrap(), 1.0);
    // Upstream uses 1.1f; any value above 1 gives the same answer.
    assert_eq!(s.get_rank(T::from_f64(2.0), false).unwrap(), 1.0);
    assert_eq!(s.get_rank(T::from_f64(f64::INFINITY), true).unwrap(), 1.0);
    assert_eq!(s.get_quantile(0.0, false).unwrap().to_f64(), 1.0);
    assert_eq!(s.get_quantile(0.5, false).unwrap().to_f64(), 1.0);
    assert_eq!(s.get_quantile(1.0, false).unwrap().to_f64(), 1.0);

    let mut count = 0;
    for (_, weight) in s.iter() {
        assert_eq!(weight, 1);
        count += 1;
    }
    assert_eq!(count, 1);

    let (item, _) = s.iter().next().unwrap();
    assert_eq!(item.to_f64(), 1.0);
}
for_both_types!(single_value_lra_t, single_value_lra);

fn repeated_values<T: TestItem>() {
    let mut s = sketch_of::<T>(12, RankAccuracy::High);
    for v in [1.0, 1.0, 1.0, 2.0, 2.0, 2.0] {
        s.update(T::from_f64(v));
    }
    assert!(!s.is_empty());
    assert!(!s.is_estimation_mode());
    assert_eq!(s.get_n(), 6);
    assert_eq!(s.get_num_retained(), 6);
    assert_eq!(s.get_rank(T::from_f64(1.0), false).unwrap(), 0.0);
    assert_eq!(s.get_rank(T::from_f64(1.0), true).unwrap(), 0.5);
    assert_eq!(s.get_rank(T::from_f64(2.0), false).unwrap(), 0.5);
    assert_eq!(s.get_rank(T::from_f64(2.0), true).unwrap(), 1.0);
}
for_both_types!(repeated_values_t, repeated_values);

fn exact_mode<T: TestItem>() {
    let mut s = sketch_of::<T>(12, RankAccuracy::High);
    fill(&mut s, 1..11);
    assert!(!s.is_empty());
    assert!(!s.is_estimation_mode());
    assert_eq!(s.get_n(), 10);
    assert_eq!(s.get_num_retained(), 10);

    let x = T::from_f64;
    let rank = |v: f64, inclusive: bool| s.get_rank(x(v), inclusive).unwrap();
    let quantile = |r: f64, inclusive: bool| s.get_quantile(r, inclusive).unwrap().to_f64();

    // exclusive
    assert_eq!(rank(1.0, false), 0.0);
    assert_eq!(rank(2.0, false), 0.1);
    assert_eq!(rank(6.0, false), 0.5);
    assert_eq!(rank(9.0, false), 0.8);
    assert_eq!(rank(10.0, false), 0.9);

    // inclusive
    assert_eq!(rank(1.0, true), 0.1);
    assert_eq!(rank(2.0, true), 0.2);
    assert_eq!(rank(5.0, true), 0.5);
    assert_eq!(rank(9.0, true), 0.9);
    assert_eq!(rank(10.0, true), 1.0);

    // exclusive
    assert_eq!(quantile(0.0, false), 1.0);
    assert_eq!(quantile(0.1, false), 2.0);
    assert_eq!(quantile(0.5, false), 6.0);
    assert_eq!(quantile(0.9, false), 10.0);
    assert_eq!(quantile(1.0, false), 10.0);

    // inclusive
    assert_eq!(quantile(0.0, true), 1.0);
    assert_eq!(quantile(0.1, true), 1.0);
    assert_eq!(quantile(0.5, true), 5.0);
    assert_eq!(quantile(0.9, true), 9.0);
    assert_eq!(quantile(1.0, true), 10.0);

    let splits = [x(2.0), x(6.0), x(9.0)];
    let cdf = s.get_cdf(&splits, false).unwrap();
    assert_eq!(cdf[0], 0.1);
    assert_eq!(cdf[1], 0.5);
    assert_eq!(cdf[2], 0.8);
    assert_eq!(cdf[3], 1.0);
    let pmf = s.get_pmf(&splits, false).unwrap();
    assert!(near(pmf[0], 0.1, 1e-8));
    assert!(near(pmf[1], 0.4, 1e-8));
    assert!(near(pmf[2], 0.3, 1e-8));
    assert!(near(pmf[3], 0.2, 1e-8));

    assert_eq!(s.get_rank_lower_bound(0.5, 1).unwrap(), 0.5);
    assert_eq!(s.get_rank_upper_bound(0.5, 1).unwrap(), 0.5);
}
for_both_types!(exact_mode_t, exact_mode);

fn estimation_mode<T: TestItem>() {
    let mut s = sketch_of::<T>(12, RankAccuracy::High);
    let n: u32 = 100_000;
    fill(&mut s, 0..n);
    assert!(!s.is_empty());
    assert!(s.is_estimation_mode());
    assert_eq!(s.get_n(), u64::from(n));
    assert!(s.get_num_retained() < n);
    // Ranks of the extremes are exact whatever the compaction did; the
    // mid-range ones are asserted within the margins upstream gives.
    assert_eq!(s.get_rank(T::from_f64(0.0), false).unwrap(), 0.0);
    assert_eq!(s.get_rank(T::from_f64(f64::from(n)), false).unwrap(), 1.0);
    let mid = s.get_rank(T::from_f64(f64::from(n) / 2.0), false).unwrap();
    assert!(near(mid, 0.5, 0.01), "{mid}");
    let top = s.get_rank(T::from_f64(f64::from(n) - 1.0), false).unwrap();
    assert!(near(top, 1.0, 0.01), "{top}");
    assert_eq!(s.get_min_item().unwrap().to_f64(), 0.0);
    assert_eq!(s.get_max_item().unwrap().to_f64(), f64::from(n - 1));
    assert!(s.get_rank_lower_bound(0.5, 1).unwrap() < 0.5);
    assert!(s.get_rank_upper_bound(0.5, 1).unwrap() > 0.5);

    let mut count = 0;
    for (_, weight) in s.iter() {
        assert!(weight >= 1);
        count += 1;
    }
    assert_eq!(count, s.get_num_retained());
}
for_both_types!(estimation_mode_t, estimation_mode);

/// The checks upstream repeats after every byte round trip.
fn assert_same_sketch<T: TestItem>(a: &ReqSketch<T>, b: &ReqSketch<T>) {
    assert_eq!(b.is_empty(), a.is_empty());
    assert_eq!(b.is_estimation_mode(), a.is_estimation_mode());
    assert_eq!(b.get_num_retained(), a.get_num_retained());
    assert_eq!(b.get_n(), a.get_n());
    assert_eq!(b.get_min_item(), a.get_min_item());
    assert_eq!(b.get_max_item(), a.get_max_item());
}

fn byte_serde_empty<T: TestItem>() {
    let s = sketch_of::<T>(12, RankAccuracy::High);
    let bytes = s.serialize();
    let s2 = ReqSketch::<T>::deserialize(&bytes).unwrap();
    assert_eq!(s2.serialize(), bytes);
    assert_same_sketch(&s, &s2);
    assert!(s2.get_min_item().is_none());
    assert!(s2.get_max_item().is_none());
}
for_both_types!(byte_serde_empty_t, byte_serde_empty);

fn byte_serde_single_item<T: TestItem>() {
    let mut s = sketch_of::<T>(12, RankAccuracy::High);
    s.update(T::from_f64(1.0));
    let bytes = s.serialize();
    let s2 = ReqSketch::<T>::deserialize(&bytes).unwrap();
    assert_eq!(s2.serialize(), bytes);
    assert_same_sketch(&s, &s2);
}
for_both_types!(byte_serde_single_item_t, byte_serde_single_item);

fn byte_serde_exact_mode<T: TestItem>() {
    let mut s = sketch_of::<T>(12, RankAccuracy::High);
    fill(&mut s, 0..50);
    assert!(!s.is_estimation_mode());
    let bytes = s.serialize();
    let s2 = ReqSketch::<T>::deserialize(&bytes).unwrap();
    assert_eq!(s2.serialize(), bytes);
    assert_same_sketch(&s, &s2);
}
for_both_types!(byte_serde_exact_mode_t, byte_serde_exact_mode);

fn byte_serde_estimation_mode<T: TestItem>() {
    let mut s = sketch_of::<T>(12, RankAccuracy::High);
    fill(&mut s, 0..100_000);
    assert!(s.is_estimation_mode());
    let bytes = s.serialize();
    let s2 = ReqSketch::<T>::deserialize(&bytes).unwrap();
    assert_eq!(s2.serialize(), bytes);
    assert_same_sketch(&s, &s2);
}
for_both_types!(byte_serde_estimation_mode_t, byte_serde_estimation_mode);

fn merge_into_empty<T: TestItem>() {
    let mut s1 = sketch_of::<T>(40, RankAccuracy::High);
    let mut s2 = sketch_of::<T>(40, RankAccuracy::High);
    fill(&mut s2, 0..1000);
    s1.merge(&s2).unwrap();
    assert_eq!(s1.get_min_item().unwrap().to_f64(), 0.0);
    assert_eq!(s1.get_max_item().unwrap().to_f64(), 999.0);
    // The quantile answers depend on the random compaction; the tolerances
    // are upstream's.
    let q = |r: f64| s1.get_quantile(r, true).unwrap().to_f64();
    assert!(near_rel(q(0.25), 250.0, 0.01), "{}", q(0.25));
    assert!(near_rel(q(0.5), 500.0, 0.01), "{}", q(0.5));
    assert!(near_rel(q(0.75), 750.0, 0.01), "{}", q(0.75));
    let r = s1.get_rank(T::from_f64(500.0), true).unwrap();
    assert!(near(r, 0.5, 0.01), "{r}");
}
for_both_types!(merge_into_empty_t, merge_into_empty);

fn merge<T: TestItem>() {
    let mut s1 = sketch_of::<T>(100, RankAccuracy::High);
    fill(&mut s1, 0..1000);
    let mut s2 = sketch_of::<T>(100, RankAccuracy::High);
    fill(&mut s2, 1000..2000);
    s1.merge(&s2).unwrap();
    assert_eq!(s1.get_min_item().unwrap().to_f64(), 0.0);
    assert_eq!(s1.get_max_item().unwrap().to_f64(), 1999.0);
    // Random compaction: upstream's tolerances.
    let q = |r: f64| s1.get_quantile(r, true).unwrap().to_f64();
    assert!(near_rel(q(0.25), 500.0, 0.01), "{}", q(0.25));
    assert!(near_rel(q(0.5), 1000.0, 0.01), "{}", q(0.5));
    assert!(near_rel(q(0.75), 1500.0, 0.01), "{}", q(0.75));
    let r = s1.get_rank(T::from_f64(1000.0), true).unwrap();
    assert!(near(r, 0.5, 0.01), "{r}");
}
for_both_types!(merge_t, merge);

fn merge_multiple<T: TestItem>() {
    let mut parts = Vec::new();
    for range in [0..40, 40..80, 80..120] {
        let mut s = sketch_of::<T>(12, RankAccuracy::High);
        fill(&mut s, range);
        parts.push(s);
    }
    let mut s = sketch_of::<T>(12, RankAccuracy::High);
    for part in &parts {
        s.merge(part).unwrap();
    }
    assert_eq!(s.get_min_item().unwrap().to_f64(), 0.0);
    assert_eq!(s.get_max_item().unwrap().to_f64(), 119.0);
    let q = s.get_quantile(0.5, true).unwrap().to_f64();
    assert!(near_rel(q, 60.0, 0.02), "{q}");
    let r = s.get_rank(T::from_f64(60.0), true).unwrap();
    assert!(near(r, 0.5, 0.01), "{r}");
}
for_both_types!(merge_multiple_t, merge_multiple);

fn merge_incompatible_accuracy<T: TestItem>() {
    let mut s1 = sketch_of::<T>(12, RankAccuracy::High);
    s1.update(T::from_f64(1.0));
    let mut s2 = sketch_of::<T>(12, RankAccuracy::Low);
    s2.update(T::from_f64(1.0));
    assert!(matches!(s1.merge(&s2), Err(SketchError::InvalidConfig(_))));
    // The failed merge left the receiver untouched.
    assert_eq!(s1.get_n(), 1);
}
for_both_types!(merge_incompatible_accuracy_t, merge_incompatible_accuracy);

fn get_rank_equivalence<T: TestItem>() {
    let mut s = sketch_of::<T>(12, RankAccuracy::High);
    let n = 1000;
    fill(&mut s, 0..n);
    assert_eq!(s.get_n(), u64::from(n));
    let view = s.sorted_view();
    for i in 0..n {
        let x = T::from_f64(f64::from(i));
        assert_eq!(
            s.get_rank(x, true).unwrap(),
            view.get_rank(x, true).unwrap()
        );
    }
}
for_both_types!(get_rank_equivalence_t, get_rank_equivalence);
