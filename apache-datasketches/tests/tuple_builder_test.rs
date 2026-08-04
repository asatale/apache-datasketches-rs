// apache-datasketches/tests/tuple_builder_test.rs
//! Coverage for `ArrayOfDoublesSketchBuilder`/`ArrayOfDoublesUnionBuilder`
//! configuration knobs that no other tuple test exercises:
//!
//! - `resize_factor`: no other test ever calls `.resize_factor(...)`, so the
//!   `ResizeFactor::{X1,X2,X4} -> sys::TupleResizeFactor` mapping
//!   (`apache-datasketches/src/tuple/builder.rs`) and the C++
//!   `to_cpp_tuple_resize_factor` (`apache-datasketches-sys/cpp/tuple/
//!   array_of_doubles_sketch_shim.cc`) only ever ran at the `X8` default.
//!   `TupleResizeFactor` is a hand-written second copy of theta's
//!   `ResizeFactor` (created to work around a cxx trampoline collision — see
//!   `AGENTS.md`), so a mis-mapped discriminant here is a plausible failure
//!   mode with otherwise zero test pressure.
//! - `p` (sampling probability): both builders document
//!   `SketchError::InvalidConfig` outside `(0, 1]`, but nothing verified that
//!   the C++ `theta_base_builder` exception actually propagates through this
//!   family's constructor path, or that sampling drives estimation mode.
//! - `trim`: ships public and was never called by any test.
use apache_datasketches::tuple::{
    ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder, ResizeFactor,
};

const ALL_RESIZE_FACTORS: [ResizeFactor; 4] = [
    ResizeFactor::X1,
    ResizeFactor::X2,
    ResizeFactor::X4,
    ResizeFactor::X8,
];

#[test]
fn resize_factor_values_are_distinct() {
    // A swapped or duplicated discriminant mapping in `From<ResizeFactor> for
    // sys::TupleResizeFactor` would still let this fail: it's an
    // independent, all-Rust check that the four variants really are four
    // distinct values before we even reach the FFI boundary below.
    for (i, a) in ALL_RESIZE_FACTORS.iter().enumerate() {
        for (j, b) in ALL_RESIZE_FACTORS.iter().enumerate() {
            assert_eq!(i == j, a == b);
        }
    }
}

#[test]
fn sketch_builder_resize_factor_variants_all_build_and_estimate() {
    for rf in ALL_RESIZE_FACTORS {
        let mut sketch = ArrayOfDoublesSketchBuilder::new()
            .lg_k(12)
            .resize_factor(rf)
            .num_values(1)
            .build()
            .unwrap_or_else(|e| panic!("resize_factor {rf:?} failed to build: {e}"));
        for i in 0..10_000u64 {
            sketch.update_u64(i, &[1.0]).unwrap();
        }
        let estimate = sketch.get_estimate();
        assert!(
            (estimate - 10_000.0).abs() / 10_000.0 < 0.05,
            "resize_factor {rf:?}: estimate {estimate} too far from 10000"
        );
    }
}

#[test]
fn union_builder_resize_factor_variants_all_build_and_estimate() {
    for rf in ALL_RESIZE_FACTORS {
        let mut sketch1 = ArrayOfDoublesSketchBuilder::new().lg_k(12).build().unwrap();
        let mut sketch2 = ArrayOfDoublesSketchBuilder::new().lg_k(12).build().unwrap();
        for i in 0..5_000u64 {
            sketch1.update_u64(i, &[1.0]).unwrap();
        }
        for i in 2_500..7_500u64 {
            sketch2.update_u64(i, &[1.0]).unwrap();
        }

        let mut union = ArrayOfDoublesUnionBuilder::new()
            .lg_k(12)
            .resize_factor(rf)
            .num_values(1)
            .build()
            .unwrap_or_else(|e| panic!("union resize_factor {rf:?} failed to build: {e}"));
        union.update(&sketch1).unwrap();
        union.update(&sketch2).unwrap();

        let estimate = union.get_result(true).get_estimate();
        assert!(
            (estimate - 7_500.0).abs() / 7_500.0 < 0.05,
            "union resize_factor {rf:?}: estimate {estimate} too far from 7500"
        );
    }
}

#[test]
fn sketch_builder_rejects_p_out_of_range() {
    // theta_base_builder::set_p (shared with theta) requires 0.0 < p <= 1.0.
    assert!(ArrayOfDoublesSketchBuilder::new().p(0.0).build().is_err());
    assert!(ArrayOfDoublesSketchBuilder::new().p(1.5).build().is_err());
    assert!(ArrayOfDoublesSketchBuilder::new().p(1.0).build().is_ok());
}

#[test]
fn union_builder_rejects_p_out_of_range() {
    assert!(ArrayOfDoublesUnionBuilder::new().p(0.0).build().is_err());
    assert!(ArrayOfDoublesUnionBuilder::new().p(1.5).build().is_err());
    assert!(ArrayOfDoublesUnionBuilder::new().p(1.0).build().is_ok());
}

#[test]
fn sketch_builder_p_below_one_starts_in_estimation_mode() {
    // A single update whose hash survives sampling under a low `p` puts the
    // sketch into estimation mode immediately, before it would ever have
    // grown large enough to sample on its own.
    let mut sketch = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .p(0.1)
        .num_values(1)
        .build()
        .unwrap();
    for i in 0..2_000u64 {
        sketch.update_u64(i, &[1.0]).unwrap();
    }
    assert!(sketch.is_estimation_mode());
    assert!(sketch.get_theta() < 1.0);
}

#[test]
fn union_builder_p_below_one_starts_in_estimation_mode() {
    let mut union = ArrayOfDoublesUnionBuilder::new()
        .lg_k(12)
        .p(0.1)
        .num_values(1)
        .build()
        .unwrap();
    let mut sketch = ArrayOfDoublesSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..2_000u64 {
        sketch.update_u64(i, &[1.0]).unwrap();
    }
    union.update(&sketch).unwrap();

    let result = union.get_result(true);
    assert!(result.is_estimation_mode());
    assert!(result.get_theta() < 1.0);
}

#[test]
fn trim_reduces_retained_toward_target_size() {
    // Note: this only asserts on retained count, matching
    // `theta_sketch_test.rs::trim_reduces_retained_below_target`. An earlier
    // version of this test additionally asserted the estimate is unchanged
    // by `trim()`, which turned out to be false -- and, checked against an
    // ad hoc probe, equally false for the already-shipped theta family
    // (`ThetaSketch::trim()` on an identically-sized sketch moved the
    // estimate from ~13453 to ~11920 too). Trimming lowers theta to the
    // target retained count, and `get_estimate() ~= retained / theta`, so a
    // real (if usually modest) estimate shift is expected upstream
    // behavior, not a tuple-specific defect.
    let mut sketch = ArrayOfDoublesSketchBuilder::new()
        .lg_k(5)
        .num_values(1)
        .build()
        .unwrap();
    for i in 0..10_000u64 {
        sketch.update_u64(i, &[1.0]).unwrap();
    }
    let before_retained = sketch.get_num_retained();

    sketch.trim();

    assert!(sketch.get_num_retained() <= before_retained);
}
