#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;

        type CompactThetaSketchShim;

        fn theta_sketch_compact(
            sketch: &ThetaSketchShim,
            ordered: bool,
        ) -> UniquePtr<CompactThetaSketchShim>;
        fn compact_theta_sketch_deserialize(
            bytes: &[u8],
        ) -> Result<UniquePtr<CompactThetaSketchShim>>;

        fn get_estimate(self: &CompactThetaSketchShim) -> f64;
        fn get_lower_bound(self: &CompactThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &CompactThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &CompactThetaSketchShim) -> bool;
        fn is_estimation_mode(self: &CompactThetaSketchShim) -> bool;
        fn is_ordered(self: &CompactThetaSketchShim) -> bool;
        fn get_theta(self: &CompactThetaSketchShim) -> f64;
        fn get_num_retained(self: &CompactThetaSketchShim) -> u32;

        fn serialize_compact(self: &CompactThetaSketchShim) -> Vec<u8>;
        fn serialize_compressed(self: &CompactThetaSketchShim) -> Vec<u8>;
    }
}
