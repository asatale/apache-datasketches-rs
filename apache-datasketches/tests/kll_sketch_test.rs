//! Port of upstream kll/test/kll_sketch_test.cpp, run for both f64 and f32
//! (upstream uses float).
//!
//! Sections not ported, and why:
//! - "stream serialize deserialize *": no std::istream API in this binding.
//! - "bytes serialize deserialize many ints", "out of order split points,
//!   int", "sketch of ints", "sketch of strings *", "issue #236", "type
//!   conversion: *", "max serialized size *": item types and APIs this
//!   binding does not expose.
//! - "floor of log2 of fraction": tests an internal helper.
//! - "move": Rust moves are not observable.
//! - "sorted view": ported in kll_sorted_view_test.rs.
//! - "deserialize one item v1": ported in kll_serialization_compat_test.rs.

#[macro_use]
mod kll_common;

use apache_datasketches::kll::KllSketch;
use apache_datasketches::SketchError;
use kll_common::TestItem;

const RANK_EPS_FOR_K_200: f64 = 0.0133;
const NUMERIC_NOISE_TOLERANCE: f64 = 1e-6;

fn t<T: TestItem>(v: f64) -> T {
    T::from_f64(v)
}

fn k_limits<T: TestItem>() {
    assert!(KllSketch::<T>::new(8).is_ok());
    assert!(KllSketch::<T>::new(u16::MAX).is_ok());
    assert!(matches!(
        KllSketch::<T>::new(7),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(k_limits_t, k_limits);

fn empty<T: TestItem>() {
    let sketch = KllSketch::<T>::default();
    assert!(sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 0);
    assert_eq!(sketch.get_num_retained(), 0);
    assert_eq!(sketch.get_min_item(), None);
    assert_eq!(sketch.get_max_item(), None);
    assert!(matches!(
        sketch.get_rank(t(0.0), true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_quantile(0.5, true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_pmf(&[t(0.0)], true),
        Err(SketchError::EmptySketch)
    ));
    assert!(matches!(
        sketch.get_cdf(&[t(0.0)], true),
        Err(SketchError::EmptySketch)
    ));
    assert_eq!(sketch.iter().count(), 0);
}
for_both_types!(empty_t, empty);

fn get_bad_quantile<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(0.0));
    assert!(matches!(
        sketch.get_quantile(-1.0, true),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(get_bad_quantile_t, get_bad_quantile);

fn one_item<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(1.0));
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_n(), 1);
    assert_eq!(sketch.get_num_retained(), 1);
    assert_eq!(sketch.get_rank(t(1.0), false).unwrap(), 0.0);
    assert_eq!(sketch.get_rank(t(1.0), true).unwrap(), 1.0);
    assert_eq!(sketch.get_rank(t(2.0), false).unwrap(), 1.0);
    assert_eq!(sketch.get_rank(t(f64::INFINITY), true).unwrap(), 1.0);
    assert_eq!(sketch.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch.get_max_item(), Some(t(1.0)));
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), t(1.0));
    let pairs: Vec<(T, u64)> = sketch.iter().collect();
    assert_eq!(pairs, vec![(t(1.0), 1)]);
}
for_both_types!(one_item_t, one_item);

fn nan<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(f64::NAN));
    assert!(sketch.is_empty());
    sketch.update(t(0.0));
    sketch.update(t(f64::NAN));
    assert_eq!(sketch.get_n(), 1);
}
for_both_types!(nan_t, nan);

fn many_items_exact_mode<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    let n = 200u32;
    for i in 1..=n {
        sketch.update(t(f64::from(i)));
        assert_eq!(sketch.get_n(), u64::from(i));
    }
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_num_retained(), n);
    assert_eq!(sketch.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch.get_quantile(0.0, true).unwrap(), t(1.0));
    assert_eq!(sketch.get_max_item(), Some(t(f64::from(n))));
    assert_eq!(sketch.get_quantile(1.0, true).unwrap(), t(f64::from(n)));
    for i in 1..=n {
        let item = t(f64::from(i));
        assert_eq!(
            sketch.get_rank(item, true).unwrap(),
            f64::from(i) / f64::from(n)
        );
        assert_eq!(
            sketch.get_rank(item, false).unwrap(),
            f64::from(i - 1) / f64::from(n)
        );
    }
}
for_both_types!(many_items_exact_mode_t, many_items_exact_mode);

