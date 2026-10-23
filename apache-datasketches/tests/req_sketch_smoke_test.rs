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
    // Asymmetric rank: High and Low differ. For k = 12, n = 1_000_000 the exact
    // threshold is 36 / n = 3.6e-5, so rank 0.9 is not exact for either.
    // relative = sqrt(0.0512 / 3) / 12 * (High ? 0.1 : 0.9) ~= 0.00108866 / 0.00979794;
    // fixed = 0.084 / 12 = 0.007; error = min(relative, fixed).
    let unit = (0.0512_f64 / 3.0).sqrt() / 12.0;
    let high = ReqSketch::<T>::a_priori_rank_error(12, 0.9, RankAccuracy::High, 1_000_000).unwrap();
    let low = ReqSketch::<T>::a_priori_rank_error(12, 0.9, RankAccuracy::Low, 1_000_000).unwrap();
    let (exp_high, exp_low) = (
        (unit * 0.1).min(0.084 / 12.0),
        (unit * 0.9).min(0.084 / 12.0),
    );
    assert!((high - exp_high).abs() < 1e-9, "{high} vs {exp_high}");
    assert!((low - exp_low).abs() < 1e-9, "{low} vs {exp_low}");
    assert!(high < low, "{high} !< {low}");
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

// --- crafted bytes -----------------------------------------------------------
//
// Hand-built serialized sketches, following upstream `req_sketch::serialize`
// and `req_compactor::serialize` (all little-endian): an 8-byte header
// (preamble_ints, serial_version = 1, family = 17, flags, k u16, num_levels,
// num_raw_items); if more than one level, n u64 then min and max; then either
// the raw items or, per level, state u64, section_size_raw f32, lg_weight u8,
// num_sections u8, 2 padding bytes, num_items u32 and the items.

const IS_EMPTY: u8 = 1 << 2;
const IS_HIGH_RANK: u8 = 1 << 3;
const RAW_ITEMS: u8 = 1 << 4;

fn item_bytes<T: TestItem>(v: f64) -> Vec<u8> {
    if std::mem::size_of::<T>() == 8 {
        v.to_le_bytes().to_vec()
    } else {
        (v as f32).to_le_bytes().to_vec()
    }
}

fn header(preamble_ints: u8, flags: u8, k: u16, num_levels: u8, num_raw_items: u8) -> Vec<u8> {
    let mut out = vec![preamble_ints, 1, 17, flags];
    out.extend_from_slice(&k.to_le_bytes());
    out.push(num_levels);
    out.push(num_raw_items);
    out
}

/// One serialized compactor. `num_items` defaults to `items.len()`.
#[derive(Clone)]
struct Level {
    state: u64,
    section_size_raw: f32,
    lg_weight: u8,
    num_sections: u8,
    num_items: Option<u32>,
    items: Vec<f64>,
}

/// A level as a genuine `k = 12` sketch would have it before any section
/// shrinking: state 0, section size `k`, 3 sections.
fn level(lg_weight: u8, items: &[f64]) -> Level {
    Level {
        state: 0,
        section_size_raw: 12.0,
        lg_weight,
        num_sections: 3,
        num_items: None,
        items: items.to_vec(),
    }
}

fn level_bytes<T: TestItem>(l: &Level) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&l.state.to_le_bytes());
    out.extend_from_slice(&l.section_size_raw.to_le_bytes());
    out.push(l.lg_weight);
    out.push(l.num_sections);
    out.extend_from_slice(&[0, 0]);
    let num_items = l.num_items.unwrap_or(l.items.len() as u32);
    out.extend_from_slice(&num_items.to_le_bytes());
    for &v in &l.items {
        out.extend(item_bytes::<T>(v));
    }
    out
}

