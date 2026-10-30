use super::{BloomFilter, DEFAULT_SEED, MAX_FILTER_SIZE_BITS, MAX_NUM_HASHES};
use crate::error::SketchError;
use apache_datasketches_sys::bloom_filter::ffi;
use std::f64::consts::LN_2;

#[derive(Clone, Copy, Debug)]
enum Sizing {
    Accuracy {
        max_distinct_items: u64,
        target_fpp: f64,
    },
    Size {
        num_bits: u64,
        num_hashes: u16,
    },
}

/// Builder for [`BloomFilter`]. Choose a sizing with [`Self::by_accuracy`]
/// or [`Self::by_size`], optionally set a [`Self::seed`], then call
/// [`Self::build`].
#[derive(Clone, Copy, Debug)]
pub struct BloomFilterBuilder {
    sizing: Sizing,
    seed: u64,
}

fn invalid(msg: impl Into<String>) -> SketchError {
    SketchError::InvalidConfig(msg.into())
}

fn check_items(max_distinct_items: u64) -> Result<(), SketchError> {
    if max_distinct_items == 0 {
        return Err(invalid("max_distinct_items must be greater than 0"));
    }
    Ok(())
}

fn check_fpp(target_fpp: f64) -> Result<(), SketchError> {
    if !(target_fpp > 0.0 && target_fpp < 1.0) {
        return Err(invalid(format!(
            "target_fpp must be in (0, 1), got {target_fpp}"
        )));
    }
    Ok(())
}

// Upstream's suggest_* (bloom_filter_builder_impl.hpp:48, :54, :61) static_cast a
// double to uint16_t / uint64_t with no range check, which is undefined
// behavior when the value does not fit, and create_by_accuracy (:70) calls
// suggest_num_filter_bits itself. These recompute the same expressions, in
// the same operation order, so out-of-range results are rejected before
// upstream runs.

/// `ceil(num_filter_bits / max_distinct_items * ln 2)`, as upstream computes
/// it (:48).
fn num_hashes_estimate(max_distinct_items: u64, num_filter_bits: u64) -> f64 {
    (num_filter_bits as f64 / max_distinct_items as f64 * LN_2).ceil()
}

/// `ceil(-max_distinct_items * ln(target_fpp) / (ln 2)^2)`, as upstream
/// computes it (:61).
fn num_filter_bits_estimate(max_distinct_items: u64, target_fpp: f64) -> f64 {
    (-(max_distinct_items as f64) * target_fpp.ln() / (LN_2 * LN_2)).ceil()
}

/// Checks the filter-size suggestion is a usable size, i.e. at most
/// [`MAX_FILTER_SIZE_BITS`] (exactly representable as an `f64`). That bound
/// is far below `u64::MAX`, so passing it also rules out the undefined cast.
fn check_num_filter_bits_estimate(
    max_distinct_items: u64,
    target_fpp: f64,
) -> Result<(), SketchError> {
    let bits = num_filter_bits_estimate(max_distinct_items, target_fpp);
    if bits > MAX_FILTER_SIZE_BITS as f64 {
        return Err(invalid(format!(
            "{max_distinct_items} items at target_fpp {target_fpp} need about {bits} bits, \
             more than the maximum filter size of {MAX_FILTER_SIZE_BITS} bits"
        )));
    }
    Ok(())
}

impl BloomFilterBuilder {
    /// Sizes the filter for `max_distinct_items` expected distinct items at a
    /// target false-positive probability `target_fpp`, which must lie
    /// strictly between 0 and 1.
    pub fn by_accuracy(max_distinct_items: u64, target_fpp: f64) -> Self {
        Self {
            sizing: Sizing::Accuracy {
                max_distinct_items,
                target_fpp,
            },
            seed: DEFAULT_SEED,
        }
    }

    /// Sizes the filter explicitly: `num_bits` bits (rounded up to a multiple
    /// of 64) and `num_hashes` hash functions, at most `u16::MAX - 1` (upstream's
    /// hashing loop never terminates for `u16::MAX`).
    pub fn by_size(num_bits: u64, num_hashes: u16) -> Self {
        Self {
            sizing: Sizing::Size {
                num_bits,
                num_hashes,
            },
            seed: DEFAULT_SEED,
        }
    }

