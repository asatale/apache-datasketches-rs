//! The item types a [`ReqSketch`](super::ReqSketch) can hold, and the
//! per-type dispatch to the matching cxx bridge. This is the only file that
//! knows there are two bridges; everything else is written once against
//! `T::…`.

use apache_datasketches_sys::{req_doubles::ffi as doubles, req_floats::ffi as floats};
use cxx::{Exception, UniquePtr};
use std::fmt::Debug;
use std::pin::Pin;

/// An item type a [`ReqSketch`](super::ReqSketch) can hold: `f64` or `f32`.
///
/// Sealed: it cannot be implemented outside this crate, because each
/// implementation is backed by its own explicitly instantiated C++ sketch.
/// `f32` bytes interoperate with Java's `ReqSketch` and Python's float REQ
/// sketch, while `f64` bytes are readable only by C++ and this crate.
pub trait ReqItem: sealed::Sealed + Copy + PartialOrd + Debug + Send + Sync + 'static {}

impl ReqItem for f64 {}
impl ReqItem for f32 {}

pub(crate) mod sealed {
    use super::*;

    /// The per-type FFI surface. Unnameable outside the crate, which is what
    /// seals [`ReqItem`](super::ReqItem).
    pub trait Sealed: Sized {
        type Sketch: cxx::memory::UniquePtrTarget;
        type View: cxx::memory::UniquePtrTarget;

        fn is_nan_item(item: Self) -> bool;

        fn sketch_new(k: u16, hra: bool) -> Result<UniquePtr<Self::Sketch>, Exception>;
        fn sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<Self::Sketch>, Exception>;
        fn rse(k: u16, rank: f64, hra: bool, n: u64) -> f64;

        fn clone_shim(s: &Self::Sketch) -> UniquePtr<Self::Sketch>;
        fn update(s: Pin<&mut Self::Sketch>, item: Self);
        fn merge(s: Pin<&mut Self::Sketch>, other: &Self::Sketch) -> Result<(), Exception>;
        fn is_empty(s: &Self::Sketch) -> bool;
        fn get_n(s: &Self::Sketch) -> u64;
        fn get_k(s: &Self::Sketch) -> u16;
        fn get_num_retained(s: &Self::Sketch) -> u32;
        fn is_estimation_mode(s: &Self::Sketch) -> bool;
        fn is_hra(s: &Self::Sketch) -> bool;
        fn get_min_item(s: &Self::Sketch) -> Result<Self, Exception>;
        fn get_max_item(s: &Self::Sketch) -> Result<Self, Exception>;
        fn get_quantile(s: &Self::Sketch, rank: f64, inclusive: bool) -> Result<Self, Exception>;
        fn get_rank(s: &Self::Sketch, item: Self, inclusive: bool) -> Result<f64, Exception>;
        fn get_pmf(
            s: &Self::Sketch,
            split_points: &[Self],
            inclusive: bool,
        ) -> Result<Vec<f64>, Exception>;
        fn get_cdf(
            s: &Self::Sketch,
            split_points: &[Self],
            inclusive: bool,
        ) -> Result<Vec<f64>, Exception>;
        fn rank_lower_bound(s: &Self::Sketch, rank: f64, num_std_dev: u8) -> f64;
        fn rank_upper_bound(s: &Self::Sketch, rank: f64, num_std_dev: u8) -> f64;
        fn serialize(s: &Self::Sketch) -> Vec<u8>;
        fn to_string_summary(s: &Self::Sketch) -> String;
        fn retained_items(s: &Self::Sketch) -> Vec<Self>;
        fn retained_weights(s: &Self::Sketch) -> Vec<u64>;

        fn sorted_view(s: &Self::Sketch) -> UniquePtr<Self::View>;
        fn view_size(v: &Self::View) -> usize;
        fn view_get_quantile(v: &Self::View, rank: f64, inclusive: bool)
            -> Result<Self, Exception>;
        fn view_get_rank(v: &Self::View, item: Self, inclusive: bool) -> Result<f64, Exception>;
        fn view_get_pmf(
            v: &Self::View,
            split_points: &[Self],
            inclusive: bool,
        ) -> Result<Vec<f64>, Exception>;
        fn view_get_cdf(
            v: &Self::View,
            split_points: &[Self],
            inclusive: bool,
        ) -> Result<Vec<f64>, Exception>;
        fn view_items(v: &Self::View) -> Vec<Self>;
        fn view_weights(v: &Self::View) -> Vec<u64>;
        fn view_cumulative_weights(v: &Self::View) -> Vec<u64>;
    }
}