fn ten_items<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    for i in 1..=10 {
        sketch.update(t(f64::from(i)));
    }
    assert_eq!(sketch.get_quantile(0.0, true).unwrap(), t(1.0));
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), t(5.0));
    assert_eq!(sketch.get_quantile(0.99, true).unwrap(), t(10.0));
    assert_eq!(sketch.get_quantile(1.0, true).unwrap(), t(10.0));
}
for_both_types!(ten_items_t, ten_items);

fn hundred_items<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    for i in 0..100 {
        sketch.update(t(f64::from(i)));
    }
    assert_eq!(sketch.get_quantile(0.0, true).unwrap(), t(0.0));
    assert_eq!(sketch.get_quantile(0.01, true).unwrap(), t(0.0));
    assert_eq!(sketch.get_quantile(0.5, true).unwrap(), t(49.0));
    assert_eq!(sketch.get_quantile(0.99, true).unwrap(), t(98.0));
    assert_eq!(sketch.get_quantile(1.0, true).unwrap(), t(99.0));
}
for_both_types!(hundred_items_t, hundred_items);

fn many_items_estimation_mode<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    let n = 1_000_000u32;
    for i in 0..n {
        sketch.update(t(f64::from(i)));
        assert_eq!(sketch.get_n(), u64::from(i) + 1);
    }
    assert!(!sketch.is_empty());
    assert!(sketch.is_estimation_mode());
    assert_eq!(sketch.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch.get_max_item(), Some(t(f64::from(n - 1))));
    for i in 0..n {
        let true_rank = f64::from(i) / f64::from(n);
        let rank = sketch.get_rank(t(f64::from(i)), false).unwrap();
        assert!(
            (rank - true_rank).abs() <= RANK_EPS_FOR_K_200,
            "item {i}: rank {rank} vs true {true_rank}"
        );
    }
    let mut count = 0u32;
    let mut total_weight = 0u64;
    for (_, w) in sketch.iter() {
        count += 1;
        total_weight += w;
    }
    assert_eq!(count, sketch.get_num_retained());
    assert_eq!(total_weight, sketch.get_n());
}
for_both_types!(many_items_estimation_mode_t, many_items_estimation_mode);

fn consistency_between_rank_and_pmf_cdf<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    let values: Vec<T> = (0..200).map(|i| t(f64::from(i))).collect();
    for &v in &values {
        sketch.update(v);
    }
    for inclusive in [false, true] {
        let ranks = sketch.get_cdf(&values, inclusive).unwrap();
        let pmf = sketch.get_pmf(&values, inclusive).unwrap();
        let mut subtotal_pmf = 0.0;
        for (i, &v) in values.iter().enumerate() {
            assert_eq!(
                sketch.get_rank(v, inclusive).unwrap(),
                ranks[i],
                "rank vs CDF at {i}"
            );
            subtotal_pmf += pmf[i];
            assert!(
                (ranks[i] - subtotal_pmf).abs() <= NUMERIC_NOISE_TOLERANCE,
                "CDF vs PMF at {i}"
            );
        }
    }
}
for_both_types!(
    consistency_between_rank_and_pmf_cdf_t,
    consistency_between_rank_and_pmf_cdf
);

fn bytes_serialize_deserialize_empty<T: TestItem>() {
    let sketch = KllSketch::<T>::default();
    let sketch2 = KllSketch::<T>::deserialize(&sketch.serialize()).unwrap();
    assert_eq!(sketch2.is_empty(), sketch.is_empty());
    assert_eq!(sketch2.is_estimation_mode(), sketch.is_estimation_mode());
    assert_eq!(sketch2.get_n(), sketch.get_n());
    assert_eq!(sketch2.get_num_retained(), sketch.get_num_retained());
    assert_eq!(sketch2.get_min_item(), None);
    assert_eq!(sketch2.get_max_item(), None);
    assert_eq!(
        sketch2.get_normalized_rank_error(false),
        sketch.get_normalized_rank_error(false)
    );
    assert_eq!(
        sketch2.get_normalized_rank_error(true),
        sketch.get_normalized_rank_error(true)
    );
}
for_both_types!(
    bytes_serialize_deserialize_empty_t,
    bytes_serialize_deserialize_empty
);

fn bytes_serialize_deserialize_one_item<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(1.0));
    let sketch2 = KllSketch::<T>::deserialize(&sketch.serialize()).unwrap();
    assert!(!sketch2.is_empty());
    assert!(!sketch2.is_estimation_mode());
    assert_eq!(sketch2.get_n(), 1);
    assert_eq!(sketch2.get_num_retained(), 1);
    assert_eq!(sketch2.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch2.get_max_item(), Some(t(1.0)));
    assert_eq!(sketch2.get_quantile(0.5, true).unwrap(), t(1.0));
    assert_eq!(sketch2.get_rank(t(1.0), false).unwrap(), 0.0);
    assert_eq!(sketch2.get_rank(t(2.0), false).unwrap(), 1.0);
}
for_both_types!(
    bytes_serialize_deserialize_one_item_t,
    bytes_serialize_deserialize_one_item
);

