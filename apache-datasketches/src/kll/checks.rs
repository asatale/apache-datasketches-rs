//! Argument validation shared by the sketch, the sorted view and the
//! Kolmogorov–Smirnov functions. It runs before crossing FFI, so callers get
//! a typed [`SketchError::InvalidConfig`] instead of a C++ exception message.

use super::KllItem;
use crate::error::SketchError;

/// Upstream `kll_constants::MIN_K`. The maximum, 65535, is `u16::MAX`, so the
/// type already enforces it.
pub(crate) const MIN_K: u16 = 8;

pub(crate) fn check_k(k: u16) -> Result<(), SketchError> {
    if k < MIN_K {
        return Err(SketchError::InvalidConfig(format!(
            "k must be in {MIN_K}..=65535, got {k}"
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

pub(crate) fn check_item<T: KllItem>(item: T) -> Result<(), SketchError> {
    if T::is_nan_item(item) {
        return Err(SketchError::InvalidConfig("item must not be NaN".into()));
    }
    Ok(())
}

pub(crate) fn check_split_points<T: KllItem>(split_points: &[T]) -> Result<(), SketchError> {
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
