//! Helpers shared by the KLL integration tests. Each test file pulls this in
//! with `#[macro_use] mod kll_common;`.

// Not every test file uses every helper.
#![allow(dead_code, unused_macros)]

use apache_datasketches::kll::KllItem;

/// The item types under test, with the lossy conversions the tests need.
/// Every value the tests feed through `from_f64` is an integer below 2^24,
/// so it is exact in `f32` too.
pub trait TestItem: KllItem {
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

/// Deterministic standard-normal sampler (splitmix64 + Box–Muller), so the
/// statistical tests need no `rand` dependency and never vary run to run on
/// the input side. (KLL's own compaction is still randomized.)
pub struct Normal {
    state: u64,
    spare: Option<f64>,
}

impl Normal {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in the open interval (0, 1), so `ln` below never sees 0.
    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub fn sample(&mut self) -> f64 {
        if let Some(s) = self.spare.take() {
            return s;
        }
        let r = (-2.0 * self.uniform().ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * self.uniform();
        self.spare = Some(r * theta.sin());
        r * theta.cos()
    }
}
