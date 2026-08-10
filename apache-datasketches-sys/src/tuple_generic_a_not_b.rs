//! The `extern "C++"` bridge for the generic Tuple a-not-b shim.
//!
//! Nothing here mentions `RustSummary`, so like the union and intersection
//! bridges this gets its own file and aliases the two sketch shim types from
//! [`crate::tuple_generic`] the ordinary `extern "C++"` way.
//!
//! Upstream's `compute` is a template over both operand types, so the shim
//! provides four concrete overloads; the safe wrapper dispatches over
//! `TupleGenericInputRef`. A-not-b takes no policy at all — retained entries
//! carry operand `a`'s summaries, copy-constructed.
//!
//! The `compute_*` methods are not declared `Result`: `compute` throws only on
//! a seed-hash mismatch, and no generic-Tuple API exposes a seed. Same shape
//! as [`crate::array_of_doubles_a_not_b`].

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");
        include!("tuple_generic_compact_shim.h");
        include!("tuple_generic_a_not_b_shim.h");

        type TupleGenericSketchShim = crate::tuple_generic::ffi::TupleGenericSketchShim;
        type CompactTupleGenericSketchShim =
            crate::tuple_generic::ffi::CompactTupleGenericSketchShim;

        type TupleGenericAnotBShim;

        fn new_tuple_generic_a_not_b() -> UniquePtr<TupleGenericAnotBShim>;

        fn compute_sketch_sketch(
            self: &TupleGenericAnotBShim,
            a: &TupleGenericSketchShim,
            b: &TupleGenericSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn compute_sketch_compact(
            self: &TupleGenericAnotBShim,
            a: &TupleGenericSketchShim,
            b: &CompactTupleGenericSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn compute_compact_sketch(
            self: &TupleGenericAnotBShim,
            a: &CompactTupleGenericSketchShim,
            b: &TupleGenericSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn compute_compact_compact(
            self: &TupleGenericAnotBShim,
            a: &CompactTupleGenericSketchShim,
            b: &CompactTupleGenericSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactTupleGenericSketchShim>;
    }
}
