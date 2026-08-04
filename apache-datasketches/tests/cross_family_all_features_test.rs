// apache-datasketches/tests/cross_family_all_features_test.rs
//!
//! Why this test exists (do not delete it as redundant with the per-family
//! jaccard/a-not-b tests):
//!
//! Every other test file in this crate compiles to its own binary and
//! exercises exactly one sketch family, so no other test file both `use`s
//! `theta::` and `tuple::` in the same compiled artifact. That mattered
//! because of a real bug on this branch: the theta and tuple jaccard C++
//! shims both declared free functions named `jaccard_sketch_sketch` (and
//! three siblings) inside the same `#[cxx::bridge(namespace =
//! "apache_datasketches_rs")]` namespace. cxx derives the generated `extern
//! "C"` trampoline symbol for a free function from the namespace and
//! function name alone — not from parameter types, and not from which
//! bridge module declared it — so both bridges emitted the identical
//! symbol. The linker silently picked one definition for both call sites,
//! which only manifested as a SIGBUS crash under `--all-features`
//! (`cargo test --workspace --all-features`), because that is the one
//! configuration where both bridges land in the same binary. No
//! single-family test could ever have caught it, and building each feature
//! combination separately never links both bridges together either.
//!
//! The original bug was fixed by renaming the tuple side to
//! `tuple_jaccard_*` (see `apache-datasketches-sys/src/array_of_doubles_jaccard.rs`)
//! and is now additionally guarded by a build-time uniqueness check in
//! `apache-datasketches-sys/build.rs`. This test is the last line of
//! defense against the same collision reappearing through a different
//! route: `ThetaAnotBShim::compute_sketch_sketch` and
//! `ArrayOfDoublesAnotBShim::compute_sketch_sketch` are *methods* today, so
//! they are safe (the receiver type is part of a method's trampoline
//! symbol) — but a refactor that turned either into a free function would
//! reintroduce this exact bug, silently, with no compiler or linker error.
//! By exercising both families' Jaccard similarity *and* both families'
//! a-not-b in one binary, a reintroduced collision shows up here as a wrong
//! numeric result or a crash, in a test explicitly designed to catch it,
//! rather than as an unexplained flake somewhere else under
//! `--all-features`.
#![cfg(all(feature = "theta", feature = "tuple"))]

use apache_datasketches::theta::{
    jaccard_similarity as theta_jaccard_similarity, ThetaAnotB, ThetaSketchBuilder,
};
use apache_datasketches::tuple::{
    array_of_doubles_jaccard_similarity, ArrayOfDoublesAnotB, ArrayOfDoublesSketchBuilder,
};

#[test]
fn theta_and_tuple_jaccard_similarity_in_the_same_binary() {
    let mut theta_a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut theta_b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        theta_a.update_u64(i);
    }
    for i in 5_000..15_000u64 {
        theta_b.update_u64(i);
    }
    let theta_result = theta_jaccard_similarity(&theta_a, &theta_b);
    // 5000 common / 15000 union ~= 0.333.
    assert!(
        (theta_result.estimate - 1.0 / 3.0).abs() < 0.05,
        "theta jaccard estimate {} too far from 1/3",
        theta_result.estimate
    );
    assert!(theta_result.lower_bound <= theta_result.estimate);
    assert!(theta_result.estimate <= theta_result.upper_bound);

    let mut tuple_a = ArrayOfDoublesSketchBuilder::new().lg_k(12).build().unwrap();
    let mut tuple_b = ArrayOfDoublesSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..10_000u64 {
        tuple_a.update_u64(i, &[1.0]).unwrap();
    }
    for i in 5_000..15_000u64 {
        tuple_b.update_u64(i, &[1.0]).unwrap();
    }
    let tuple_result = array_of_doubles_jaccard_similarity(&tuple_a, &tuple_b).unwrap();
    assert!(
        (tuple_result.estimate - 1.0 / 3.0).abs() < 0.05,
        "tuple jaccard estimate {} too far from 1/3",
        tuple_result.estimate
    );
    assert!(tuple_result.lower_bound <= tuple_result.estimate);
    assert!(tuple_result.estimate <= tuple_result.upper_bound);

    // If the two families' jaccard trampolines had collided, one of the two
    // computations above would silently reinterpret the wrong shim type and
    // produce a nonsensical estimate (or crash) rather than ~1/3 for both.
}

#[test]
fn theta_and_tuple_a_not_b_in_the_same_binary() {
    let mut theta_a = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    let mut theta_b = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        theta_a.update_u64(i);
    }
    for i in 500..1500u64 {
        theta_b.update_u64(i);
    }
    let theta_a_not_b = ThetaAnotB::new();
    let theta_result = theta_a_not_b.compute(&theta_a, &theta_b, true);
    assert_eq!(theta_result.get_estimate(), 500.0);

    let mut tuple_a = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    let mut tuple_b = ArrayOfDoublesSketchBuilder::new().build().unwrap();
    for i in 0..1000i32 {
        tuple_a.update_i32(i, &[1.0]).unwrap();
    }
    for i in 500..1500i32 {
        tuple_b.update_i32(i, &[1.0]).unwrap();
    }
    let tuple_a_not_b = ArrayOfDoublesAnotB::new();
    let tuple_result = tuple_a_not_b.compute(&tuple_a, &tuple_b, true).unwrap();
    assert_eq!(tuple_result.get_estimate(), 500.0);

    // As above: a collided `compute_sketch_sketch` trampoline would corrupt
    // one or both of these results instead of both landing on exactly 500.
}
