//! Every row of the spec's error table, for both item types.
//!
//! Not a port of upstream's tests: upstream's REQ code trusts its arguments,
//! so these pin the checks the binding adds in front of it.

#[macro_use]
mod req_common;

use apache_datasketches::req::{RankAccuracy, ReqSketch};
use apache_datasketches::SketchError;
use req_common::TestItem;

fn invalid(r: Result<impl std::fmt::Debug, SketchError>) -> bool {
    matches!(r, Err(SketchError::InvalidConfig(_)))
}

fn filled<T: TestItem>() -> ReqSketch<T> {
    let mut sketch = ReqSketch::<T>::default();
    for i in 0..10 {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

const BAD_K: [u16; 8] = [0, 2, 3, 5, 13, 1023, 1025, 2048];

fn k_in_new<T: TestItem>() {
    for k in BAD_K {
        for accuracy in [RankAccuracy::High, RankAccuracy::Low] {
            assert!(
                invalid(ReqSketch::<T>::new(k, accuracy).map(|_| ())),
                "k {k}"
            );
        }
    }
    // Even and within upstream's documented range, but beyond what its
    // constructor can hold (it truncates k to 8 bits).
    for k in [256, 1024] {
        assert!(
            invalid(ReqSketch::<T>::new(k, RankAccuracy::High).map(|_| ())),
            "k {k}"
        );
    }
    for k in [4, 6, 12, 254] {
        let s = ReqSketch::<T>::new(k, RankAccuracy::High).unwrap();
        assert_eq!(s.get_k(), k);
    }
}
for_both_types!(k_in_new_t, k_in_new);

fn k_in_a_priori_rank_error<T: TestItem>() {
    for k in BAD_K {
        assert!(
            invalid(ReqSketch::<T>::a_priori_rank_error(
                k,
                0.5,
                RankAccuracy::High,
                1000
            )),
            "k {k}"
        );
    }
    for k in [4, 6, 12, 254, 256, 1024] {
        let e = ReqSketch::<T>::a_priori_rank_error(k, 0.5, RankAccuracy::High, 1000).unwrap();
        assert!(e.is_finite() && e >= 0.0, "k {k}: {e}");
    }
}
for_both_types!(k_in_a_priori_rank_error_t, k_in_a_priori_rank_error);

const BAD_RANKS: [f64; 4] = [-0.0001, 1.0001, f64::NAN, f64::INFINITY];

fn rank_out_of_range<T: TestItem>() {
    let sketch = filled::<T>();
    let view = sketch.sorted_view();
    for rank in BAD_RANKS {
        assert!(invalid(sketch.get_quantile(rank, true)), "rank {rank}");
        assert!(invalid(view.get_quantile(rank, false)), "rank {rank}");
        assert!(invalid(sketch.get_rank_lower_bound(rank, 1)), "rank {rank}");
        assert!(invalid(sketch.get_rank_upper_bound(rank, 1)), "rank {rank}");
        assert!(
            invalid(ReqSketch::<T>::a_priori_rank_error(
                12,
                rank,
                RankAccuracy::High,
                1000
            )),
            "rank {rank}"
        );
    }
    // The endpoints are valid.
    assert!(sketch.get_quantile(0.0, true).is_ok());
    assert!(sketch.get_quantile(1.0, true).is_ok());
    assert!(view.get_quantile(0.0, true).is_ok());
    assert!(view.get_quantile(1.0, true).is_ok());
    assert!(ReqSketch::<T>::a_priori_rank_error(12, 0.0, RankAccuracy::Low, 1000).is_ok());
    assert!(ReqSketch::<T>::a_priori_rank_error(12, 1.0, RankAccuracy::Low, 1000).is_ok());
}
for_both_types!(rank_out_of_range_t, rank_out_of_range);

fn num_std_dev_out_of_range<T: TestItem>() {
    let sketch = filled::<T>();
    for num_std_dev in [0, 4, 255] {
        assert!(
            invalid(sketch.get_rank_lower_bound(0.5, num_std_dev)),
            "{num_std_dev}"
        );
        assert!(
            invalid(sketch.get_rank_upper_bound(0.5, num_std_dev)),
            "{num_std_dev}"
        );
    }
    for num_std_dev in [1, 2, 3] {
        assert!(sketch.get_rank_lower_bound(0.5, num_std_dev).is_ok());
        assert!(sketch.get_rank_upper_bound(0.5, num_std_dev).is_ok());
    }
}
for_both_types!(num_std_dev_out_of_range_t, num_std_dev_out_of_range);

fn nan_item_in_get_rank<T: TestItem>() {
    let sketch = filled::<T>();
    assert!(invalid(sketch.get_rank(T::from_f64(f64::NAN), true)));
    assert!(invalid(
        sketch.sorted_view().get_rank(T::from_f64(f64::NAN), true)
    ));
}
for_both_types!(nan_item_in_get_rank_t, nan_item_in_get_rank);

fn bad_split_points<T: TestItem>() {
    let sketch = filled::<T>();
    let view = sketch.sorted_view();
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
        assert!(invalid(view.get_pmf(split, true)), "{split:?}");
        assert!(invalid(view.get_cdf(split, true)), "{split:?}");
    }
    // No split points at all is valid: one interval holding everything.
    assert_eq!(sketch.get_cdf(&[], true).unwrap(), vec![1.0]);
    assert_eq!(view.get_cdf(&[], true).unwrap(), vec![1.0]);
}
for_both_types!(bad_split_points_t, bad_split_points);

fn merge_with_differing_accuracy<T: TestItem>() {
    let mut high = ReqSketch::<T>::new(12, RankAccuracy::High).unwrap();
    let mut low = ReqSketch::<T>::new(12, RankAccuracy::Low).unwrap();
    for i in 0..10 {
        high.update(T::from_f64(f64::from(i)));
        low.update(T::from_f64(f64::from(i)));
    }
    assert!(invalid(high.merge(&low)));
    assert!(invalid(low.merge(&high)));
    assert_eq!(high.get_n(), 10);
    assert_eq!(low.get_n(), 10);
    // Merging an empty sketch of the other accuracy is rejected as well:
    // upstream checks accuracy before emptiness.
    let empty_low = ReqSketch::<T>::new(12, RankAccuracy::Low).unwrap();
    assert!(invalid(high.merge(&empty_low)));
}
for_both_types!(
    merge_with_differing_accuracy_t,
    merge_with_differing_accuracy
);

fn empty_wins_over_bad_arguments<T: TestItem>() {
    // Upstream checks emptiness first; so does the binding.
    let sketch = ReqSketch::<T>::default();
    assert!(matches!(
        sketch.get_quantile(2.0, true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_rank_lower_bound(2.0, 0),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_rank(T::from_f64(f64::NAN), true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_cdf(&[T::from_f64(f64::NAN)], true),
        Err(SketchError::EmptySketch)
    ));
}
for_both_types!(
    empty_wins_over_bad_arguments_t,
    empty_wins_over_bad_arguments
);
