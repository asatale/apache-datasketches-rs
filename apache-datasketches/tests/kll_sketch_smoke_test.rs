#[macro_use]
mod kll_common;

use apache_datasketches::kll::KllSketch;
use apache_datasketches::SketchError;
use kll_common::TestItem;

fn filled<T: TestItem>(k: u16, n: u32) -> KllSketch<T> {
    let mut sketch = KllSketch::<T>::new(k).unwrap();
    for i in 1..=n {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

fn construction<T: TestItem>() {
    assert_eq!(KllSketch::<T>::default().get_k(), 200);
    assert_eq!(KllSketch::<T>::new(8).unwrap().get_k(), 8);
    assert_eq!(KllSketch::<T>::new(u16::MAX).unwrap().get_k(), u16::MAX);
    assert!(matches!(
        KllSketch::<T>::new(7),
        Err(SketchError::InvalidConfig(_))
    ));
    assert!(matches!(
        KllSketch::<T>::new(0),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(construction_t, construction);

fn exact_mode_queries<T: TestItem>() {
    let sketch = filled::<T>(200, 100);
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 100);
    assert_eq!(sketch.get_num_retained(), 100);
    assert_eq!(sketch.get_min_item().unwrap().to_f64(), 1.0);
    assert_eq!(sketch.get_max_item().unwrap().to_f64(), 100.0);
    assert_eq!(sketch.get_quantile(0.5, true).unwrap().to_f64(), 50.0);
    assert_eq!(sketch.get_rank(T::from_f64(50.0), true).unwrap(), 0.5);
    assert_eq!(sketch.get_rank(T::from_f64(50.0), false).unwrap(), 0.49);
    let split = [T::from_f64(25.0), T::from_f64(75.0)];
    assert_eq!(sketch.get_cdf(&split, true).unwrap(), vec![0.25, 0.75, 1.0]);
    assert_eq!(sketch.get_pmf(&split, true).unwrap(), vec![0.25, 0.5, 0.25]);
}
for_both_types!(exact_mode_queries_t, exact_mode_queries);

fn empty_sketch<T: TestItem>() {
    let sketch = KllSketch::<T>::default();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_n(), 0);
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
    let split = [T::from_f64(0.0)];
    assert!(matches!(
        sketch.get_pmf(&split, true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_cdf(&split, true),
        Err(SketchError::EmptySketch)
    ));
    assert_eq!(sketch.iter().count(), 0);
}
for_both_types!(empty_sketch_t, empty_sketch);

fn nan_update_is_ignored<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(T::from_f64(f64::NAN));
    assert!(sketch.is_empty());
}
for_both_types!(nan_update_is_ignored_t, nan_update_is_ignored);

fn merge_combines<T: TestItem>() {
    let mut a = filled::<T>(200, 100);
    let mut b = KllSketch::<T>::default();
    for i in 101..=200 {
        b.update(T::from_f64(f64::from(i)));
    }
    a.merge(&b);
    assert_eq!(a.get_n(), 200);
    assert_eq!(a.get_min_item().unwrap().to_f64(), 1.0);
    assert_eq!(a.get_max_item().unwrap().to_f64(), 200.0);
    assert_eq!(b.get_n(), 100, "merge must not modify its argument");
}
for_both_types!(merge_combines_t, merge_combines);

fn serialize_roundtrip<T: TestItem>() {
    let sketch = filled::<T>(200, 1000);
    let restored = KllSketch::<T>::deserialize(&sketch.serialize()).unwrap();
    assert_eq!(restored.get_n(), sketch.get_n());
    assert_eq!(restored.get_k(), sketch.get_k());
    assert_eq!(restored.get_num_retained(), sketch.get_num_retained());
    assert_eq!(restored.get_min_item(), sketch.get_min_item());
    assert_eq!(restored.get_max_item(), sketch.get_max_item());
    // Same retained items, so even estimation-mode answers are identical.
    assert_eq!(
        restored.get_quantile(0.5, true).unwrap(),
        sketch.get_quantile(0.5, true).unwrap()
    );
}
for_both_types!(serialize_roundtrip_t, serialize_roundtrip);

fn garbage_bytes_fail<T: TestItem>() {
    assert!(matches!(
        KllSketch::<T>::deserialize(&[1, 2, 3]),
        Err(SketchError::Deserialization(_))
    ));
}
for_both_types!(garbage_bytes_fail_t, garbage_bytes_fail);

fn iter_covers_every_retained_item<T: TestItem>() {
    let sketch = filled::<T>(200, 10_000);
    let pairs: Vec<(T, u64)> = sketch.iter().collect();
    assert_eq!(pairs.len() as u32, sketch.get_num_retained());
    assert_eq!(pairs.iter().map(|&(_, w)| w).sum::<u64>(), sketch.get_n());
}
for_both_types!(
    iter_covers_every_retained_item_t,
    iter_covers_every_retained_item
);

fn clone_is_independent<T: TestItem>() {
    let mut original = filled::<T>(200, 10);
    let copy = original.clone();
    original.update(T::from_f64(11.0));
    assert_eq!(original.get_n(), 11);
    assert_eq!(copy.get_n(), 10);
}
for_both_types!(clone_is_independent_t, clone_is_independent);

fn rank_error<T: TestItem>() {
    let sketch = KllSketch::<T>::default();
    assert_eq!(
        KllSketch::<T>::normalized_rank_error_for_k(200, false).unwrap(),
        sketch.get_normalized_rank_error(false)
    );
    assert!(sketch.get_normalized_rank_error(true) > sketch.get_normalized_rank_error(false));
    assert!(matches!(
        KllSketch::<T>::normalized_rank_error_for_k(7, false),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(rank_error_t, rank_error);

fn to_string_summary_mentions_k<T: TestItem>() {
    let summary = filled::<T>(200, 10).to_string_summary();
    assert!(summary.contains("200"), "summary was: {summary}");
}
for_both_types!(to_string_summary_mentions_k_t, to_string_summary_mentions_k);
