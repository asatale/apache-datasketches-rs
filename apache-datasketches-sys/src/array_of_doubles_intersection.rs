#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");
        include!("array_of_doubles_intersection_shim.h");

        type ArrayOfDoublesSketchShim =
            crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim =
            crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;

        type ArrayOfDoublesIntersectionShim;

        fn new_array_of_doubles_intersection(
            num_values: u8,
        ) -> UniquePtr<ArrayOfDoublesIntersectionShim>;

        fn update_with_sketch(
            self: Pin<&mut ArrayOfDoublesIntersectionShim>,
            sketch: &ArrayOfDoublesSketchShim,
        );
        fn update_with_compact(
            self: Pin<&mut ArrayOfDoublesIntersectionShim>,
            sketch: &CompactArrayOfDoublesSketchShim,
        );

        fn get_result(
            self: &ArrayOfDoublesIntersectionShim,
            ordered: bool,
        ) -> Result<UniquePtr<CompactArrayOfDoublesSketchShim>>;
        fn has_result(self: &ArrayOfDoublesIntersectionShim) -> bool;
    }
}
