//! Every row of the spec's error table, for both item types.
//!
//! Not a port of upstream kll_sketch_validation.cpp: that test only builds
//! with -DKLL_VALIDATION, which replaces the random compaction bit with a
//! deterministic one, and the binding builds without it.

#[macro_use]
mod kll_common;

use apache_datasketches::kll::KllSketch;
use apache_datasketches::SketchError;
use kll_common::TestItem;

fn invalid(r: Result<impl std::fmt::Debug, SketchError>) -> bool {
    matches!(r, Err(SketchError::InvalidConfig(_)))
}

fn filled<T: TestItem>() -> KllSketch<T> {
    let mut sketch = KllSketch::<T>::default();
    for i in 0..10 {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

fn k_out_of_range<T: TestItem>() {
    for k in [0, 1, 7] {
        assert!(invalid(KllSketch::<T>::new(k).map(|_| ())));
        assert!(invalid(KllSketch::<T>::normalized_rank_error_for_k(
            k, true
        )));
    }
}
for_both_types!(k_out_of_range_t, k_out_of_range);

fn rank_out_of_range<T: TestItem>() {
    let sketch = filled::<T>();
    for rank in [-0.0001, 1.0001, f64::NAN, f64::INFINITY] {
        assert!(invalid(sketch.get_quantile(rank, true)), "rank {rank}");
        assert!(
            invalid(sketch.sorted_view().get_quantile(rank, false)),
            "rank {rank}"
        );
    }
    // The endpoints are valid.
    assert!(sketch.get_quantile(0.0, true).is_ok());
    assert!(sketch.get_quantile(1.0, true).is_ok());
}
for_both_types!(rank_out_of_range_t, rank_out_of_range);

fn nan_item_in_get_rank<T: TestItem>() {
    let sketch = filled::<T>();
    assert!(invalid(sketch.get_rank(T::from_f64(f64::NAN), true)));
}
for_both_types!(nan_item_in_get_rank_t, nan_item_in_get_rank);

fn bad_split_points<T: TestItem>() {
    let sketch = filled::<T>();
    let x = T::from_f64;
    let cases: [Vec<T>; 4] = [
        vec![x(f64::NAN)],
        vec![x(1.0), x(f64::NAN)],
        vec![x(2.0), x(1.0)],
        vec![x(1.0), x(1.0)],
    ];
    for split in &cases {
        assert!(invalid(sketch.get_pmf(split, true)), "{split:?}");
        assert!(invalid(sketch.get_cdf(split, true)), "{split:?}");
    }
    // No split points at all is valid: one interval holding everything.
    assert_eq!(sketch.get_cdf(&[], true).unwrap(), vec![1.0]);
}
for_both_types!(bad_split_points_t, bad_split_points);

fn empty_wins_over_bad_arguments<T: TestItem>() {
    // Upstream checks emptiness first; so does the binding.
    let sketch = KllSketch::<T>::default();
    assert!(matches!(
        sketch.get_quantile(2.0, true),
        Err(SketchError::EmptySketch)
    ));
}
for_both_types!(
    empty_wins_over_bad_arguments_t,
    empty_wins_over_bad_arguments
);
