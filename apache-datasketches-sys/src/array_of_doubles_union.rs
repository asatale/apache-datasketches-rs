#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");
        include!("array_of_doubles_union_shim.h");

        type ArrayOfDoublesSketchShim =
            crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim =
            crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;
        type TupleResizeFactor = crate::array_of_doubles_sketch::ffi::TupleResizeFactor;

        type ArrayOfDoublesUnionShim;

        fn new_array_of_doubles_union(
            lg_k: u8,
            rf: TupleResizeFactor,
            p: f32,
            num_values: u8,
        ) -> Result<UniquePtr<ArrayOfDoublesUnionShim>>;

        fn update_with_sketch(
            self: Pin<&mut ArrayOfDoublesUnionShim>,
            sketch: &ArrayOfDoublesSketchShim,
        );
        fn update_with_compact(
            self: Pin<&mut ArrayOfDoublesUnionShim>,
            sketch: &CompactArrayOfDoublesSketchShim,
        );

        fn get_result(
            self: &ArrayOfDoublesUnionShim,
            ordered: bool,
        ) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn reset(self: Pin<&mut ArrayOfDoublesUnionShim>);
    }
}
