//! Argument validation shared by the sketch and the sorted view. It runs
//! before crossing FFI, so callers get a typed [`SketchError::InvalidConfig`]
//! instead of a C++ exception message.

use super::ReqItem;
use crate::error::SketchError;

/// Upstream `req_constants::MIN_K`.
pub(crate) const MIN_K: u16 = 4;
/// Upstream documents `k` up to 1024 (`req_sketch.hpp`), but its constructor
/// computes `std::max<uint8_t>(k & -2, MIN_K)`, which truncates `k` to 8 bits:
/// `k = 256` silently becomes 4 and `k = 300` becomes 44. 254 is the largest
/// even `k` that survives, so it is the largest this crate accepts.
pub(crate) const MAX_K: u16 = 254;

/// Upstream documents `k` as even and in `[4, 1024]` but silently rounds an
/// odd `k` down and clamps below 4, and corrupts `k >= 256` (see `MAX_K`);
/// this enforces the contract that actually holds.
pub(crate) fn check_k(k: u16) -> Result<(), SketchError> {
    if !(MIN_K..=MAX_K).contains(&k) || !k.is_multiple_of(2) {
        return Err(SketchError::InvalidConfig(format!(
            "k must be even and in {MIN_K}..={MAX_K}, got {k}"
        )));
    }
    Ok(())
}

/// `k` for the static error math and for deserialized sketches: even and in
/// the documented `4..=1024`. Neither path goes through the truncating
/// constructor, so the full documented range is valid here.
pub(crate) fn check_k_static(k: u16) -> Result<(), SketchError> {
    if !(MIN_K..=1024).contains(&k) || !k.is_multiple_of(2) {
        return Err(SketchError::InvalidConfig(format!(
            "k must be even and in {MIN_K}..=1024, got {k}"
        )));
    }
    Ok(())
}

pub(crate) fn check_rank(rank: f64) -> Result<(), SketchError> {
    // `contains` is false for NaN, so this rejects it too.
    if !(0.0..=1.0).contains(&rank) {
        return Err(SketchError::InvalidConfig(format!(
            "rank must be in [0, 1], got {rank}"
        )));
    }
    Ok(())
}

pub(crate) fn check_num_std_dev(num_std_dev: u8) -> Result<(), SketchError> {
    if !(1..=3).contains(&num_std_dev) {
        return Err(SketchError::InvalidConfig(format!(
            "num_std_dev must be 1, 2 or 3, got {num_std_dev}"
        )));
    }
    Ok(())
}

pub(crate) fn check_item<T: ReqItem>(item: T) -> Result<(), SketchError> {
    if T::is_nan_item(item) {
        return Err(SketchError::InvalidConfig("item must not be NaN".into()));
    }
    Ok(())
}

pub(crate) fn check_split_points<T: ReqItem>(split_points: &[T]) -> Result<(), SketchError> {
    if u32::try_from(split_points.len()).is_err() {
        return Err(SketchError::InvalidConfig(format!(
            "at most {} split points are supported, got {}",
            u32::MAX,
            split_points.len()
        )));
    }
    if split_points.iter().any(|&p| T::is_nan_item(p)) {
        return Err(SketchError::InvalidConfig(
            "split points must not contain NaN".into(),
        ));
    }
    if !split_points.windows(2).all(|w| w[0] < w[1]) {
        return Err(SketchError::InvalidConfig(
            "split points must be unique and strictly increasing".into(),
        ));
    }
    Ok(())
}