/// A non-raw, non-empty High-accuracy sketch over `levels`. With more than one
/// level, `n` defaults to the levels' total weight and min/max to the
/// extreme items.
fn compactor_sketch<T: TestItem>(k: u16, levels: &[Level], n: Option<u64>) -> Vec<u8> {
    let num_levels = levels.len() as u8;
    let mut out = header(
        if num_levels > 1 { 4 } else { 2 },
        IS_HIGH_RANK,
        k,
        num_levels,
        0,
    );
    if num_levels > 1 {
        let total: u64 = levels
            .iter()
            .map(|l| (l.items.len() as u64) << l.lg_weight)
            .sum();
        out.extend_from_slice(&n.unwrap_or(total).to_le_bytes());
        let all = levels.iter().flat_map(|l| l.items.iter().copied());
        let min = all.clone().fold(f64::INFINITY, f64::min);
        let max = all.fold(f64::NEG_INFINITY, f64::max);
        out.extend(item_bytes::<T>(min));
        out.extend(item_bytes::<T>(max));
    }
    for l in levels {
        out.extend(level_bytes::<T>(l));
    }
    out
}

/// A raw-items sketch (as upstream writes for `n <= 4`).
fn raw_sketch<T: TestItem>(k: u16, items: &[f64]) -> Vec<u8> {
    let mut out = header(2, IS_HIGH_RANK | RAW_ITEMS, k, 1, items.len() as u8);
    for &v in items {
        out.extend(item_bytes::<T>(v));
    }
    out
}

fn empty_sketch_bytes(k: u16) -> Vec<u8> {
    header(2, IS_EMPTY | IS_HIGH_RANK | RAW_ITEMS, k, 0, 0)
}

#[track_caller]
fn assert_rejected<T: TestItem>(bytes: &[u8], what: &str) {
    match ReqSketch::<T>::deserialize(bytes) {
        Err(SketchError::Deserialization(_)) => {}
        Err(e) => panic!("{what}: expected Deserialization, got {e:?}"),
        Ok(s) => panic!(
            "{what}: expected Deserialization, got Ok (n = {})",
            s.get_n()
        ),
    }
}

#[track_caller]
fn assert_accepted<T: TestItem>(bytes: &[u8], what: &str) -> ReqSketch<T> {
    match ReqSketch::<T>::deserialize(bytes) {
        Ok(s) => s,
        Err(e) => panic!("{what}: expected Ok, got {e:?}"),
    }
}

/// Runs every query and mutation on a deserialized sketch; none may fail
/// except where the API documents an error.
fn exercise<T: TestItem>(s: &ReqSketch<T>) {
    let x = T::from_f64(3.0);
    s.get_quantile(0.5, true).unwrap();
    s.get_rank(x, true).unwrap();
    s.get_cdf(&[x], true).unwrap();
    assert_eq!(s.sorted_view().iter().count() as u32, s.get_num_retained());
    assert_eq!(s.iter().map(|(_, w)| w).sum::<u64>(), s.get_n());
    let again = ReqSketch::<T>::deserialize(&s.serialize()).unwrap();
    assert_eq!(again.get_n(), s.get_n());
    let mut updated = s.clone();
    for i in 0..1000 {
        updated.update(T::from_f64(f64::from(i)));
    }
    let mut fresh = ReqSketch::<T>::new(254, s.rank_accuracy()).unwrap();
    fresh.merge(s).unwrap();
    let mut merged = s.clone();
    merged.merge(&fresh).unwrap();
}

fn single_level_with_zero_items_is_rejected<T: TestItem>() {
    let bytes = compactor_sketch::<T>(12, &[level(0, &[])], None);
    assert_eq!(&bytes[..8], &[2, 1, 17, 1 << 3, 12, 0, 1, 0]);
    assert_rejected::<T>(&bytes, "one level, zero items");
}
for_both_types!(
    single_level_with_zero_items_is_rejected_t,
    single_level_with_zero_items_is_rejected
);