macro_rules! impl_sealed {
    ($t:ty, $m:ident, $sketch:ident, $view:ident, $new:ident, $deser:ident, $rse:ident) => {
        impl sealed::Sealed for $t {
            type Sketch = $m::$sketch;
            type View = $m::$view;

            fn is_nan_item(item: Self) -> bool {
                item.is_nan()
            }

            fn sketch_new(k: u16, hra: bool) -> Result<UniquePtr<Self::Sketch>, Exception> {
                $m::$new(k, hra)
            }
            fn sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<Self::Sketch>, Exception> {
                $m::$deser(bytes)
            }
            fn rse(k: u16, rank: f64, hra: bool, n: u64) -> f64 {
                $m::$rse(k, rank, hra, n)
            }

            fn clone_shim(s: &Self::Sketch) -> UniquePtr<Self::Sketch> {
                s.clone_shim()
            }
            fn update(s: Pin<&mut Self::Sketch>, item: Self) {
                s.update(item)
            }
            fn merge(s: Pin<&mut Self::Sketch>, other: &Self::Sketch) -> Result<(), Exception> {
                s.merge(other)
            }
            fn is_empty(s: &Self::Sketch) -> bool {
                s.is_empty()
            }
            fn get_n(s: &Self::Sketch) -> u64 {
                s.get_n()
            }
            fn get_k(s: &Self::Sketch) -> u16 {
                s.get_k()
            }
            fn get_num_retained(s: &Self::Sketch) -> u32 {
                s.get_num_retained()
            }
            fn is_estimation_mode(s: &Self::Sketch) -> bool {
                s.is_estimation_mode()
            }
            fn is_hra(s: &Self::Sketch) -> bool {
                s.is_hra()
            }
            fn get_min_item(s: &Self::Sketch) -> Result<Self, Exception> {
                s.get_min_item()
            }
            fn get_max_item(s: &Self::Sketch) -> Result<Self, Exception> {
                s.get_max_item()
            }
            fn get_quantile(
                s: &Self::Sketch,
                rank: f64,
                inclusive: bool,
            ) -> Result<Self, Exception> {
                s.get_quantile(rank, inclusive)
            }
            fn get_rank(s: &Self::Sketch, item: Self, inclusive: bool) -> Result<f64, Exception> {
                s.get_rank(item, inclusive)
            }
            fn get_pmf(
                s: &Self::Sketch,
                split_points: &[Self],
                inclusive: bool,
            ) -> Result<Vec<f64>, Exception> {
                s.get_pmf(split_points, inclusive)
                    .map(|v| v.as_slice().to_vec())
            }
            fn get_cdf(
                s: &Self::Sketch,
                split_points: &[Self],
                inclusive: bool,
            ) -> Result<Vec<f64>, Exception> {
                s.get_cdf(split_points, inclusive)
                    .map(|v| v.as_slice().to_vec())
            }
            fn rank_lower_bound(s: &Self::Sketch, rank: f64, num_std_dev: u8) -> f64 {
                s.get_rank_lower_bound(rank, num_std_dev)
            }
            fn rank_upper_bound(s: &Self::Sketch, rank: f64, num_std_dev: u8) -> f64 {
                s.get_rank_upper_bound(rank, num_std_dev)
            }
            fn serialize(s: &Self::Sketch) -> Vec<u8> {
                s.serialize().as_slice().to_vec()
            }
            fn to_string_summary(s: &Self::Sketch) -> String {
                s.to_string_summary()
            }
            fn retained_items(s: &Self::Sketch) -> Vec<Self> {
                s.retained_items().as_slice().to_vec()
            }
            fn retained_weights(s: &Self::Sketch) -> Vec<u64> {
                s.retained_weights().as_slice().to_vec()
            }

            fn sorted_view(s: &Self::Sketch) -> UniquePtr<Self::View> {
                s.sorted_view()
            }
            fn view_size(v: &Self::View) -> usize {
                v.size()
            }
            fn view_get_quantile(
                v: &Self::View,
                rank: f64,
                inclusive: bool,
            ) -> Result<Self, Exception> {
                v.get_quantile(rank, inclusive)
            }
            fn view_get_rank(
                v: &Self::View,
                item: Self,
                inclusive: bool,
            ) -> Result<f64, Exception> {
                v.get_rank(item, inclusive)
            }
            fn view_get_pmf(
                v: &Self::View,
                split_points: &[Self],
                inclusive: bool,
            ) -> Result<Vec<f64>, Exception> {
                v.get_pmf(split_points, inclusive)
                    .map(|r| r.as_slice().to_vec())
            }
            fn view_get_cdf(
                v: &Self::View,
                split_points: &[Self],
                inclusive: bool,
            ) -> Result<Vec<f64>, Exception> {
                v.get_cdf(split_points, inclusive)
                    .map(|r| r.as_slice().to_vec())
            }
            fn view_items(v: &Self::View) -> Vec<Self> {
                v.items().as_slice().to_vec()
            }
            fn view_weights(v: &Self::View) -> Vec<u64> {
                v.weights().as_slice().to_vec()
            }
            fn view_cumulative_weights(v: &Self::View) -> Vec<u64> {
                v.cumulative_weights().as_slice().to_vec()
            }
        }
    };
}

impl_sealed!(
    f64,
    doubles,
    ReqDoublesSketchShim,
    ReqDoublesSortedViewShim,
    req_doubles_new,
    req_doubles_deserialize,
    req_doubles_rse
);
impl_sealed!(
    f32,
    floats,
    ReqFloatsSketchShim,
    ReqFloatsSortedViewShim,
    req_floats_new,
    req_floats_deserialize,
    req_floats_rse
);
