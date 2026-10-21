//! cxx bridge for `kll_sketch<float>` and its sorted view.
//!
//! Keep identical to `kll_doubles.rs` modulo `f64`/`f32` and
//! `Doubles`/`Floats`: both declare the one C++ template in
//! `cpp/kll/kll_sketch_shim.h`. Free functions carry the `kll_floats_`
//! prefix because cxx keys their symbols on name alone (see AGENTS.md);
//! methods need no prefix.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("kll_sketch_shim.h");

        type KllFloatsSketchShim;
        type KllFloatsSortedViewShim;

        fn kll_floats_new(k: u16) -> Result<UniquePtr<KllFloatsSketchShim>>;
        fn kll_floats_deserialize(bytes: &[u8]) -> Result<UniquePtr<KllFloatsSketchShim>>;
        fn kll_floats_normalized_rank_error(k: u16, pmf: bool) -> f64;

        fn clone_shim(self: &KllFloatsSketchShim) -> UniquePtr<KllFloatsSketchShim>;
        fn update(self: Pin<&mut KllFloatsSketchShim>, item: f32);
        fn merge(self: Pin<&mut KllFloatsSketchShim>, other: &KllFloatsSketchShim);

        fn is_empty(self: &KllFloatsSketchShim) -> bool;
        fn get_n(self: &KllFloatsSketchShim) -> u64;
        fn get_k(self: &KllFloatsSketchShim) -> u16;
        fn get_num_retained(self: &KllFloatsSketchShim) -> u32;
        fn is_estimation_mode(self: &KllFloatsSketchShim) -> bool;
        fn get_min_item(self: &KllFloatsSketchShim) -> Result<f32>;
        fn get_max_item(self: &KllFloatsSketchShim) -> Result<f32>;

        fn get_quantile(self: &KllFloatsSketchShim, rank: f64, inclusive: bool) -> Result<f32>;
        fn get_rank(self: &KllFloatsSketchShim, item: f32, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &KllFloatsSketchShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &KllFloatsSketchShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_normalized_rank_error(self: &KllFloatsSketchShim, pmf: bool) -> f64;

        fn serialize(self: &KllFloatsSketchShim) -> UniquePtr<CxxVector<u8>>;
        fn to_string_summary(self: &KllFloatsSketchShim) -> String;

        fn retained_items(self: &KllFloatsSketchShim) -> UniquePtr<CxxVector<f32>>;
        fn retained_weights(self: &KllFloatsSketchShim) -> UniquePtr<CxxVector<u64>>;
        fn sorted_view(self: &KllFloatsSketchShim) -> UniquePtr<KllFloatsSortedViewShim>;

        fn ks_delta(self: &KllFloatsSketchShim, other: &KllFloatsSketchShim) -> f64;
        fn ks_threshold(self: &KllFloatsSketchShim, other: &KllFloatsSketchShim, p: f64) -> f64;
        fn ks_test(self: &KllFloatsSketchShim, other: &KllFloatsSketchShim, p: f64) -> bool;

        fn size(self: &KllFloatsSortedViewShim) -> usize;
        fn get_quantile(self: &KllFloatsSortedViewShim, rank: f64, inclusive: bool) -> Result<f32>;
        fn get_rank(self: &KllFloatsSortedViewShim, item: f32, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &KllFloatsSortedViewShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &KllFloatsSortedViewShim,
            split_points: &[f32],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn items(self: &KllFloatsSortedViewShim) -> UniquePtr<CxxVector<f32>>;
        fn weights(self: &KllFloatsSortedViewShim) -> UniquePtr<CxxVector<u64>>;
        fn cumulative_weights(self: &KllFloatsSortedViewShim) -> UniquePtr<CxxVector<u64>>;
    }
}
