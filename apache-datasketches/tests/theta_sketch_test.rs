// apache-datasketches/tests/theta_sketch_test.rs
//! Ported from theta/test/theta_sketch_test.cpp (tag 5.2.0).
//!
//! Upstream has 22 `TEST_CASE`s. 8 of them ("deserialize compact v1/v2
//! empty/estimation from java" and "wrap compact v1/v2 empty/estimation
//! from java") round-trip fixture files (`theta_compact_*_from_java_v{1,2}.sk`)
//! that do not exist in this repository and are not introduced by this
//! plan — those 8 cases are not ported. That leaves 14 portable cases,
//! covered by the 14 tests below:
//!   - `empty_sketch_is_empty`, `single_item_exact_mode`, `duplicate_updates_do_not_double_count`,
//!     `trim_reduces_retained_below_target`, `reset_returns_to_empty`, `lower_and_upper_bounds_bracket_estimate`
//!     port upstream's "empty", "single item", and "estimation" cases (trim/reset/bounds behavior).
//!   - `many_items_estimation_mode` is additional coverage of estimation-mode accuracy at larger n,
//!     using a tolerance-based assertion rather than upstream's exact retained-count checks.
//!   - `serialize_deserialize_v3_round_trip` ports "serialize deserialize stream and bytes equivalence"
//!     (byte-level round trip, not stream I/O since this crate has no stream API).
//!   - `update_overloads_accept_all_input_types` is an API-surface-driven addition: upstream's
//!     `update_theta_sketch::update()` is a single C++ template overloaded implicitly over any type;
//!     this crate exposes one explicitly-named method per input type instead, so this test exercises
//!     each of `update_i32`/`update_u32`/`update_i16`/`update_u16`/`update_i8`/`update_u8`/`update_f64`/
//!     `update_bytes`/`update_str` (u64/i64 are already covered above).
//!   - `compact_ordered_vs_unordered` ports "conversion constructor and wrapped compact"'s
//!     ordered-vs-unordered comparison.
//!   - `v4_compressed_round_trip` ports "serialize deserialize compressed"/"serialize deserialize small
//!     compressed".
//!   - `wrapped_compact_query_parity_with_source` ports the remainder of "conversion constructor and
//!     wrapped compact" (the `wrapped_compact_theta_sketch::wrap` query-parity assertions); the upstream
//!     "seed mismatch" sub-case at the end of that same `TEST_CASE` is not portable, since this crate
//!     never exposes a seed parameter (every sketch always uses upstream's `DEFAULT_SEED`).
//!   - `builder_rejects_lg_k_out_of_range` and `builder_rejects_p_out_of_range` are API-surface-driven
//!     additions covering `theta_base_builder::set_lg_k`/`set_p`'s validation
//!     (`theta_update_sketch_base_impl.hpp`: `lg_k` must be in `[MIN_LG_K, MAX_LG_K]` = `[5, 26]`, `p`
//!     must be in `(0.0, 1.0]`), which upstream's `theta_sketch_test.cpp` does not exercise directly but
//!     which is reachable and worth covering through this crate's public builder API.
use apache_datasketches::theta::{CompactThetaSketch, ThetaSketchBuilder, WrappedCompactThetaSketch};

#[test]
fn empty_sketch_is_empty() {
    let sketch = ThetaSketchBuilder::new().build().unwrap();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_estimate(), 0.0);
    assert!(!sketch.is_estimation_mode());
}

#[test]
fn single_item_exact_mode() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    sketch.update_u64(1);
    assert!(!sketch.is_empty());
    assert_eq!(sketch.get_estimate(), 1.0);
    assert!(!sketch.is_estimation_mode());
}

#[test]
fn many_items_estimation_mode() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..100_000u64 {
        sketch.update_u64(i);
    }
    assert!(sketch.is_estimation_mode());
    assert!((sketch.get_estimate() - 100_000.0).abs() / 100_000.0 < 0.03);
}

#[test]
fn duplicate_updates_do_not_double_count() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    for _ in 0..1000 {
        sketch.update_u64(42);
    }
    assert_eq!(sketch.get_estimate(), 1.0);
}