fn num_items_beyond_the_buffer_is_rejected<T: TestItem>() {
    for num_items in [u32::MAX, 1 << 28, 6] {
        let mut l = level(0, &[1.0, 2.0, 3.0, 4.0, 5.0]);
        l.num_items = Some(num_items);
        assert_rejected::<T>(
            &compactor_sketch::<T>(12, &[l.clone()], None),
            &format!("single level num_items = {num_items}"),
        );
        let levels = [level(0, &[1.0, 2.0, 3.0]), l];
        assert_rejected::<T>(
            &compactor_sketch::<T>(12, &levels, Some(13)),
            &format!("top level num_items = {num_items}"),
        );
    }
    // A truncated compactor header.
    let bytes = compactor_sketch::<T>(12, &[level(0, &[1.0, 2.0, 3.0, 4.0, 5.0])], None);
    for len in [9, 20, 27] {
        assert_rejected::<T>(&bytes[..len], &format!("truncated at {len}"));
    }
}
for_both_types!(
    num_items_beyond_the_buffer_is_rejected_t,
    num_items_beyond_the_buffer_is_rejected
);

fn n_inconsistent_with_level_weights_is_rejected<T: TestItem>() {
    let levels = [level(0, &[1.0, 2.0, 3.0, 4.0, 5.0]), level(1, &[2.5, 4.5])];
    let ok = assert_accepted::<T>(&compactor_sketch::<T>(12, &levels, None), "consistent");
    assert_eq!(ok.get_n(), 9);
    exercise(&ok);
    for n in [0, 1, 8, 10, u64::MAX] {
        assert_rejected::<T>(
            &compactor_sketch::<T>(12, &levels, Some(n)),
            &format!("n = {n}"),
        );
    }
    // More than one level at n <= 4: upstream would serialize it as raw
    // items, reading past level 0.
    let tiny = [level(0, &[1.0]), level(1, &[2.0])];
    assert_rejected::<T>(&compactor_sketch::<T>(12, &tiny, None), "two levels, n = 3");
    // lg_weight must be the level index.
    let mut wrong_weight = levels.clone();
    wrong_weight[1].lg_weight = 2;
    assert_rejected::<T>(
        &compactor_sketch::<T>(12, &wrong_weight, Some(13)),
        "lg_weight 2 at level 1",
    );
    let mut wrong_base = levels.clone();
    wrong_base[0].lg_weight = 1;
    assert_rejected::<T>(
        &compactor_sketch::<T>(12, &wrong_base, Some(18)),
        "lg_weight 1 at level 0",
    );
}
for_both_types!(
    n_inconsistent_with_level_weights_is_rejected_t,
    n_inconsistent_with_level_weights_is_rejected
);

fn empty_level_is_rejected<T: TestItem>() {
    // Upstream's iterator dereferences an empty level and then runs past it.
    let cases: [(&str, Vec<Level>); 3] = [
        ("empty level 0", vec![level(0, &[]), level(1, &[1.0, 2.0])]),
        (
            "empty middle level",
            vec![level(0, &[1.0, 2.0]), level(1, &[]), level(2, &[3.0])],
        ),
        (
            "empty top level",
            vec![level(0, &[1.0, 2.0]), level(1, &[])],
        ),
    ];
    for (what, levels) in cases {
        assert_rejected::<T>(&compactor_sketch::<T>(12, &levels, None), what);
    }
}
for_both_types!(empty_level_is_rejected_t, empty_level_is_rejected);

fn bad_k_is_rejected<T: TestItem>() {
    for k in [0, 2, 3, 13, 1025, 1026, 2048, u16::MAX] {
        assert_rejected::<T>(&raw_sketch::<T>(k, &[1.0, 2.0]), &format!("raw k = {k}"));
        let mut l = level(0, &[1.0, 2.0, 3.0, 4.0, 5.0]);
        l.section_size_raw = 4.0;
        assert_rejected::<T>(
            &compactor_sketch::<T>(k, &[l], None),
            &format!("compactor k = {k}"),
        );
    }
    for k in [256, 300, 1024, u16::MAX] {
        assert_rejected::<T>(&empty_sketch_bytes(k), &format!("empty k = {k}"));
    }
    for k in [4, 12, 254] {
        let s = assert_accepted::<T>(&empty_sketch_bytes(k), "empty");
        assert!(s.is_empty());
        assert_eq!(s.get_k(), k);
    }
    // Non-empty bytes keep k as written, up to 1024.
    for k in [256, 300, 1024] {
        let s = assert_accepted::<T>(&raw_sketch::<T>(k, &[1.0, 2.0]), "raw");
        assert_eq!(s.get_k(), k);
        exercise(&s);
        let mut l = level(0, &[1.0, 2.0, 3.0, 4.0, 5.0]);
        l.section_size_raw = f32::from(k);
        let s = assert_accepted::<T>(&compactor_sketch::<T>(k, &[l], None), "compactor");
        assert_eq!(s.get_k(), k);
        exercise(&s);
    }
}
for_both_types!(bad_k_is_rejected_t, bad_k_is_rejected);

