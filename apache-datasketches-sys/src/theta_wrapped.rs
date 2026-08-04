#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_wrapped_shim.h");

        type WrappedCompactThetaSketchShim;

        fn wrapped_compact_theta_sketch_wrap(
            bytes: &[u8],
        ) -> Result<UniquePtr<WrappedCompactThetaSketchShim>>;

        fn get_estimate(self: &WrappedCompactThetaSketchShim) -> f64;
        fn get_lower_bound(self: &WrappedCompactThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &WrappedCompactThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &WrappedCompactThetaSketchShim) -> bool;
        fn is_estimation_mode(self: &WrappedCompactThetaSketchShim) -> bool;
        fn is_ordered(self: &WrappedCompactThetaSketchShim) -> bool;
        fn get_theta(self: &WrappedCompactThetaSketchShim) -> f64;
        fn get_num_retained(self: &WrappedCompactThetaSketchShim) -> u32;
    }
}
