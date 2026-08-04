// apache-datasketches/tests/theta_setop_test.rs
//! Ported from theta/test/theta_setop_test.cpp (tag 5.2.0): the 4x4 = 16
//! pairwise combinations of {Empty, Exact, Estimation, Degenerate} sketch
//! states run through union/intersection/a-not-b.
use apache_datasketches::theta::{
    ResizeFactor, ThetaAnotB, ThetaIntersection, ThetaSketch, ThetaSketchBuilder, ThetaUnionBuilder,
};

const LG_K: u8 = 5;
const GT_MIDP_V: u64 = 3;
// MIDP is one of the upstream constants confirmed from theta_setop_test.cpp
// (used there to vary `p` for one operand across a couple of the 16 cases);
// this port always uses LOWP for Estimation/Degenerate (see build_sketch),
// so MIDP itself is unused here but kept for parity with the verified
// upstream constant set.
#[allow(dead_code)]
const MIDP: f32 = 0.5;
const GT_LOWP_V: u64 = 6;
const LOWP: f32 = 0.1;
const LT_LOWP_V: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
enum SkType {
    Empty,
    Exact,
    Estimation,
    Degenerate,
}

const ALL_TYPES: [SkType; 4] = [
    SkType::Empty,
    SkType::Exact,
    SkType::Estimation,
    SkType::Degenerate,
];

fn build_sketch(ty: SkType) -> ThetaSketch {
    match ty {
        SkType::Empty => ThetaSketchBuilder::new().lg_k(LG_K).build().unwrap(),
        SkType::Exact => {
            // p = 1.0 (no sampling): a single update always yields theta ==
            // 1.0, 1 retained entry, non-empty. The specific value doesn't
            // matter here (unlike Estimation/Degenerate below, where the
            // value's hash relative to theta determines whether it survives
            // sampling), so we reuse upstream's GT_MIDP_V for parity with
            // the verified constants.
            let mut s = ThetaSketchBuilder::new().lg_k(LG_K).build().unwrap();
            s.update_u64(GT_MIDP_V);
            s
        }
        SkType::Estimation => {
            // p = LOWP forces theta < 1. LT_LOWP_V is upstream's
            // pre-verified value whose hash (under the default seed) falls
            // *below* the LOWP threshold, so it survives sampling: theta <
            // 1, 1 retained entry, non-empty, estimation mode. A single
            // update() call, not a loop, matches upstream's
            // build_sketch(ESTIMATION, LOWP, LT_LOWP_V).
            let mut s = ThetaSketchBuilder::new()
                .lg_k(LG_K)
                .resize_factor(ResizeFactor::X1)
                .p(LOWP)
                .build()
                .unwrap();
            s.update_u64(LT_LOWP_V);
            s
        }
        SkType::Degenerate => {
            // p = LOWP forces theta < 1. GT_LOWP_V is upstream's
            // pre-verified value whose hash (under the default seed) falls
            // *above* the LOWP threshold, so it is rejected by sampling:
            // theta < 1, 0 retained entries, non-empty sketch (empty *set*
            // but theta < 1 means is_empty() is false). A single update()
            // call, not a loop, matches upstream's
            // build_sketch(DEGENERATE, LOWP, GT_LOWP_V). Using a loop with
            // sequential small values here (as an earlier draft of this
            // port did) is non-deterministic with respect to which values'
            // hashes fall above/below theta and does not reliably produce
            // zero retained entries.
            let mut s = ThetaSketchBuilder::new()
                .lg_k(LG_K)
                .p(LOWP)
                .build()
                .unwrap();
            s.update_u64(GT_LOWP_V);
            s
        }
    }
}

fn is_estimation_expected(ty: SkType) -> bool {
    matches!(ty, SkType::Estimation | SkType::Degenerate)
}

fn is_empty_expected(ty: SkType) -> bool {
    matches!(ty, SkType::Empty)
}

#[test]
fn degenerate_sketch_is_actually_degenerate() {
    // Guard against the Degenerate builder silently falling back to a
    // normal (theta == 1, non-empty retained set) sketch: theta must be
    // strictly less than 1, num_retained must be 0, and the sketch must
    // report estimation mode and non-empty (an empty *set*, but not an
    // empty *sketch*, since theta < 1).
    let s = build_sketch(SkType::Degenerate);
    assert!(
        s.get_theta() < 1.0,
        "expected theta < 1, got {}",
        s.get_theta()
    );
    assert_eq!(s.get_num_retained(), 0, "expected zero retained entries");
    assert!(s.is_estimation_mode());
    assert!(!s.is_empty());
}

#[test]
fn union_all_type_combinations() {
    for &a_ty in &ALL_TYPES {
        for &b_ty in &ALL_TYPES {
            let a = build_sketch(a_ty);
            let b = build_sketch(b_ty);
            let mut union_ = ThetaUnionBuilder::new().lg_k(LG_K).build().unwrap();
            union_.update(&a);
            union_.update(&b);
            let result = union_.get_result(true);

            let expect_empty = is_empty_expected(a_ty) && is_empty_expected(b_ty);
            assert_eq!(
                result.is_empty(),
                expect_empty,
                "union({:?}, {:?}).is_empty()",
                a_ty,
                b_ty
            );
            if is_estimation_expected(a_ty) || is_estimation_expected(b_ty) {
                assert!(
                    result.is_estimation_mode() || expect_empty,
                    "union({:?}, {:?}) expected estimation mode",
                    a_ty,
                    b_ty
                );
            }
        }
    }
}

#[test]
fn intersection_all_type_combinations() {
    for &a_ty in &ALL_TYPES {
        for &b_ty in &ALL_TYPES {
            let a = build_sketch(a_ty);
            let b = build_sketch(b_ty);
            let mut isect = ThetaIntersection::new();
            isect.update(&a);
            isect.update(&b);
            let result = isect.get_result(true).unwrap();

            let expect_empty = is_empty_expected(a_ty) || is_empty_expected(b_ty);
            assert_eq!(
                result.is_empty(),
                expect_empty,
                "intersection({:?}, {:?}).is_empty()",
                a_ty,
                b_ty
            );
        }
    }
}

#[test]
fn a_not_b_all_type_combinations() {
    let a_not_b = ThetaAnotB::new();
    for &a_ty in &ALL_TYPES {
        for &b_ty in &ALL_TYPES {
            let a = build_sketch(a_ty);
            let b = build_sketch(b_ty);
            let result = a_not_b.compute(&a, &b, true);

            if is_empty_expected(a_ty) {
                assert!(
                    result.is_empty(),
                    "a_not_b({:?}, {:?}) expected empty when a is empty",
                    a_ty,
                    b_ty
                );
            }
        }
    }
}