fn nan_items_are_rejected<T: TestItem>() {
    assert_rejected::<T>(&raw_sketch::<T>(12, &[1.0, f64::NAN]), "raw NaN");
    assert_rejected::<T>(
        &compactor_sketch::<T>(12, &[level(0, &[1.0, 2.0, f64::NAN, 4.0, 5.0])], None),
        "single-level NaN",
    );
    let levels = [level(0, &[1.0, 2.0, 3.0]), level(1, &[f64::NAN])];
    assert_rejected::<T>(&compactor_sketch::<T>(12, &levels, Some(5)), "level 1 NaN");
    // NaN in the min slot of a multi-level sketch.
    let levels = [level(0, &[1.0, 2.0, 3.0]), level(1, &[4.0])];
    let mut bytes = compactor_sketch::<T>(12, &levels, None);
    let width = std::mem::size_of::<T>();
    bytes[16..16 + width].copy_from_slice(&item_bytes::<T>(f64::NAN));
    assert_rejected::<T>(&bytes, "NaN min");
    // Infinities are ordinary items.
    assert_accepted::<T>(
        &raw_sketch::<T>(12, &[f64::NEG_INFINITY, f64::INFINITY]),
        "inf",
    );
}
for_both_types!(nan_items_are_rejected_t, nan_items_are_rejected);

fn malformed_header_and_layout_are_rejected<T: TestItem>() {
    let good = raw_sketch::<T>(12, &[1.0, 2.0]);
    assert_accepted::<T>(&good, "control");
    for len in 0..8 {
        assert_rejected::<T>(&good[..len], &format!("header truncated at {len}"));
    }
    for bit in [0, 1, 6, 7] {
        let mut bytes = good.clone();
        bytes[3] |= 1 << bit;
        assert_rejected::<T>(&bytes, &format!("flag bit {bit}"));
    }
    let mut trailing = good.clone();
    trailing.push(0);
    assert_rejected::<T>(&trailing, "trailing byte after raw items");
    let mut trailing = compactor_sketch::<T>(12, &[level(0, &[1.0, 2.0, 3.0, 4.0, 5.0])], None);
    trailing.push(0);
    assert_rejected::<T>(&trailing, "trailing byte after compactors");
    // Upstream pads genuine 2-to-4-item raw bytes with 20 zero bytes (sized
    // as if they held a compactor); exactly that is accepted, nothing else.
    let width = std::mem::size_of::<T>();
    for n in 1..=4u32 {
        let genuine = filled::<T>(n).serialize();
        let padding = if n == 1 { 0 } else { 20 };
        assert_eq!(genuine.len(), 8 + n as usize * width + padding, "n = {n}");
        assert_accepted::<T>(&genuine, "genuine raw");
        if n > 1 {
            let mut dirty = genuine.clone();
            *dirty.last_mut().unwrap() = 1;
            assert_rejected::<T>(&dirty, "non-zero padding");
            assert_rejected::<T>(&genuine[..genuine.len() - 1], "short padding");
        }
    }
    let mut trailing = empty_sketch_bytes(12);
    trailing.push(0);
    assert_rejected::<T>(&trailing, "trailing byte after empty header");
    // Raw items: count 1..=4, exactly that many items, a single level.
    assert_rejected::<T>(&header(2, IS_HIGH_RANK | RAW_ITEMS, 12, 1, 0), "0 raw");
    let mut five = header(2, IS_HIGH_RANK | RAW_ITEMS, 12, 1, 5);
    for v in 1..=5 {
        five.extend(item_bytes::<T>(f64::from(v)));
    }
    assert_rejected::<T>(&five, "5 raw items");
    let mut short = raw_sketch::<T>(12, &[1.0, 2.0]);
    short[7] = 3;
    assert_rejected::<T>(&short, "raw count beyond the buffer");
    let mut two_levels = header(4, IS_HIGH_RANK | RAW_ITEMS, 12, 2, 2);
    two_levels.extend_from_slice(&2u64.to_le_bytes());
    for v in [1.0, 2.0, 1.0, 2.0] {
        two_levels.extend(item_bytes::<T>(v));
    }
    assert_rejected::<T>(&two_levels, "raw items with two levels");
    // Non-empty with no levels.
    let mut no_levels = header(2, IS_HIGH_RANK | RAW_ITEMS, 12, 0, 1);
    no_levels.extend(item_bytes::<T>(1.0));
    assert_rejected::<T>(&no_levels, "raw, zero levels");
    assert_rejected::<T>(
        &header(2, IS_HIGH_RANK, 12, 0, 0),
        "compactors, zero levels",
    );
}
for_both_types!(
    malformed_header_and_layout_are_rejected_t,
    malformed_header_and_layout_are_rejected
);

