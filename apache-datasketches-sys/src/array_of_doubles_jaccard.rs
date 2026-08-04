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

        // Named `tuple_jaccard_*` rather than `jaccard_*` because cxx derives
        // each function's extern "C" trampoline symbol from just the C++
        // namespace and function name (not the argument types or the
        // originating bridge module), so a same-named free function declared
        // in the theta bridge (see `theta_jaccard.rs`) would silently
        // collide at link time when both bridges are compiled into the same
        // binary (i.e. under `--all-features`) — the two overloads are
        // distinguishable to C++ but not to cxx's trampoline naming, and the
        // linker picks one definition for both call sites, causing the
        // wrong shim type to be reinterpreted at runtime.
        fn tuple_jaccard_sketch_sketch(
            a: &ArrayOfDoublesSketchShim,
            b: &ArrayOfDoublesSketchShim,
        ) -> TupleJaccardBoundsFfi;
        fn tuple_jaccard_sketch_compact(
            a: &ArrayOfDoublesSketchShim,
            b: &CompactArrayOfDoublesSketchShim,
        ) -> TupleJaccardBoundsFfi;
        fn tuple_jaccard_compact_sketch(
            a: &CompactArrayOfDoublesSketchShim,
            b: &ArrayOfDoublesSketchShim,
        ) -> TupleJaccardBoundsFfi;
        fn tuple_jaccard_compact_compact(
            a: &CompactArrayOfDoublesSketchShim,
            b: &CompactArrayOfDoublesSketchShim,
        ) -> TupleJaccardBoundsFfi;
    }
}
