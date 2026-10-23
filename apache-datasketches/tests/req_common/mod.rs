//! Helpers shared by the REQ integration tests. Each test file pulls this in
//! with `#[macro_use] mod req_common;`.

// Not every test file uses every helper.
#![allow(dead_code, unused_macros)]

use apache_datasketches::req::ReqItem;

/// The item types under test, with the lossy conversions the tests need.
/// Every value the tests feed through `from_f64` is an integer below 2^24,
/// so it is exact in `f32` too.
pub trait TestItem: ReqItem {
    fn from_f64(v: f64) -> Self;
    fn to_f64(self) -> f64;
}

impl TestItem for f64 {
    fn from_f64(v: f64) -> Self {
        v
    }
    fn to_f64(self) -> f64 {
        self
    }
}

impl TestItem for f32 {
    fn from_f64(v: f64) -> Self {
        v as f32
    }
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
}

/// Generates a module holding one `#[test]` per item type, each calling the
/// generic test body `$body::<T>()`.
macro_rules! for_both_types {
    ($name:ident, $body:ident) => {
        mod $name {
            #[test]
            fn as_f64() {
                super::$body::<f64>();
            }
            #[test]
            fn as_f32() {
                super::$body::<f32>();
            }
        }
    };
}
