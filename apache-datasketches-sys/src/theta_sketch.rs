#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ResizeFactor {
        X1,
        X2,
        X4,
        X8,
    }

    unsafe extern "C++" {
        include!("theta_sketch_shim.h");

        type ThetaSketchShim;

        fn new_theta_sketch(lg_k: u8, rf: ResizeFactor, p: f32) -> Result<UniquePtr<ThetaSketchShim>>;

        fn update_u64(self: Pin<&mut ThetaSketchShim>, value: u64);
        fn update_i64(self: Pin<&mut ThetaSketchShim>, value: i64);
        fn update_u32(self: Pin<&mut ThetaSketchShim>, value: u32);
        fn update_i32(self: Pin<&mut ThetaSketchShim>, value: i32);
        fn update_u16(self: Pin<&mut ThetaSketchShim>, value: u16);
        fn update_i16(self: Pin<&mut ThetaSketchShim>, value: i16);
        fn update_u8(self: Pin<&mut ThetaSketchShim>, value: u8);
        fn update_i8(self: Pin<&mut ThetaSketchShim>, value: i8);
        fn update_f64(self: Pin<&mut ThetaSketchShim>, value: f64);
        fn update_str(self: Pin<&mut ThetaSketchShim>, value: &str);
        fn update_bytes(self: Pin<&mut ThetaSketchShim>, value: &[u8]);

        fn trim(self: Pin<&mut ThetaSketchShim>);
        fn reset(self: Pin<&mut ThetaSketchShim>);

        fn get_estimate(self: &ThetaSketchShim) -> f64;
        fn get_lower_bound(self: &ThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &ThetaSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &ThetaSketchShim) -> bool;
        fn is_estimation_mode(self: &ThetaSketchShim) -> bool;
        fn is_ordered(self: &ThetaSketchShim) -> bool;
        fn get_theta(self: &ThetaSketchShim) -> f64;
        fn get_num_retained(self: &ThetaSketchShim) -> u32;
    }
}
