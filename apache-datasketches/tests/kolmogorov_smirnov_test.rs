//! Port of upstream kll/test/kolmogorov_smirnov_test.cpp. Upstream draws from
//! std::default_random_engine; this uses kll_common::Normal instead. The
//! assertions are statistical, so they hold for any decent normal source.

#[macro_use]
mod kll_common;

use apache_datasketches::kll::{
    kolmogorov_smirnov_delta, kolmogorov_smirnov_test, kolmogorov_smirnov_threshold, KllSketch,
};
use apache_datasketches::SketchError;
use kll_common::Normal;

/// Two sketches of size `k` fed `3k - 1` normal samples, the first shifted by
/// `shift`. Same samples on both sides, as upstream does.
fn shifted_pair(k: u16, shift: f64) -> (KllSketch<f64>, KllSketch<f64>) {
    let mut a = KllSketch::new(k).unwrap();
    let mut b = KllSketch::new(k).unwrap();
    let mut normal = Normal::new(42);
    for _ in 0..(u32::from(k) * 3 - 1) {
        let x = normal.sample();
        a.update(x + shift);
        b.update(x);
    }
    (a, b)
}

// Upstream asserts delta == 0 and test == false for two empty sketches. Those
// answers carry no information, so this binding rejects empty operands
// instead (see the spec's Errors section).
#[test]
fn empty_operands_are_rejected() {
    let empty = KllSketch::<f64>::default();
    let mut full = KllSketch::<f64>::default();
    full.update(1.0);
    for (a, b) in [(&empty, &empty), (&empty, &full), (&full, &empty)] {
        assert!(matches!(
            kolmogorov_smirnov_delta(a, b),
            Err(SketchError::EmptySketch)
        ));
        assert!(matches!(
            kolmogorov_smirnov_threshold(a, b, 0.05),
            Err(SketchError::EmptySketch)
        ));
        assert!(matches!(
            kolmogorov_smirnov_test(a, b, 0.05),
            Err(SketchError::EmptySketch)
        ));
    }
}

#[test]
fn same_distribution() {
    let (a, b) = shifted_pair(200, 0.0);
    assert!(kolmogorov_smirnov_delta(&a, &b).unwrap().abs() <= 0.02);
    assert!(!kolmogorov_smirnov_test(&a, &b, 0.01).unwrap());
}

#[test]
fn very_different_distributions() {
    let (a, b) = shifted_pair(200, 100.0);
    let delta = kolmogorov_smirnov_delta(&a, &b).unwrap();
    assert!((delta - 1.0).abs() <= 1e-6, "delta was {delta}");
    assert!(delta <= 1.0);
    assert!(kolmogorov_smirnov_test(&a, &b, 0.05).unwrap());
}

#[test]
fn slightly_different_distributions() {
    let (a, b) = shifted_pair(2000, 0.05);
    let delta = kolmogorov_smirnov_delta(&a, &b).unwrap();
    let threshold = kolmogorov_smirnov_threshold(&a, &b, 0.05).unwrap();
    assert!((delta - 0.02).abs() <= 0.01, "delta was {delta}");
    assert!(delta <= threshold);
    assert!(!kolmogorov_smirnov_test(&a, &b, 0.05).unwrap());
}

#[test]
fn slightly_different_distributions_high_resolution() {
    let (a, b) = shifted_pair(8000, 0.05);
    let delta = kolmogorov_smirnov_delta(&a, &b).unwrap();
    let threshold = kolmogorov_smirnov_threshold(&a, &b, 0.05).unwrap();
    assert!((delta - 0.02).abs() <= 0.01, "delta was {delta}");
    assert!(delta > threshold);
    assert!(kolmogorov_smirnov_test(&a, &b, 0.05).unwrap());
}

#[test]
fn p_out_of_range_is_rejected() {
    let (a, b) = shifted_pair(200, 0.0);
    for p in [0.0, 1.0, -0.5, 1.5, f64::NAN] {
        assert!(matches!(
            kolmogorov_smirnov_threshold(&a, &b, p),
            Err(SketchError::InvalidConfig(_))
        ));
        assert!(matches!(
            kolmogorov_smirnov_test(&a, &b, p),
            Err(SketchError::InvalidConfig(_))
        ));
    }
}

#[test]
fn works_for_f32() {
    let mut a = KllSketch::<f32>::default();
    let mut b = KllSketch::<f32>::default();
    for i in 0..1000u16 {
        a.update(f32::from(i));
        b.update(f32::from(i) + 10_000.0);
    }
    assert!(kolmogorov_smirnov_test(&a, &b, 0.05).unwrap());
}
