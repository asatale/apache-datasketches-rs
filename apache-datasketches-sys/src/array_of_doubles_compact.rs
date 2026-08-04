#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");

        type ArrayOfDoublesSketchShim =
            crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;

        type CompactArrayOfDoublesSketchShim;

        fn array_of_doubles_sketch_compact(
            sketch: &ArrayOfDoublesSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
        fn compact_array_of_doubles_sketch_deserialize(
            bytes: &[u8],
        ) -> Result<UniquePtr<CompactArrayOfDoublesSketchShim>>;

        fn get_estimate(self: &CompactArrayOfDoublesSketchShim) -> f64;
        fn get_lower_bound(self: &CompactArrayOfDoublesSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &CompactArrayOfDoublesSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &CompactArrayOfDoublesSketchShim) -> bool;
        fn is_estimation_mode(self: &CompactArrayOfDoublesSketchShim) -> bool;
        fn is_ordered(self: &CompactArrayOfDoublesSketchShim) -> bool;
        fn get_theta(self: &CompactArrayOfDoublesSketchShim) -> f64;
        fn get_num_retained(self: &CompactArrayOfDoublesSketchShim) -> u32;
        fn get_num_values(self: &CompactArrayOfDoublesSketchShim) -> u8;

        fn entry_hashes(self: &CompactArrayOfDoublesSketchShim) -> Vec<u64>;
        fn entry_values(self: &CompactArrayOfDoublesSketchShim) -> Vec<f64>;

        fn serialize(self: &CompactArrayOfDoublesSketchShim) -> Vec<u8>;
    }
}
