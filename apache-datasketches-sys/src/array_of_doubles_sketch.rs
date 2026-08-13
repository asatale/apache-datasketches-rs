#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    /// Named `TupleResizeFactor` rather than `ResizeFactor` because cxx emits
    /// one C++ definition per shared type into the bridge namespace, and the
    /// theta bridge already emits `apache_datasketches_rs::ResizeFactor`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TupleResizeFactor {
        X1,
        X2,
        X4,
        X8,
    }

    unsafe extern "C++" {
        include!("array_of_doubles_sketch_shim.h");
        include!("array_of_doubles_compact_shim.h");

        type ArrayOfDoublesSketchShim;
        type CompactArrayOfDoublesSketchShim =
            crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;

        fn new_array_of_doubles_sketch(
            lg_k: u8,
            rf: TupleResizeFactor,
            p: f32,
            num_values: u8,
        ) -> Result<UniquePtr<ArrayOfDoublesSketchShim>>;

        fn update_u64(self: Pin<&mut ArrayOfDoublesSketchShim>, key: u64, values: &[f64]);
        fn update_i64(self: Pin<&mut ArrayOfDoublesSketchShim>, key: i64, values: &[f64]);
        fn update_u32(self: Pin<&mut ArrayOfDoublesSketchShim>, key: u32, values: &[f64]);
        fn update_i32(self: Pin<&mut ArrayOfDoublesSketchShim>, key: i32, values: &[f64]);
        fn update_u16(self: Pin<&mut ArrayOfDoublesSketchShim>, key: u16, values: &[f64]);
        fn update_i16(self: Pin<&mut ArrayOfDoublesSketchShim>, key: i16, values: &[f64]);
        fn update_u8(self: Pin<&mut ArrayOfDoublesSketchShim>, key: u8, values: &[f64]);
        fn update_i8(self: Pin<&mut ArrayOfDoublesSketchShim>, key: i8, values: &[f64]);
        fn update_f64(self: Pin<&mut ArrayOfDoublesSketchShim>, key: f64, values: &[f64]);
        fn update_str(self: Pin<&mut ArrayOfDoublesSketchShim>, key: &str, values: &[f64]);
        fn update_bytes(self: Pin<&mut ArrayOfDoublesSketchShim>, key: &[u8], values: &[f64]);

        fn trim(self: Pin<&mut ArrayOfDoublesSketchShim>);
        fn reset(self: Pin<&mut ArrayOfDoublesSketchShim>);

        fn get_estimate(self: &ArrayOfDoublesSketchShim) -> f64;
        fn get_lower_bound(self: &ArrayOfDoublesSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &ArrayOfDoublesSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &ArrayOfDoublesSketchShim) -> bool;
        fn is_estimation_mode(self: &ArrayOfDoublesSketchShim) -> bool;
        fn is_ordered(self: &ArrayOfDoublesSketchShim) -> bool;
        fn get_theta(self: &ArrayOfDoublesSketchShim) -> f64;
        fn get_num_retained(self: &ArrayOfDoublesSketchShim) -> u32;
        fn get_num_values(self: &ArrayOfDoublesSketchShim) -> u8;

        fn entry_hashes(self: &ArrayOfDoublesSketchShim) -> UniquePtr<CxxVector<u64>>;
        fn entry_values(self: &ArrayOfDoublesSketchShim) -> UniquePtr<CxxVector<f64>>;

        fn compact(
            self: &ArrayOfDoublesSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactArrayOfDoublesSketchShim>;
    }
}
