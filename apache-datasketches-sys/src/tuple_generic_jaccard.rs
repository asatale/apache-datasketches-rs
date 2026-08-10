#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    /// Distinct from the ArrayOfDoubles bridge's `TupleJaccardBoundsFfi`:
    /// cxx emits one C++ definition per shared type into the bridge
    /// namespace, and names must be globally unique across bridges.
    struct TupleGenericJaccardBoundsFfi {
        lower_bound: f64,
        estimate: f64,
        upper_bound: f64,
    }

    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");
        include!("tuple_generic_compact_shim.h");
        include!("tuple_generic_jaccard_shim.h");

        type TupleGenericSketchShim = crate::tuple_generic::ffi::TupleGenericSketchShim;
        type CompactTupleGenericSketchShim =
            crate::tuple_generic::ffi::CompactTupleGenericSketchShim;

        fn tuple_generic_jaccard_sketch_sketch(
            a: &TupleGenericSketchShim,
            b: &TupleGenericSketchShim,
        ) -> TupleGenericJaccardBoundsFfi;
        fn tuple_generic_jaccard_sketch_compact(
            a: &TupleGenericSketchShim,
            b: &CompactTupleGenericSketchShim,
        ) -> TupleGenericJaccardBoundsFfi;
        fn tuple_generic_jaccard_compact_sketch(
            a: &CompactTupleGenericSketchShim,
            b: &TupleGenericSketchShim,
        ) -> TupleGenericJaccardBoundsFfi;
        fn tuple_generic_jaccard_compact_compact(
            a: &CompactTupleGenericSketchShim,
            b: &CompactTupleGenericSketchShim,
        ) -> TupleGenericJaccardBoundsFfi;
    }
}
