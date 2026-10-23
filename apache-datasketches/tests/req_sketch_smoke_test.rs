#[macro_use]
mod req_common;

use apache_datasketches::req::{RankAccuracy, ReqSketch};
use apache_datasketches::SketchError;
use req_common::TestItem;

fn filled<T: TestItem>(n: u32) -> ReqSketch<T> {
    let mut sketch = ReqSketch::<T>::default();
    for i in 1..=n {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

fn construction<T: TestItem>() {
    let d = ReqSketch::<T>::default();
    assert_eq!(d.get_k(), 12);
    assert_eq!(d.rank_accuracy(), RankAccuracy::High);
    assert_eq!(
        ReqSketch::<T>::new(4, RankAccuracy::Low)
            .unwrap()
            .rank_accuracy(),
        RankAccuracy::Low
    );
    assert_eq!(
        ReqSketch::<T>::new(254, RankAccuracy::High)
            .unwrap()
            .get_k(),
        254
    );
    for k in [0, 2, 3, 13, 256, 1024, 1025, 1026, u16::MAX] {
        assert!(
            matches!(
                ReqSketch::<T>::new(k, RankAccuracy::High),
                Err(SketchError::InvalidConfig(_))
            ),
            "k = {k}"
        );
    }
}
for_both_types!(construction_t, construction);

fn exact_mode_queries<T: TestItem>() {
    let sketch = filled::<T>(10);
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 10);
    assert_eq!(sketch.get_num_retained(), 10);
    assert_eq!(sketch.get_min_item().unwrap().to_f64(), 1.0);
    assert_eq!(sketch.get_max_item().unwrap().to_f64(), 10.0);
    assert_eq!(sketch.get_rank(T::from_f64(5.0), true).unwrap(), 0.5);
    assert_eq!(sketch.get_rank(T::from_f64(5.0), false).unwrap(), 0.4);
    assert_eq!(sketch.get_quantile(0.5, true).unwrap().to_f64(), 5.0);
    let split = [T::from_f64(3.0), T::from_f64(7.0)];
    assert_eq!(sketch.get_cdf(&split, true).unwrap(), vec![0.3, 0.7, 1.0]);
    let pmf = sketch.get_pmf(&split, true).unwrap();
    assert_eq!(pmf.len(), 3);
    assert!((pmf.iter().sum::<f64>() - 1.0).abs() < 1e-12);
}
for_both_types!(exact_mode_queries_t, exact_mode_queries);

fn empty_sketch<T: TestItem>() {
    let sketch = ReqSketch::<T>::default();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_min_item(), None);
    assert_eq!(sketch.get_max_item(), None);
    assert!(matches!(
        sketch.get_quantile(0.5, true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_rank(T::from_f64(0.0), true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_cdf(&[T::from_f64(0.0)], true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_rank_lower_bound(0.5, 1),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_rank_upper_bound(0.5, 1),
        Err(SketchError::EmptySketch)
    ));
    assert_eq!(sketch.iter().count(), 0);
}
for_both_types!(empty_sketch_t, empty_sketch);

fn nan_update_is_ignored<T: TestItem>() {
    let mut sketch = ReqSketch::<T>::default();
    sketch.update(T::from_f64(f64::NAN));
    assert!(sketch.is_empty());
}
for_both_types!(nan_update_is_ignored_t, nan_update_is_ignored);

fn merge_combines<T: TestItem>() {
    let mut a = filled::<T>(10);
    let mut b = ReqSketch::<T>::default();
    for i in 11..=20 {
        b.update(T::from_f64(f64::from(i)));
    }
    a.merge(&b).unwrap();
    assert_eq!(a.get_n(), 20);
    assert_eq!(a.get_min_item().unwrap().to_f64(), 1.0);
    assert_eq!(a.get_max_item().unwrap().to_f64(), 20.0);
    assert_eq!(b.get_n(), 10, "merge must not modify its argument");
}
for_both_types!(merge_combines_t, merge_combines);

fn merge_mismatched_accuracy_fails<T: TestItem>() {
    let mut high = ReqSketch::<T>::new(12, RankAccuracy::High).unwrap();
    let mut low = ReqSketch::<T>::new(12, RankAccuracy::Low).unwrap();
    high.update(T::from_f64(1.0));
    low.update(T::from_f64(1.0));
    assert!(matches!(
        high.merge(&low),
        Err(SketchError::InvalidConfig(_))
    ));
    assert!(matches!(
        low.merge(&high),
        Err(SketchError::InvalidConfig(_))
    ));
    assert_eq!(
        high.get_n(),
        1,
        "a rejected merge must not change the sketch"
    );
}
for_both_types!(
    merge_mismatched_accuracy_fails_t,
    merge_mismatched_accuracy_fails
);

fn rank_bounds<T: TestItem>() {
    let sketch = filled::<T>(10_000);
    for std_dev in 1..=3 {
        let lb = sketch.get_rank_lower_bound(0.5, std_dev).unwrap();
        let ub = sketch.get_rank_upper_bound(0.5, std_dev).unwrap();
        assert!(lb <= 0.5 && 0.5 <= ub, "std_dev {std_dev}: [{lb}, {ub}]");
    }
    for bad in [0, 4] {
        assert!(matches!(
            sketch.get_rank_lower_bound(0.5, bad),
            Err(SketchError::InvalidConfig(_))
        ));
        assert!(matches!(
            sketch.get_rank_upper_bound(0.5, bad),
            Err(SketchError::InvalidConfig(_))
        ));
    }
    assert!(matches!(
        sketch.get_rank_lower_bound(1.5, 1),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(rank_bounds_t, rank_bounds);

fn a_priori_rank_error<T: TestItem>() {
    let err = ReqSketch::<T>::a_priori_rank_error(12, 0.5, RankAccuracy::High, 1000).unwrap();
    let expected = (0.0512_f64 / 3.0).sqrt() / 12.0 * 0.5;
    assert!((err - expected).abs() < 1e-9, "{err} vs {expected}");
    assert_eq!(
        ReqSketch::<T>::a_priori_rank_error(12, 0.5, RankAccuracy::High, 10).unwrap(),
        0.0
    );
    assert!(ReqSketch::<T>::a_priori_rank_error(1024, 0.5, RankAccuracy::High, 1000).is_ok());
    for k in [13, 1026] {
        assert!(matches!(
            ReqSketch::<T>::a_priori_rank_error(k, 0.5, RankAccuracy::High, 1000),
            Err(SketchError::InvalidConfig(_))
        ));
    }
    assert!(matches!(
        ReqSketch::<T>::a_priori_rank_error(12, f64::NAN, RankAccuracy::High, 1000),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(a_priori_rank_error_t, a_priori_rank_error);

fn serialize_roundtrip<T: TestItem>() {
    for accuracy in [RankAccuracy::High, RankAccuracy::Low] {
        let mut sketch = ReqSketch::<T>::new(12, accuracy).unwrap();
        for i in 0..10_000 {
            sketch.update(T::from_f64(f64::from(i)));
        }
        let restored = ReqSketch::<T>::deserialize(&sketch.serialize()).unwrap();
        assert_eq!(restored.get_n(), sketch.get_n());
        assert_eq!(restored.get_k(), sketch.get_k());
        assert_eq!(restored.rank_accuracy(), accuracy);
        assert_eq!(restored.get_num_retained(), sketch.get_num_retained());
        assert_eq!(restored.get_min_item(), sketch.get_min_item());
        assert_eq!(restored.get_max_item(), sketch.get_max_item());
        assert_eq!(
            restored.get_quantile(0.5, true).unwrap(),
            sketch.get_quantile(0.5, true).unwrap()
        );
    }
}
for_both_types!(serialize_roundtrip_t, serialize_roundtrip);

fn garbage_bytes_fail<T: TestItem>() {
    assert!(matches!(
        ReqSketch::<T>::deserialize(&[1, 2, 3]),
        Err(SketchError::Deserialization(_))
    ));
}
for_both_types!(garbage_bytes_fail_t, garbage_bytes_fail);

fn iter_covers_every_retained_item<T: TestItem>() {
    let sketch = filled::<T>(10_000);
    let pairs: Vec<(T, u64)> = sketch.iter().collect();
    assert_eq!(pairs.len() as u32, sketch.get_num_retained());
    assert_eq!(pairs.iter().map(|&(_, w)| w).sum::<u64>(), sketch.get_n());
}
for_both_types!(
    iter_covers_every_retained_item_t,
    iter_covers_every_retained_item
);

fn clone_is_independent<T: TestItem>() {
    let mut original = filled::<T>(10);
    let copy = original.clone();
    original.update(T::from_f64(11.0));
    assert_eq!(copy.get_n(), 10);
}
for_both_types!(clone_is_independent_t, clone_is_independent);

fn to_string_summary_is_non_empty<T: TestItem>() {
    assert!(!filled::<T>(10).to_string_summary().is_empty());
}
for_both_types!(
    to_string_summary_is_non_empty_t,
    to_string_summary_is_non_empty
);