fn bytes_serialize_deserialize_three_items<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    for v in [1.0, 2.0, 3.0] {
        sketch.update(t(v));
    }
    let sketch2 = KllSketch::<T>::deserialize(&sketch.serialize()).unwrap();
    assert!(!sketch2.is_empty());
    assert!(!sketch2.is_estimation_mode());
    assert_eq!(sketch2.get_n(), 3);
    assert_eq!(sketch2.get_num_retained(), 3);
    assert_eq!(sketch2.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch2.get_max_item(), Some(t(3.0)));
}
for_both_types!(
    bytes_serialize_deserialize_three_items_t,
    bytes_serialize_deserialize_three_items
);

fn bytes_serialize_deserialize_many<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    let n = 1000;
    for i in 0..n {
        sketch.update(t(f64::from(i)));
    }
    let bytes = sketch.serialize();
    let sketch2 = KllSketch::<T>::deserialize(&bytes).unwrap();
    assert_eq!(sketch2.is_empty(), sketch.is_empty());
    assert_eq!(sketch2.is_estimation_mode(), sketch.is_estimation_mode());
    assert_eq!(sketch2.get_n(), sketch.get_n());
    assert_eq!(sketch2.get_num_retained(), sketch.get_num_retained());
    assert_eq!(sketch2.get_min_item(), sketch.get_min_item());
    assert_eq!(sketch2.get_max_item(), sketch.get_max_item());
    assert_eq!(
        sketch2.get_normalized_rank_error(false),
        sketch.get_normalized_rank_error(false)
    );
    assert_eq!(
        sketch2.get_normalized_rank_error(true),
        sketch.get_normalized_rank_error(true)
    );
    assert_eq!(
        sketch2.get_quantile(0.5, true).unwrap(),
        sketch.get_quantile(0.5, true).unwrap()
    );
    assert_eq!(
        sketch2.get_rank(t(0.0), true).unwrap(),
        sketch.get_rank(t(0.0), true).unwrap()
    );
    let top = t(f64::from(n));
    assert_eq!(
        sketch2.get_rank(top, true).unwrap(),
        sketch.get_rank(top, true).unwrap()
    );
    for len in [7, 15, bytes.len() - 1] {
        assert!(
            matches!(
                KllSketch::<T>::deserialize(&bytes[..len]),
                Err(SketchError::Deserialization(_))
            ),
            "truncated to {len} bytes"
        );
    }
}
for_both_types!(
    bytes_serialize_deserialize_many_t,
    bytes_serialize_deserialize_many
);

