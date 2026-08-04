#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    /// Named `TupleJaccardBoundsFfi` rather than `JaccardBoundsFfi` because
    /// cxx emits one C++ definition per shared type into the bridge namespace,
    /// and the theta bridge already emits
    /// `apache_datasketches_rs::JaccardBoundsFfi`.
    struct TupleJaccardBoundsFfi {
        lower_bound: f64,
        estimate: f64,
        upper_bound: f64,
    }

    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");
        include!("array_of_doubles_jaccard_shim.h");

        type ArrayOfDoublesSketchShim = crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim = crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;

        fn jaccard_sketch_sketch(a: &ArrayOfDoublesSketchShim, b: &ArrayOfDoublesSketchShim) -> TupleJaccardBoundsFfi;
        fn jaccard_sketch_compact(a: &ArrayOfDoublesSketchShim, b: &CompactArrayOfDoublesSketchShim) -> TupleJaccardBoundsFfi;
        fn jaccard_compact_sketch(a: &CompactArrayOfDoublesSketchShim, b: &ArrayOfDoublesSketchShim) -> TupleJaccardBoundsFfi;
        fn jaccard_compact_compact(a: &CompactArrayOfDoublesSketchShim, b: &CompactArrayOfDoublesSketchShim) -> TupleJaccardBoundsFfi;
    }
}
