use super::{BloomFilter, DEFAULT_SEED, MAX_FILTER_SIZE_BITS};
use crate::error::SketchError;
use apache_datasketches_sys::bloom_filter::ffi;

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
    /// of 64) and `num_hashes` hash functions.
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
    /// Returns [`SketchError::InvalidConfig`] if an argument is out of range.
    pub fn build(self) -> Result<BloomFilter, SketchError> {
        let inner = match self.sizing {
            Sizing::Accuracy {
                max_distinct_items,
                target_fpp,
            } => {
                check_items(max_distinct_items)?;
                check_fpp(target_fpp)?;
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
                if num_hashes == 0 {
                    return Err(invalid("num_hashes must be at least 1"));
                }
                ffi::bloom_new_by_size(num_bits, num_hashes, self.seed)
            }
        }
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(BloomFilter { inner })
    }

    /// Suggests the number of hash functions for `max_distinct_items` items in
    /// a filter of `num_filter_bits` bits.
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
        ffi::bloom_suggest_num_hashes(max_distinct_items, num_filter_bits)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Suggests the number of hash functions for a target false-positive
    /// probability.
    pub fn suggest_num_hashes_for_fpp(target_fpp: f64) -> Result<u16, SketchError> {
        check_fpp(target_fpp)?;
        ffi::bloom_suggest_num_hashes_for_fpp(target_fpp)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Suggests the number of filter bits for `max_distinct_items` items at a
    /// target false-positive probability.
    pub fn suggest_num_filter_bits(
        max_distinct_items: u64,
        target_fpp: f64,
    ) -> Result<u64, SketchError> {
        check_items(max_distinct_items)?;
        check_fpp(target_fpp)?;
        ffi::bloom_suggest_num_filter_bits(max_distinct_items, target_fpp)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }
}
