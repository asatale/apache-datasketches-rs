//! cxx bridge for `req_sketch<double>` and its sorted view.
//!
//! Keep identical to `req_floats.rs` modulo `f64`/`f32` and
//! `Doubles`/`Floats`: both declare the one C++ template in
//! `cpp/req/req_sketch_shim.h`. Free functions carry the `req_doubles_`
//! prefix because cxx keys their symbols on name alone (see AGENTS.md);
//! methods need no prefix.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("req_sketch_shim.h");

        type ReqDoublesSketchShim;
        type ReqDoublesSortedViewShim;

        fn req_doubles_new(k: u16, hra: bool) -> Result<UniquePtr<ReqDoublesSketchShim>>;
        fn req_doubles_deserialize(bytes: &[u8]) -> Result<UniquePtr<ReqDoublesSketchShim>>;
        fn req_doubles_a_priori_rank_error(k: u16, rank: f64, hra: bool, n: u64) -> f64;

        fn clone_shim(self: &ReqDoublesSketchShim) -> UniquePtr<ReqDoublesSketchShim>;
        fn update(self: Pin<&mut ReqDoublesSketchShim>, item: f64);
        fn merge(self: Pin<&mut ReqDoublesSketchShim>, other: &ReqDoublesSketchShim) -> Result<()>;

        fn is_empty(self: &ReqDoublesSketchShim) -> bool;
        fn get_n(self: &ReqDoublesSketchShim) -> u64;
        fn get_k(self: &ReqDoublesSketchShim) -> u16;
        fn get_num_retained(self: &ReqDoublesSketchShim) -> u32;
        fn is_estimation_mode(self: &ReqDoublesSketchShim) -> bool;
        fn is_hra(self: &ReqDoublesSketchShim) -> bool;
        fn get_min_item(self: &ReqDoublesSketchShim) -> Result<f64>;
        fn get_max_item(self: &ReqDoublesSketchShim) -> Result<f64>;

        fn get_quantile(self: &ReqDoublesSketchShim, rank: f64, inclusive: bool) -> Result<f64>;
        fn get_rank(self: &ReqDoublesSketchShim, item: f64, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &ReqDoublesSketchShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &ReqDoublesSketchShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_rank_lower_bound(self: &ReqDoublesSketchShim, rank: f64, num_std_dev: u8) -> f64;
        fn get_rank_upper_bound(self: &ReqDoublesSketchShim, rank: f64, num_std_dev: u8) -> f64;

        fn serialize(self: &ReqDoublesSketchShim) -> UniquePtr<CxxVector<u8>>;
        fn to_string_summary(self: &ReqDoublesSketchShim) -> String;

        fn retained_items(self: &ReqDoublesSketchShim) -> UniquePtr<CxxVector<f64>>;
        fn retained_weights(self: &ReqDoublesSketchShim) -> UniquePtr<CxxVector<u64>>;
        fn sorted_view(self: &ReqDoublesSketchShim) -> UniquePtr<ReqDoublesSortedViewShim>;

        fn size(self: &ReqDoublesSortedViewShim) -> usize;
        fn get_quantile(self: &ReqDoublesSortedViewShim, rank: f64, inclusive: bool)
            -> Result<f64>;
        fn get_rank(self: &ReqDoublesSortedViewShim, item: f64, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &ReqDoublesSortedViewShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &ReqDoublesSortedViewShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn items(self: &ReqDoublesSortedViewShim) -> UniquePtr<CxxVector<f64>>;
        fn weights(self: &ReqDoublesSortedViewShim) -> UniquePtr<CxxVector<u64>>;
        fn cumulative_weights(self: &ReqDoublesSortedViewShim) -> UniquePtr<CxxVector<u64>>;
    }
}
