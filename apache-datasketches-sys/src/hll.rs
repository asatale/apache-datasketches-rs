#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TargetHllType {
        Hll4,
        Hll6,
        Hll8,
    }

    unsafe extern "C++" {
        include!("hll_sketch_shim.h");

        type HllSketchShim;

        fn new_hll_sketch(lg_config_k: u8, tgt_type: TargetHllType) -> Result<UniquePtr<HllSketchShim>>;
        fn hll_sketch_copy_as(sketch: &HllSketchShim, tgt_type: TargetHllType) -> UniquePtr<HllSketchShim>;
        fn hll_sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<HllSketchShim>>;

        fn update_u64(self: Pin<&mut HllSketchShim>, value: u64);
        fn update_i64(self: Pin<&mut HllSketchShim>, value: i64);
        fn update_f64(self: Pin<&mut HllSketchShim>, value: f64);
        fn update_str(self: Pin<&mut HllSketchShim>, value: &str);
        fn update_bytes(self: Pin<&mut HllSketchShim>, value: &[u8]);

        fn get_estimate(self: &HllSketchShim) -> f64;
        fn get_lower_bound(self: &HllSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &HllSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_lg_config_k(self: &HllSketchShim) -> u8;
        fn get_target_type(self: &HllSketchShim) -> TargetHllType;
        fn is_empty(self: &HllSketchShim) -> bool;
        fn reset(self: Pin<&mut HllSketchShim>);
        fn to_string_summary(self: &HllSketchShim) -> String;

        fn serialize_compact(self: &HllSketchShim) -> Vec<u8>;
        fn serialize_updatable(self: &HllSketchShim) -> Vec<u8>;
    }
}
