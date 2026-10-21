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

// Byte offsets in the full (estimation/exact, n > 1) serialized layout.
const N_OFFSET: usize = 8;
const NUM_LEVELS_OFFSET: usize = 18;
const LEVELS_OFFSET: usize = 20;

fn estimation_bytes<T: TestItem>() -> Vec<u8> {
    let sketch = filled::<T>(200, 1000);
    assert!(sketch.is_estimation_mode());
    let bytes = sketch.serialize();
    assert!(bytes[NUM_LEVELS_OFFSET] >= 2, "need several levels");
    bytes
}

fn read_level(bytes: &[u8], i: usize) -> u32 {
    let at = LEVELS_OFFSET + 4 * i;
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn tampered_n_fails<T: TestItem>() {
    let mut bytes = estimation_bytes::<T>();
    bytes[N_OFFSET..N_OFFSET + 8].copy_from_slice(&5000u64.to_le_bytes());
    assert!(matches!(
        KllSketch::<T>::deserialize(&bytes),
        Err(SketchError::Deserialization(_))
    ));
}
for_both_types!(tampered_n_fails_t, tampered_n_fails);

fn decreasing_levels_fail<T: TestItem>() {
    // levels[0] raised above levels[1].
    let mut bytes = estimation_bytes::<T>();
    let bigger = read_level(&bytes, 1) + 1;
    bytes[LEVELS_OFFSET..LEVELS_OFFSET + 4].copy_from_slice(&bigger.to_le_bytes());
    assert!(matches!(
        KllSketch::<T>::deserialize(&bytes),
        Err(SketchError::Deserialization(_))
    ));
    // levels[1] dropped below levels[0]; levels[0] (and so the item count and
    // byte length) is untouched, so only a layout check can catch this.
    let mut bytes = estimation_bytes::<T>();
    let level0 = read_level(&bytes, 0);
    assert!(level0 > 0, "need room below levels[0]");
    let at = LEVELS_OFFSET + 4;
    bytes[at..at + 4].copy_from_slice(&(level0 - 1).to_le_bytes());
    assert!(matches!(
        KllSketch::<T>::deserialize(&bytes),
        Err(SketchError::Deserialization(_))
    ));
}
for_both_types!(decreasing_levels_fail_t, decreasing_levels_fail);

fn truncated_levels_fail<T: TestItem>() {
    let bytes = estimation_bytes::<T>();
    // Cut mid-way through the second level entry.
    let cut = LEVELS_OFFSET + 4 + 2;
    assert!(matches!(
        KllSketch::<T>::deserialize(&bytes[..cut]),
        Err(SketchError::Deserialization(_))
    ));
}
for_both_types!(truncated_levels_fail_t, truncated_levels_fail);

fn untampered_estimation_roundtrip_merges<T: TestItem>() {
    let restored = KllSketch::<T>::deserialize(&estimation_bytes::<T>()).unwrap();
    assert_eq!(restored.get_n(), 1000);
    let mut target = filled::<T>(200, 1000);
    target.merge(&restored);
    assert_eq!(target.get_n(), 2000);
    let mut other = KllSketch::<T>::default();
    other.merge(&restored);
    assert_eq!(other.get_n(), 1000);
}
for_both_types!(
    untampered_estimation_roundtrip_merges_t,
    untampered_estimation_roundtrip_merges
);

/// Mirror of upstream `kll_helper::int_cap_aux_aux` / `int_cap_aux`.
fn int_cap_aux_aux(k: u64, depth: u32) -> u64 {
    let twok = k << 1;
    let tmp = (twok << depth) / 3u64.pow(depth);
    (tmp + 1) >> 1
}

fn int_cap_aux(k: u64, depth: u32) -> u64 {
    if depth <= 30 {
        return int_cap_aux_aux(k, depth);
    }
    let half = depth / 2;
    int_cap_aux_aux(int_cap_aux_aux(k, half), depth - half)
}

/// Mirror of upstream `kll_helper::compute_total_capacity` with m = 8.
fn total_capacity(k: u16, num_levels: u8) -> u32 {
    (0..u32::from(num_levels))
        .map(|depth| int_cap_aux(u64::from(k), depth).max(8) as u32)
        .sum()
}

/// A k=200, n=3 sketch re-encoded with `num_levels` levels: all three items
/// in level 0, every higher level empty. The layout is self-consistent
/// (levels non-decreasing, weights sum to n) but has far more levels than
/// n = 3 could ever produce.
fn n3_bytes_with_levels<T: TestItem>(num_levels: u8) -> Vec<u8> {
    let real = filled::<T>(200, 3).serialize();
    assert_eq!(real[NUM_LEVELS_OFFSET], 1);
    let capacity = total_capacity(200, num_levels);
    let mut bytes = real[..NUM_LEVELS_OFFSET].to_vec();
    bytes.extend_from_slice(&[num_levels, 0]);
    bytes.extend_from_slice(&(capacity - 3).to_le_bytes());
    for _ in 1..num_levels {
        bytes.extend_from_slice(&capacity.to_le_bytes());
    }
    // min, max and the three items, unchanged.
    bytes.extend_from_slice(&real[LEVELS_OFFSET + 4..]);
    bytes
}

fn excess_levels_fail<T: TestItem>() {
    // Sanity: the builder reproduces the real encoding at one level.
    assert_eq!(
        n3_bytes_with_levels::<T>(1),
        filled::<T>(200, 3).serialize()
    );
    for num_levels in [10, 61] {
        // Assert on deserialize only: merging such a sketch aborts (10) or
        // overflows the heap (61) in upstream.
        assert!(
            matches!(
                KllSketch::<T>::deserialize(&n3_bytes_with_levels::<T>(num_levels)),
                Err(SketchError::Deserialization(_))
            ),
            "num_levels = {num_levels}"
        );
    }
}
for_both_types!(excess_levels_fail_t, excess_levels_fail);

fn genuine_sketches_roundtrip_and_merge<T: TestItem>() {
    for k in [8u16, 200] {
        for n in [1u32, 2, 3, 199, 200, 201, 1000, 10_000, 100_000] {
            let sketch = filled::<T>(k, n);
            let restored = KllSketch::<T>::deserialize(&sketch.serialize())
                .unwrap_or_else(|e| panic!("k={k} n={n}: {e:?}"));
            assert_eq!(restored.get_n(), u64::from(n));
            let mut small = filled::<T>(k, 1);
            small.merge(&restored);
            assert_eq!(small.get_n(), u64::from(n) + 1);
            let mut target = restored.clone();
            target.merge(&filled::<T>(k, 1));
            assert_eq!(target.get_n(), u64::from(n) + 1);
            let mut both = restored.clone();
            both.merge(&restored);
            assert_eq!(both.get_n(), 2 * u64::from(n));
        }
    }
}
for_both_types!(
    genuine_sketches_roundtrip_and_merge_t,
    genuine_sketches_roundtrip_and_merge
);

fn nan_bytes(width: usize) -> Vec<u8> {
    match width {
        8 => f64::NAN.to_le_bytes().to_vec(),
        4 => f32::NAN.to_le_bytes().to_vec(),
        _ => panic!("unexpected item width {width}"),
    }
}

fn nan_items_fail<T: TestItem>() {
    // Single-item layout: 8-byte header, then the one item.
    let mut bytes = filled::<T>(200, 1).serialize();
    let width = bytes.len() - 8;
    bytes[8..].copy_from_slice(&nan_bytes(width));
    assert!(matches!(
        KllSketch::<T>::deserialize(&bytes),
        Err(SketchError::Deserialization(_))
    ));

    // Full layout, n = 10, one level: header + levels[0], then min, max and
    // ten items.
    let real = filled::<T>(200, 10).serialize();
    let width = (real.len() - (LEVELS_OFFSET + 4)) / 12;
    let items_at = LEVELS_OFFSET + 4 + 2 * width;
    // A retained item, then the min field, set to NaN.
    for at in [items_at + 5 * width, LEVELS_OFFSET + 4] {
        let mut bytes = real.clone();
        bytes[at..at + width].copy_from_slice(&nan_bytes(width));
        assert!(
            matches!(
                KllSketch::<T>::deserialize(&bytes),
                Err(SketchError::Deserialization(_))
            ),
            "NaN at byte {at}"
        );
    }
}
for_both_types!(nan_items_fail_t, nan_items_fail);

fn small_k_fails<T: TestItem>() {
    let mut bytes = filled::<T>(200, 1).serialize();
    bytes[4..6].copy_from_slice(&7u16.to_le_bytes());
    assert!(matches!(
        KllSketch::<T>::deserialize(&bytes),
        Err(SketchError::Deserialization(_))
    ));
}
for_both_types!(small_k_fails_t, small_k_fails);
