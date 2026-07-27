#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("cpc_sketch_shim.h");

        type CpcSketchShim;

        fn new_cpc_sketch(lg_k: u8) -> Result<UniquePtr<CpcSketchShim>>;
        fn cpc_sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<CpcSketchShim>>;
        fn cpc_sketch_max_serialized_size_bytes(lg_k: u8) -> Result<usize>;
        fn cpc_init() -> Result<()>;

        fn update_u64(self: Pin<&mut CpcSketchShim>, value: u64);
        fn update_i64(self: Pin<&mut CpcSketchShim>, value: i64);
        fn update_u32(self: Pin<&mut CpcSketchShim>, value: u32);
        fn update_i32(self: Pin<&mut CpcSketchShim>, value: i32);
        fn update_u16(self: Pin<&mut CpcSketchShim>, value: u16);
        fn update_i16(self: Pin<&mut CpcSketchShim>, value: i16);
        fn update_u8(self: Pin<&mut CpcSketchShim>, value: u8);
        fn update_i8(self: Pin<&mut CpcSketchShim>, value: i8);
        fn update_f64(self: Pin<&mut CpcSketchShim>, value: f64);
        fn update_f32(self: Pin<&mut CpcSketchShim>, value: f32);
        fn update_str(self: Pin<&mut CpcSketchShim>, value: &str);
        fn update_bytes(self: Pin<&mut CpcSketchShim>, value: &[u8]);

        fn is_empty(self: &CpcSketchShim) -> bool;
        fn get_estimate(self: &CpcSketchShim) -> f64;
        fn get_lower_bound(self: &CpcSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &CpcSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_lg_k(self: &CpcSketchShim) -> u8;
        fn to_string_summary(self: &CpcSketchShim) -> String;

        fn serialize(self: &CpcSketchShim) -> Vec<u8>;
    }
}