fn bad_section_parameters_are_rejected<T: TestItem>() {
    let items = [1.0, 2.0, 3.0, 4.0, 5.0];
    // Genuine values: 3 << j sections, a section size in [3, k].
    for (num_sections, ssr) in [(3, 12.0), (6, 8.485_281), (12, 6.0), (48, 3.0)] {
        let mut l = level(0, &items);
        l.num_sections = num_sections;
        l.section_size_raw = ssr;
        exercise(&assert_accepted::<T>(
            &compactor_sketch::<T>(12, &[l], None),
            &format!("{num_sections} sections of {ssr}"),
        ));
    }
    for num_sections in [0, 1, 2, 4, 96, 192, 255] {
        let mut l = level(0, &items);
        l.num_sections = num_sections;
        assert_rejected::<T>(
            &compactor_sketch::<T>(12, &[l], None),
            &format!("num_sections = {num_sections}"),
        );
    }
    // nearest_even(ssr) must be at least 4 and ssr at most k.
    for ssr in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -4.0,
        0.0,
        1.0,
        2.99,
        12.01,
        1e30,
    ] {
        let mut l = level(0, &items);
        l.section_size_raw = ssr;
        assert_rejected::<T>(
            &compactor_sketch::<T>(12, &[l], None),
            &format!("section_size_raw = {ssr}"),
        );
    }
}
for_both_types!(
    bad_section_parameters_are_rejected_t,
    bad_section_parameters_are_rejected
);

fn huge_compaction_state_is_rejected<T: TestItem>() {
    // Merged into a k = 254 sketch, a huge state keeps doubling the receiving
    // level's sections (req_compactor::ensure_enough_sections) past 48, where
    // `1ULL << (num_sections - 1)` shifts by 64 or more and the uint8_t count
    // wraps to 0, leaving a zero nominal capacity.
    for state in [u64::MAX, 1 << 63, 1 << 47, 1 << 46] {
        let mut l = level(0, &[1.0, 2.0, 3.0, 4.0, 5.0]);
        l.state = state;
        assert_rejected::<T>(
            &compactor_sketch::<T>(12, &[l], None),
            &format!("state = {state}"),
        );
    }
    let mut l = level(0, &[1.0, 2.0, 3.0, 4.0, 5.0]);
    l.state = (1 << 46) - 1;
    let s = assert_accepted::<T>(&compactor_sketch::<T>(12, &[l], None), "state below 2^46");
    exercise(&s);
}
for_both_types!(
    huge_compaction_state_is_rejected_t,
    huge_compaction_state_is_rejected
);

fn retained_at_nominal_capacity_is_rejected<T: TestItem>() {
    // k = 4: one level of nominal capacity 2 * 3 * 4 = 24. Upstream compresses
    // on update only when the retained count *equals* the total capacity, so
    // a sketch already at or past it would never compact again.
    let items: Vec<f64> = (1..=24).map(f64::from).collect();
    let mut l = level(0, &items);
    l.section_size_raw = 4.0;
    assert_rejected::<T>(&compactor_sketch::<T>(4, &[l.clone()], None), "24 of 24");
    l.items.pop();
    let s = assert_accepted::<T>(&compactor_sketch::<T>(4, &[l], None), "23 of 24");
    exercise(&s);
    assert!(s.get_num_retained() < 24);
}
for_both_types!(
    retained_at_nominal_capacity_is_rejected_t,
    retained_at_nominal_capacity_is_rejected
);

