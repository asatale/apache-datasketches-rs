//! The `extern "C++"` bridge for the generic Tuple union shim.
//!
//! Nothing here mentions `RustSummary`, so unlike the sketch and compact
//! shims this gets its own bridge file and aliases the two sketch shim types
//! from [`crate::tuple_generic`] the ordinary `extern "C++"` way.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");
        include!("tuple_generic_compact_shim.h");
        include!("tuple_generic_union_shim.h");

        type TupleGenericSketchShim = crate::tuple_generic::ffi::TupleGenericSketchShim;
        type CompactTupleGenericSketchShim =
            crate::tuple_generic::ffi::CompactTupleGenericSketchShim;

        type TupleGenericUnionShim;

        fn new_tuple_generic_union(
            lg_k: u8,
            rf: u8,
            p: f32,
        ) -> Result<UniquePtr<TupleGenericUnionShim>>;

        fn update_with_sketch(
            self: Pin<&mut TupleGenericUnionShim>,
            sketch: &TupleGenericSketchShim,
        );
        fn update_with_compact(
            self: Pin<&mut TupleGenericUnionShim>,
            sketch: &CompactTupleGenericSketchShim,
        );

        fn get_result(
            self: &TupleGenericUnionShim,
            ordered: bool,
        ) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn reset(self: Pin<&mut TupleGenericUnionShim>);
    }
}