    /// Sets the hash seed. The default is [`DEFAULT_SEED`].
    ///
    /// Filters must share a seed (and the same size and number of hash
    /// functions) to be combined with `union_with` or `intersect`.
    ///
    /// Security note: a known seed lets an adversary craft inputs that
    /// collide. In adversarial settings, use a random seed.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Validates the configuration and builds an empty [`BloomFilter`].
    ///
    /// Returns [`SketchError::InvalidConfig`] if an argument is out of range,
    /// including a [`Self::by_accuracy`] sizing that would need more than the
    /// maximum filter size (see [`Self::suggest_num_filter_bits`]).
    pub fn build(self) -> Result<BloomFilter, SketchError> {
        let inner = match self.sizing {
            Sizing::Accuracy {
                max_distinct_items,
                target_fpp,
            } => {
                check_items(max_distinct_items)?;
                check_fpp(target_fpp)?;
                check_num_filter_bits_estimate(max_distinct_items, target_fpp)?;
                ffi::bloom_new_by_accuracy(max_distinct_items, target_fpp, self.seed)
            }
            Sizing::Size {
                num_bits,
                num_hashes,
            } => {
                if num_bits == 0 || num_bits > MAX_FILTER_SIZE_BITS {
                    return Err(invalid(format!(
                        "num_bits must be in 1..={MAX_FILTER_SIZE_BITS}, got {num_bits}"
                    )));
                }
                if num_hashes == 0 || num_hashes > MAX_NUM_HASHES {
                    return Err(invalid(format!(
                        "num_hashes must be in 1..={MAX_NUM_HASHES}, got {num_hashes}"
                    )));
                }
                ffi::bloom_new_by_size(num_bits, num_hashes, self.seed)
            }
        }
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(BloomFilter { inner })
    }

    /// Suggests the number of hash functions for `max_distinct_items` items in
    /// a filter of `num_filter_bits` bits: `ceil(num_filter_bits /
    /// max_distinct_items * ln 2)`.
    ///
    /// Returns [`SketchError::InvalidConfig`] if an argument is out of range
    /// or the suggestion exceeds `u16::MAX - 1`, the most hash functions a
    /// filter can use (many bits for few items).
    pub fn suggest_num_hashes(
        max_distinct_items: u64,
        num_filter_bits: u64,
    ) -> Result<u16, SketchError> {
        check_items(max_distinct_items)?;
        if num_filter_bits == 0 || num_filter_bits > MAX_FILTER_SIZE_BITS {
            return Err(invalid(format!(
                "num_filter_bits must be in 1..={MAX_FILTER_SIZE_BITS}, got {num_filter_bits}"
            )));
        }
        let hashes = num_hashes_estimate(max_distinct_items, num_filter_bits);
        if hashes > f64::from(MAX_NUM_HASHES) {
            return Err(invalid(format!(
                "{num_filter_bits} bits for {max_distinct_items} items suggest {hashes} hash \
                 functions, more than the maximum of {MAX_NUM_HASHES}"
            )));
        }
        ffi::bloom_suggest_num_hashes(max_distinct_items, num_filter_bits)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Suggests the number of hash functions for a target false-positive
    /// probability: `ceil(-ln(target_fpp) / ln 2)`.
    ///
    /// Returns [`SketchError::InvalidConfig`] unless `target_fpp` lies
    /// strictly between 0 and 1. (Within that range the suggestion is at most
    /// about 1074, at the smallest positive `f64`, so it always fits.)
    pub fn suggest_num_hashes_for_fpp(target_fpp: f64) -> Result<u16, SketchError> {
        check_fpp(target_fpp)?;
        ffi::bloom_suggest_num_hashes_for_fpp(target_fpp)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Suggests the number of filter bits for `max_distinct_items` items at a
    /// target false-positive probability: `ceil(-max_distinct_items *
    /// ln(target_fpp) / (ln 2)^2)`.
    ///
    /// Returns [`SketchError::InvalidConfig`] if an argument is out of range
    /// or the suggestion exceeds the maximum filter size of
    /// `(i32::MAX - 32) * 8` bits (upstream cannot build a larger filter).
    pub fn suggest_num_filter_bits(
        max_distinct_items: u64,
        target_fpp: f64,
    ) -> Result<u64, SketchError> {
        check_items(max_distinct_items)?;
        check_fpp(target_fpp)?;
        check_num_filter_bits_estimate(max_distinct_items, target_fpp)?;
        ffi::bloom_suggest_num_filter_bits(max_distinct_items, target_fpp)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }
}
