//! cxx bridge for `kll_sketch<double>` and its sorted view.
//!
//! Keep identical to `kll_floats.rs` modulo `f64`/`f32` and
//! `Doubles`/`Floats`: both declare the one C++ template in
//! `cpp/kll/kll_sketch_shim.h`. Free functions carry the `kll_doubles_`
//! prefix because cxx keys their symbols on name alone (see AGENTS.md);
//! methods need no prefix.

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("kll_sketch_shim.h");

        type KllDoublesSketchShim;
        type KllDoublesSortedViewShim;

        fn kll_doubles_new(k: u16) -> Result<UniquePtr<KllDoublesSketchShim>>;
        fn kll_doubles_deserialize(bytes: &[u8]) -> Result<UniquePtr<KllDoublesSketchShim>>;
        fn kll_doubles_normalized_rank_error(k: u16, pmf: bool) -> f64;

        fn clone_shim(self: &KllDoublesSketchShim) -> UniquePtr<KllDoublesSketchShim>;
        fn update(self: Pin<&mut KllDoublesSketchShim>, item: f64);
        fn merge(self: Pin<&mut KllDoublesSketchShim>, other: &KllDoublesSketchShim);

        fn is_empty(self: &KllDoublesSketchShim) -> bool;
        fn get_n(self: &KllDoublesSketchShim) -> u64;
        fn get_k(self: &KllDoublesSketchShim) -> u16;
        fn get_num_retained(self: &KllDoublesSketchShim) -> u32;
        fn is_estimation_mode(self: &KllDoublesSketchShim) -> bool;
        fn get_min_item(self: &KllDoublesSketchShim) -> Result<f64>;
        fn get_max_item(self: &KllDoublesSketchShim) -> Result<f64>;

        fn get_quantile(self: &KllDoublesSketchShim, rank: f64, inclusive: bool) -> Result<f64>;
        fn get_rank(self: &KllDoublesSketchShim, item: f64, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &KllDoublesSketchShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &KllDoublesSketchShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_normalized_rank_error(self: &KllDoublesSketchShim, pmf: bool) -> f64;

        fn serialize(self: &KllDoublesSketchShim) -> UniquePtr<CxxVector<u8>>;
        fn to_string_summary(self: &KllDoublesSketchShim) -> String;

        fn retained_items(self: &KllDoublesSketchShim) -> UniquePtr<CxxVector<f64>>;
        fn retained_weights(self: &KllDoublesSketchShim) -> UniquePtr<CxxVector<u64>>;
        fn sorted_view(self: &KllDoublesSketchShim) -> UniquePtr<KllDoublesSortedViewShim>;

        fn ks_delta(self: &KllDoublesSketchShim, other: &KllDoublesSketchShim) -> f64;
        fn ks_threshold(self: &KllDoublesSketchShim, other: &KllDoublesSketchShim, p: f64) -> f64;
        fn ks_test(self: &KllDoublesSketchShim, other: &KllDoublesSketchShim, p: f64) -> bool;

        fn size(self: &KllDoublesSortedViewShim) -> usize;
        fn get_quantile(self: &KllDoublesSortedViewShim, rank: f64, inclusive: bool)
            -> Result<f64>;
        fn get_rank(self: &KllDoublesSortedViewShim, item: f64, inclusive: bool) -> Result<f64>;
        fn get_pmf(
            self: &KllDoublesSortedViewShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn get_cdf(
            self: &KllDoublesSortedViewShim,
            split_points: &[f64],
            inclusive: bool,
        ) -> Result<UniquePtr<CxxVector<f64>>>;
        fn items(self: &KllDoublesSortedViewShim) -> UniquePtr<CxxVector<f64>>;
        fn weights(self: &KllDoublesSortedViewShim) -> UniquePtr<CxxVector<u64>>;
        fn cumulative_weights(self: &KllDoublesSortedViewShim) -> UniquePtr<CxxVector<u64>>;
    }
}
