//! The item types a [`KllSketch`](super::KllSketch) can hold, and the
//! per-type dispatch to the matching cxx bridge. This is the only file that
//! knows there are two bridges; everything else is written once against
//! `T::…`.

use apache_datasketches_sys::{kll_doubles::ffi as doubles, kll_floats::ffi as floats};
use cxx::{Exception, UniquePtr};
use std::fmt::Debug;
use std::pin::Pin;

/// An item type a [`KllSketch`](super::KllSketch) can hold: `f64` or `f32`.
///
/// Sealed: it cannot be implemented outside this crate, because each
/// implementation is backed by its own explicitly instantiated C++ sketch.
/// Both types match a Java class (`KllDoublesSketch`, `KllFloatsSketch`), so
/// serialized bytes are readable by the Java and Python DataSketches
/// libraries.
pub trait KllItem: sealed::Sealed + Copy + PartialOrd + Debug + Send + Sync + 'static {}

impl KllItem for f64 {}
impl KllItem for f32 {}

pub(crate) mod sealed {
    use super::*;

    /// The per-type FFI surface. Unnameable outside the crate, which is what
    /// seals [`KllItem`](super::KllItem).
    pub trait Sealed: Sized {
        type Sketch: cxx::memory::UniquePtrTarget;

        fn is_nan_item(item: Self) -> bool;

        fn sketch_new(k: u16) -> Result<UniquePtr<Self::Sketch>, Exception>;
        fn sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<Self::Sketch>, Exception>;
        fn normalized_rank_error_for_k(k: u16, pmf: bool) -> f64;

        fn clone_shim(s: &Self::Sketch) -> UniquePtr<Self::Sketch>;
        fn update(s: Pin<&mut Self::Sketch>, item: Self);
        fn merge(s: Pin<&mut Self::Sketch>, other: &Self::Sketch);
        fn is_empty(s: &Self::Sketch) -> bool;
        fn get_n(s: &Self::Sketch) -> u64;
        fn get_k(s: &Self::Sketch) -> u16;
        fn get_num_retained(s: &Self::Sketch) -> u32;
        fn is_estimation_mode(s: &Self::Sketch) -> bool;
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
        fn get_normalized_rank_error(s: &Self::Sketch, pmf: bool) -> f64;
        fn serialize(s: &Self::Sketch) -> Vec<u8>;
        fn to_string_summary(s: &Self::Sketch) -> String;
        fn retained_items(s: &Self::Sketch) -> Vec<Self>;
        fn retained_weights(s: &Self::Sketch) -> Vec<u64>;
    }
}

macro_rules! impl_sealed {
    ($t:ty, $m:ident, $sketch:ident, $new:ident, $deser:ident, $rank_err:ident) => {
        impl sealed::Sealed for $t {
            type Sketch = $m::$sketch;

            fn is_nan_item(item: Self) -> bool {
                item.is_nan()
            }

            fn sketch_new(k: u16) -> Result<UniquePtr<Self::Sketch>, Exception> {
                $m::$new(k)
            }
            fn sketch_deserialize(bytes: &[u8]) -> Result<UniquePtr<Self::Sketch>, Exception> {
                $m::$deser(bytes)
            }
            fn normalized_rank_error_for_k(k: u16, pmf: bool) -> f64 {
                $m::$rank_err(k, pmf)
            }

            fn clone_shim(s: &Self::Sketch) -> UniquePtr<Self::Sketch> {
                s.clone_shim()
            }
            fn update(s: Pin<&mut Self::Sketch>, item: Self) {
                s.update(item)
            }
            fn merge(s: Pin<&mut Self::Sketch>, other: &Self::Sketch) {
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
            fn get_normalized_rank_error(s: &Self::Sketch, pmf: bool) -> f64 {
                s.get_normalized_rank_error(pmf)
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
        }
    };
}

impl_sealed!(
    f64,
    doubles,
    KllDoublesSketchShim,
    kll_doubles_new,
    kll_doubles_deserialize,
    kll_doubles_normalized_rank_error
);
impl_sealed!(
    f32,
    floats,
    KllFloatsSketchShim,
    kll_floats_new,
    kll_floats_deserialize,
    kll_floats_normalized_rank_error
);
