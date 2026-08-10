#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");
        include!("array_of_doubles_a_not_b_shim.h");

        type ArrayOfDoublesSketchShim =
            crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim =
            crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;

        type ArrayOfDoublesAnotBShim;

        fn new_array_of_doubles_a_not_b() -> UniquePtr<ArrayOfDoublesAnotBShim>;

        fn compute_sketch_sketch(
            self: &ArrayOfDoublesAnotBShim,
            a: &ArrayOfDoublesSketchShim,
            b: &ArrayOfDoublesSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn compute_sketch_compact(
            self: &ArrayOfDoublesAnotBShim,
            a: &ArrayOfDoublesSketchShim,
            b: &CompactArrayOfDoublesSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn compute_compact_sketch(
            self: &ArrayOfDoublesAnotBShim,
            a: &CompactArrayOfDoublesSketchShim,
            b: &ArrayOfDoublesSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn compute_compact_compact(
            self: &ArrayOfDoublesAnotBShim,
            a: &CompactArrayOfDoublesSketchShim,
            b: &CompactArrayOfDoublesSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
    }
}