/// `count` levels, one item each, so level `i` carries weight `2^i`.
fn deep_levels(count: u8) -> Vec<Level> {
    (0..count).map(|i| level(i, &[f64::from(i)])).collect()
}

fn n_limit_is_enforced<T: TestItem>() {
    // 63 one-item levels: n = 2^63 - 1, the largest n accepted.
    let s = assert_accepted::<T>(
        &compactor_sketch::<T>(12, &deep_levels(63), None),
        "2^63 - 1",
    );
    assert_eq!(s.get_n(), u64::MAX >> 1);
    s.get_quantile(1.0, true).unwrap();
    s.get_quantile(0.0, false).unwrap();
    s.get_rank(T::from_f64(100.0), true).unwrap();
    // One more item at level 0: n = 2^63.
    let mut levels = deep_levels(63);
    levels[0].items.push(0.5);
    assert_rejected::<T>(&compactor_sketch::<T>(12, &levels, None), "2^63");
    // 64 levels cannot be summed below 2^63 either.
    assert_rejected::<T>(
        &compactor_sketch::<T>(12, &deep_levels(64), None),
        "64 levels",
    );

    // n = 2^62: merging it with itself reaches 2^63, which fits in u64 but
    // is refused.
    let mut levels = deep_levels(62);
    levels[0].items.push(0.5);
    let half = assert_accepted::<T>(&compactor_sketch::<T>(12, &levels, None), "2^62");
    assert_eq!(half.get_n(), 1 << 62);
    let mut a = half.clone();
    assert!(matches!(a.merge(&half), Err(SketchError::InvalidConfig(_))));
    assert_eq!(
        a.get_n(),
        1 << 62,
        "a rejected merge must not change the sketch"
    );
    // Just below the limit merges fine.
    let mut below = assert_accepted::<T>(
        &compactor_sketch::<T>(12, &deep_levels(62), None),
        "2^62 - 1",
    );
    below.merge(&half).unwrap();
    assert_eq!(below.get_n(), (1 << 63) - 1);
    below.get_quantile(0.5, true).unwrap();
}
for_both_types!(n_limit_is_enforced_t, n_limit_is_enforced);

fn untampered_round_trips_still_work<T: TestItem>() {
    for accuracy in [RankAccuracy::High, RankAccuracy::Low] {
        for n in [1u32, 4, 5, 12, 100, 10_000, 1_000_000] {
            let mut sketch = ReqSketch::<T>::new(12, accuracy).unwrap();
            for i in 1..=n {
                sketch.update(T::from_f64(f64::from(i)));
            }
            let restored = ReqSketch::<T>::deserialize(&sketch.serialize())
                .unwrap_or_else(|e| panic!("n = {n}, {accuracy:?}: {e:?}"));
            assert_eq!(restored.get_n(), u64::from(n));
            assert_eq!(restored.get_k(), 12);
            assert_eq!(restored.rank_accuracy(), accuracy);
            restored.get_quantile(0.5, true).unwrap();
            restored.get_rank(T::from_f64(1.0), true).unwrap();
            let mut a = restored.clone();
            a.merge(&sketch).unwrap();
            let mut b = sketch.clone();
            b.merge(&restored).unwrap();
            assert_eq!(a.get_n(), 2 * u64::from(n));
            assert_eq!(b.get_n(), 2 * u64::from(n));
            a.get_quantile(0.99, true).unwrap();
            b.get_cdf(&[T::from_f64(10.0)], false).unwrap();
            ReqSketch::<T>::deserialize(&a.serialize()).unwrap();
        }
    }
}
for_both_types!(
    untampered_round_trips_still_work_t,
    untampered_round_trips_still_work
);