fn out_of_order_split_points<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(0.0));
    assert!(matches!(
        sketch.get_cdf(&[t(1.0), t(0.0)], true),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(out_of_order_split_points_t, out_of_order_split_points);

fn nan_split_point<T: TestItem>() {
    let mut sketch = KllSketch::<T>::default();
    sketch.update(t(0.0));
    assert!(matches!(
        sketch.get_cdf(&[t(f64::NAN)], true),
        Err(SketchError::InvalidConfig(_))
    ));
}
for_both_types!(nan_split_point_t, nan_split_point);

/// Two sketches over `0..n` and `n..2n`, the second fed in descending order.
fn two_halves<T: TestItem>(k1: u16, k2: u16, n: u32) -> (KllSketch<T>, KllSketch<T>) {
    let mut sketch1 = KllSketch::<T>::new(k1).unwrap();
    let mut sketch2 = KllSketch::<T>::new(k2).unwrap();
    for i in 0..n {
        sketch1.update(t(f64::from(i)));
        sketch2.update(t(f64::from(2 * n - i - 1)));
    }
    (sketch1, sketch2)
}

fn merge<T: TestItem>() {
    let n = 10_000u32;
    let (mut sketch1, sketch2) = two_halves::<T>(200, 200, n);
    assert_eq!(sketch1.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch1.get_max_item(), Some(t(f64::from(n - 1))));
    assert_eq!(sketch2.get_min_item(), Some(t(f64::from(n))));
    assert_eq!(sketch2.get_max_item(), Some(t(f64::from(2 * n - 1))));
    sketch1.merge(&sketch2);
    assert!(!sketch1.is_empty());
    assert_eq!(sketch1.get_n(), u64::from(2 * n));
    assert_eq!(sketch1.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch1.get_max_item(), Some(t(f64::from(2 * n - 1))));
    let median = sketch1.get_quantile(0.5, true).unwrap().to_f64();
    assert!((median - f64::from(n)).abs() <= f64::from(n) * RANK_EPS_FOR_K_200);
}
for_both_types!(merge_t, merge);

fn merge_lower_k<T: TestItem>() {
    let n = 10_000u32;
    let (mut sketch1, sketch2) = two_halves::<T>(256, 128, n);
    assert_eq!(sketch1.get_k(), 256);
    assert_eq!(sketch2.get_k(), 128);
    assert!(sketch1.get_normalized_rank_error(false) < sketch2.get_normalized_rank_error(false));
    assert!(sketch1.get_normalized_rank_error(true) < sketch2.get_normalized_rank_error(true));
    sketch1.merge(&sketch2);
    // sketch1 is "contaminated" by sketch2's lower k.
    assert_eq!(
        sketch2.get_normalized_rank_error(false),
        sketch1.get_normalized_rank_error(false)
    );
    assert_eq!(
        sketch2.get_normalized_rank_error(true),
        sketch1.get_normalized_rank_error(true)
    );
    assert_eq!(sketch1.get_n(), u64::from(2 * n));
    assert_eq!(sketch1.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch1.get_max_item(), Some(t(f64::from(2 * n - 1))));
    let median = sketch1.get_quantile(0.5, true).unwrap().to_f64();
    assert!((median - f64::from(n)).abs() <= f64::from(n) * RANK_EPS_FOR_K_200);
}
for_both_types!(merge_lower_k_t, merge_lower_k);

fn merge_exact_mode_lower_k<T: TestItem>() {
    let n = 10_000u32;
    let mut sketch1 = KllSketch::<T>::new(256).unwrap();
    let mut sketch2 = KllSketch::<T>::new(128).unwrap();
    for i in 0..n {
        sketch1.update(t(f64::from(i)));
    }
    // Merging an empty sketch with lower k must not change the rank error.
    let before = sketch1.get_normalized_rank_error(true);
    sketch1.merge(&sketch2);
    assert_eq!(sketch1.get_normalized_rank_error(true), before);
    assert_eq!(sketch1.get_n(), u64::from(n));
    assert_eq!(sketch1.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch1.get_max_item(), Some(t(f64::from(n - 1))));
    let median = sketch1.get_quantile(0.5, true).unwrap().to_f64();
    let half = f64::from(n / 2);
    assert!((median - half).abs() <= half * RANK_EPS_FOR_K_200);
    // Nor must merging an exact-mode sketch with lower k.
    sketch2.update(t(0.0));
    sketch1.merge(&sketch2);
    assert_eq!(sketch1.get_normalized_rank_error(true), before);
}
for_both_types!(merge_exact_mode_lower_k_t, merge_exact_mode_lower_k);

fn merge_min_value_from_other<T: TestItem>() {
    let mut sketch1 = KllSketch::<T>::default();
    let mut sketch2 = KllSketch::<T>::default();
    sketch1.update(t(1.0));
    sketch2.update(t(2.0));
    sketch2.merge(&sketch1);
    assert_eq!(sketch2.get_min_item(), Some(t(1.0)));
    assert_eq!(sketch2.get_max_item(), Some(t(2.0)));
}
for_both_types!(merge_min_value_from_other_t, merge_min_value_from_other);

fn merge_min_and_max_values_from_other<T: TestItem>() {
    let mut sketch1 = KllSketch::<T>::default();
    for i in 0..1_000_000 {
        sketch1.update(t(f64::from(i)));
    }
    let mut sketch2 = KllSketch::<T>::default();
    sketch2.merge(&sketch1);
    assert_eq!(sketch2.get_min_item(), Some(t(0.0)));
    assert_eq!(sketch2.get_max_item(), Some(t(999_999.0)));
}
for_both_types!(
    merge_min_and_max_values_from_other_t,
    merge_min_and_max_values_from_other
);

fn copy<T: TestItem>() {
    let mut sketch1 = KllSketch::<T>::default();
    for i in 0..1000 {
        sketch1.update(t(f64::from(i)));
    }
    let sketch2 = sketch1.clone();
    for i in 0..1000 {
        let item = t(f64::from(i));
        assert_eq!(
            sketch2.get_rank(item, true).unwrap(),
            sketch1.get_rank(item, true).unwrap()
        );
    }
}
for_both_types!(copy_t, copy);
