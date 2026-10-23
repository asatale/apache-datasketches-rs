//! cxx bridge for `req_sketch<float>` and its sorted view.
//!
//! Keep identical to `req_doubles.rs` modulo `f64`/`f32` and
//! `Doubles`/`Floats`: both declare the one C++ template in
//! `cpp/req/req_sketch_shim.h`. Free functions carry the `req_floats_`
//! prefix because cxx keys their symbols on name alone (see AGENTS.md);
//! methods need no prefix.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("req_sketch_shim.h");

        type ReqFloatsSketchShim;
        type ReqFloatsSortedViewShim;

        fn req_floats_new(k: u16, hra: bool) -> Result<UniquePtr<ReqFloatsSketchShim>>;
        fn req_floats_deserialize(bytes: &[u8]) -> Result<UniquePtr<ReqFloatsSketchShim>>;
        fn req_floats_rse(k: u16, rank: f64, hra: bool, n: u64) -> f64;

        fn clone_shim(self: &ReqFloatsSketchShim) -> UniquePtr<ReqFloatsSketchShim>;
        fn update(self: Pin<&mut ReqFloatsSketchShim>, item: f32);
        fn merge(self: Pin<&mut ReqFloatsSketchShim>, other: &ReqFloatsSketchShim) -> Result<()>;

        fn is_empty(self: &ReqFloatsSketchShim) -> bool;
        fn get_n(self: &ReqFloatsSketchShim) -> u64;
        fn get_k(self: &ReqFloatsSketchShim) -> u16;
        fn get_num_retained(self: &ReqFloatsSketchShim) -> u32;
        fn is_estimation_mode(self: &ReqFloatsSketchShim) -> bool;
        fn is_hra(self: &ReqFloatsSketchShim) -> bool;
        fn get_min_item(self: &ReqFloatsSketchShim) -> Result<f32>;
        fn get_max_item(self: &ReqFloatsSketchShim) -> Result<f32>;

        fn get_quantile(self: &ReqFloatsSketchShim, rank: f64, inclusive: bool) -> Result<f32>;
        fn get_rank(self: &ReqFloatsSketchShim, item: f32, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &ReqFloatsSketchShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &ReqFloatsSketchShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_rank_lower_bound(self: &ReqFloatsSketchShim, rank: f64, num_std_dev: u8) -> f64;
        fn get_rank_upper_bound(self: &ReqFloatsSketchShim, rank: f64, num_std_dev: u8) -> f64;

        fn serialize(self: &ReqFloatsSketchShim) -> UniquePtr<CxxVector<u8>>;
        fn to_string_summary(self: &ReqFloatsSketchShim) -> String;

        fn retained_items(self: &ReqFloatsSketchShim) -> UniquePtr<CxxVector<f32>>;
        fn retained_weights(self: &ReqFloatsSketchShim) -> UniquePtr<CxxVector<u64>>;
        fn sorted_view(self: &ReqFloatsSketchShim) -> UniquePtr<ReqFloatsSortedViewShim>;

        fn size(self: &ReqFloatsSortedViewShim) -> usize;
        fn get_quantile(self: &ReqFloatsSortedViewShim, rank: f64, inclusive: bool) -> Result<f32>;
        fn get_rank(self: &ReqFloatsSortedViewShim, item: f32, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &ReqFloatsSortedViewShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &ReqFloatsSortedViewShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn items(self: &ReqFloatsSortedViewShim) -> UniquePtr<CxxVector<f32>>;
        fn weights(self: &ReqFloatsSortedViewShim) -> UniquePtr<CxxVector<u64>>;
        fn cumulative_weights(self: &ReqFloatsSortedViewShim) -> UniquePtr<CxxVector<u64>>;
    }
}
