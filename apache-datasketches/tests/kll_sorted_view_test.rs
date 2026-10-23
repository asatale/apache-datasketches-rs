#[macro_use]
mod kll_common;

use apache_datasketches::kll::{KllSketch, SortedViewEntry};
use apache_datasketches::SketchError;
use kll_common::TestItem;

fn filled<T: TestItem>(n: u32) -> KllSketch<T> {
    let mut sketch = KllSketch::<T>::default();
    for i in 0..n {
        sketch.update(T::from_f64(f64::from(i)));
    }
    sketch
}

// Port of upstream kll_sketch_test.cpp SECTION("sorted view").
fn upstream_three_items<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    for v in [2.0, 3.0, 1.0] {
        sketch.update(T::from_f64(v));
    }
    let view = sketch.sorted_view();
    assert_eq!(view.len(), 3);
    let entries: Vec<SortedViewEntry<T>> = view.iter().collect();
    let expected: Vec<SortedViewEntry<T>> = (1..=3u64)
        .map(|i| SortedViewEntry {
            item: T::from_f64(i as f64),
            weight: 1,
            cumulative_weight: i,
        })
        .collect();
    assert_eq!(entries, expected);
}
for_both_types!(upstream_three_items_t, upstream_three_items);

// The view and the sketch answer from the same retained items, so they must
// agree exactly even in estimation mode.
fn view_matches_sketch<T: TestItem>() {
    let sketch = filled::<T>(10_000);
    assert!(sketch.is_estimation_mode());
    let view = sketch.sorted_view();
    for i in 0..=100 {
        let rank = f64::from(i) / 100.0;
        for inclusive in [true, false] {
            assert_eq!(
                view.get_quantile(rank, inclusive).unwrap(),
                sketch.get_quantile(rank, inclusive).unwrap()
            );
        }
    }
    for v in (0..10_000).step_by(97) {
        let item = T::from_f64(f64::from(v));
        for inclusive in [true, false] {
            assert_eq!(
                view.get_rank(item, inclusive).unwrap(),
                sketch.get_rank(item, inclusive).unwrap()
            );
        }
    }
    let split: Vec<T> = [1000.0, 5000.0, 9000.0].map(T::from_f64).to_vec();
    assert_eq!(
        view.get_pmf(&split, true).unwrap(),
        sketch.get_pmf(&split, true).unwrap()
    );
    assert_eq!(
        view.get_cdf(&split, false).unwrap(),
        sketch.get_cdf(&split, false).unwrap()
    );
}
for_both_types!(view_matches_sketch_t, view_matches_sketch);

fn entries_are_sorted_and_cumulative<T: TestItem>() {
    let sketch = filled::<T>(10_000);
    let entries: Vec<SortedViewEntry<T>> = sketch.sorted_view().iter().collect();
    assert_eq!(entries.len() as u32, sketch.get_num_retained());
    assert!(entries.windows(2).all(|w| w[0].item <= w[1].item));
    let mut running = 0;
    for e in &entries {
        running += e.weight;
        assert_eq!(e.cumulative_weight, running);
    }
    assert_eq!(running, sketch.get_n());
}
for_both_types!(
    entries_are_sorted_and_cumulative_t,
    entries_are_sorted_and_cumulative
);

fn empty_view<T: TestItem>() {
    let view = KllSketch::<T>::default().sorted_view();
    assert_eq!(view.len(), 0);
    assert!(view.is_empty());
    assert_eq!(view.iter().count(), 0);
    assert!(matches!(
        view.get_quantile(0.5, true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        view.get_rank(T::from_f64(1.0), true),
        Err(SketchError::EmptySketch)
    ));
}
for_both_types!(empty_view_t, empty_view);

fn view_is_a_snapshot<T: TestItem>() {
    let mut sketch = filled::<T>(10);
    let view = sketch.sorted_view();
    sketch.update(T::from_f64(100.0));
    assert_eq!(view.len(), 10);
    assert_eq!(view.get_quantile(1.0, true).unwrap().to_f64(), 9.0);
}
for_both_types!(view_is_a_snapshot_t, view_is_a_snapshot);

fn view_validates_arguments<T: TestItem>() {
    let view = filled::<T>(10).sorted_view();
    assert!(matches!(
        view.get_quantile(1.5, true),
        Err(SketchError::InvalidConfig(_))
    ));
    assert!(matches!(
        view.get_rank(T::from_f64(f64::NAN), true),
        Err(SketchError::InvalidConfig(_))
    ));
    let bad = [T::from_f64(2.0), T::from_f64(1.0)];
    assert!(matches!(
        view.get_cdf(&bad, true),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(view_validates_arguments_t, view_validates_arguments);
