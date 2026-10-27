//! The `update_*` methods are not declared `Result`: upstream throws from them only on
//! a seed-hash mismatch, and no ArrayOfDoubles API exposes a seed, so every
//! sketch built here carries `DEFAULT_SEED`'s hash. Upstream's deserialize
//! checks the seed hash only for images that have entries, so
//! `compact_array_of_doubles_sketch_deserialize` rejects any other non-empty
//! image with a foreign seed hash before upstream sees it
//! (`validate_aod_seed_hash` in `cpp/tuple/array_of_doubles_compact_shim.cc`).

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
