//! The `extern "C++"` bridge for the generic Tuple intersection shim.
//!
//! Nothing here mentions `RustSummary`, so like the union bridge this gets its
//! own file and aliases the two sketch shim types from [`crate::tuple_generic`]
//! the ordinary `extern "C++"` way.
//!
//! `get_result` is declared `Result<..>` because upstream throws when it is
//! called before any `update`: that state is the infinite "universe", not an
//! empty result.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");
        include!("tuple_generic_compact_shim.h");
        include!("tuple_generic_intersection_shim.h");

        type TupleGenericSketchShim = crate::tuple_generic::ffi::TupleGenericSketchShim;
        type CompactTupleGenericSketchShim =
            crate::tuple_generic::ffi::CompactTupleGenericSketchShim;

        type TupleGenericIntersectionShim;

        fn new_tuple_generic_intersection() -> UniquePtr<TupleGenericIntersectionShim>;

        fn update_with_sketch(
            self: Pin<&mut TupleGenericIntersectionShim>,
            sketch: &TupleGenericSketchShim,
        );
        fn update_with_compact(
            self: Pin<&mut TupleGenericIntersectionShim>,
            sketch: &CompactTupleGenericSketchShim,
        );

        fn get_result(
            self: &TupleGenericIntersectionShim,
            ordered: bool,
        ) -> Result<UniquePtr<CompactTupleGenericSketchShim>>;
        fn has_result(self: &TupleGenericIntersectionShim) -> bool;
    }
}