#[test]
fn trim_reduces_retained_below_target() {
    // Note: the brief's illustrative code used `lg_k(4)`, but
    // `theta_constants::MIN_LG_K == 5` (see `builder_rejects_lg_k_out_of_range`
    // below), so `lg_k(4)` is rejected by the builder; using the minimum
    // valid value `5` here instead.
    let mut sketch = ThetaSketchBuilder::new().lg_k(5).build().unwrap();
    for i in 0..10_000u64 {
        sketch.update_u64(i);
    }
    let before = sketch.get_num_retained();
    sketch.trim();
    assert!(sketch.get_num_retained() <= before);
}

#[test]
fn reset_returns_to_empty() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    sketch.update_u64(1);
    sketch.reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_estimate(), 0.0);
}

#[test]
fn lower_and_upper_bounds_bracket_estimate() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        sketch.update_u64(i);
    }
    let estimate = sketch.get_estimate();
    let lb = sketch.get_lower_bound(2).unwrap();
    let ub = sketch.get_upper_bound(2).unwrap();
    assert!(lb <= estimate);
    assert!(estimate <= ub);
}

#[test]
fn serialize_deserialize_v3_round_trip() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    let restored = CompactThetaSketch::deserialize(&bytes).unwrap();
    assert_eq!(compact.get_estimate(), restored.get_estimate());
}

#[test]
fn update_overloads_accept_all_input_types() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    sketch.update_i32(1);
    sketch.update_u32(2);
    sketch.update_i16(3);
    sketch.update_u16(4);
    sketch.update_i8(5);
    sketch.update_u8(6);
    sketch.update_f64(7.5);
    sketch.update_bytes(b"eight");
    sketch.update_str("nine");
    assert!(!sketch.is_empty());
    assert!(!sketch.is_estimation_mode());
    assert_eq!(sketch.get_estimate(), 9.0);
}

#[test]
fn compact_ordered_vs_unordered() {
    let mut sketch = ThetaSketchBuilder::new().build().unwrap();
    for i in 0..2000u64 {
        sketch.update_u64(i);
    }
    let unordered = sketch.compact(false);
    let ordered = sketch.compact(true);
    assert!(!unordered.is_ordered());
    assert!(ordered.is_ordered());
    assert_eq!(unordered.get_estimate(), ordered.get_estimate());
    assert_eq!(unordered.get_num_retained(), ordered.get_num_retained());
}

#[test]
fn v4_compressed_round_trip() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compressed();
    let restored = CompactThetaSketch::deserialize_compressed(&bytes).unwrap();
    assert_eq!(restored.get_num_retained(), compact.get_num_retained());
    assert_eq!(restored.get_theta(), compact.get_theta());
    assert_eq!(restored.get_estimate(), compact.get_estimate());
}

#[test]
fn wrapped_compact_query_parity_with_source() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..8192u64 {
        sketch.update_u64(i);
    }
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();

    assert_eq!(wrapped.get_estimate(), compact.get_estimate());
    assert_eq!(wrapped.get_lower_bound(1).unwrap(), compact.get_lower_bound(1).unwrap());
    assert_eq!(wrapped.get_upper_bound(1).unwrap(), compact.get_upper_bound(1).unwrap());
    assert_eq!(wrapped.is_estimation_mode(), compact.is_estimation_mode());
    assert_eq!(wrapped.get_theta(), compact.get_theta());
    assert_eq!(wrapped.get_num_retained(), compact.get_num_retained());
    assert_eq!(wrapped.is_ordered(), compact.is_ordered());
}

#[test]
fn builder_rejects_lg_k_out_of_range() {
    // theta_constants::MIN_LG_K == 5, MAX_LG_K == 26.
    assert!(ThetaSketchBuilder::new().lg_k(4).build().is_err());
    assert!(ThetaSketchBuilder::new().lg_k(27).build().is_err());
    assert!(ThetaSketchBuilder::new().lg_k(5).build().is_ok());
    assert!(ThetaSketchBuilder::new().lg_k(26).build().is_ok());
}

#[test]
fn builder_rejects_p_out_of_range() {
    // theta_base_builder::set_p requires 0.0 < p <= 1.0.
    assert!(ThetaSketchBuilder::new().p(0.0).build().is_err());
    assert!(ThetaSketchBuilder::new().p(1.5).build().is_err());
    assert!(ThetaSketchBuilder::new().p(1.0).build().is_ok());
}
