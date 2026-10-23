//! The Kolmogorov–Smirnov test over two KLL sketches: do they look like
//! samples of the same distribution?
//!
//! All three functions return [`SketchError::EmptySketch`] if either sketch is
//! empty. Upstream returns `delta = 0`/`test = false` for two empty sketches,
//! `delta = 1` for one, and a NaN or infinite threshold, none of which mean
//! anything; an error is more useful than a silent "not different".

use super::checks::check_p;
use super::{KllItem, KllSketch};
use crate::error::SketchError;

/// The raw delta: the largest gap between the two sketches' approximate
/// CDFs, in `[0, 1]`.
pub fn kolmogorov_smirnov_delta<T: KllItem>(
    a: &KllSketch<T>,
    b: &KllSketch<T>,
) -> Result<f64, SketchError> {
    require_non_empty(a, b)?;
    Ok(T::ks_delta(a.shim(), b.shim()))
}

/// The threshold `delta` must exceed to reject, at p-value `p` (typically
/// 0.001 to 0.1), that the sketches share a distribution. It is widened by
/// both sketches' rank error. `p` must be in `(0, 1)`.
pub fn kolmogorov_smirnov_threshold<T: KllItem>(
    a: &KllSketch<T>,
    b: &KllSketch<T>,
    p: f64,
) -> Result<f64, SketchError> {
    require_non_empty(a, b)?;
    check_p(p)?;
    Ok(T::ks_threshold(a.shim(), b.shim(), p))
}

/// `true` if the null hypothesis — that both sketches reflect the same
/// distribution — can be rejected at p-value `p`, i.e. if
/// [`kolmogorov_smirnov_delta`] exceeds [`kolmogorov_smirnov_threshold`].
/// Sketches too small to tell return `false`.
pub fn kolmogorov_smirnov_test<T: KllItem>(
    a: &KllSketch<T>,
    b: &KllSketch<T>,
    p: f64,
) -> Result<bool, SketchError> {
    require_non_empty(a, b)?;
    check_p(p)?;
    Ok(T::ks_test(a.shim(), b.shim(), p))
}

fn require_non_empty<T: KllItem>(a: &KllSketch<T>, b: &KllSketch<T>) -> Result<(), SketchError> {
    if a.is_empty() || b.is_empty() {
        return Err(SketchError::EmptySketch);
    }
    Ok(())
}
