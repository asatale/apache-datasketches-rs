#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_jaccard::ffi as jaccard_ffi;
use apache_datasketches_sys::array_of_doubles_sketch::ffi as sketch_ffi;

fn sketch(keys: std::ops::Range<u64>) -> cxx::UniquePtr<sketch_ffi::ArrayOfDoublesSketchShim> {
    let mut s =
        sketch_ffi::new_array_of_doubles_sketch(12, sketch_ffi::TupleResizeFactor::X8, 1.0, 1)
            .unwrap();
    for key in keys {
        s.pin_mut().update_u64(key, &[1.0]);
    }
    s
}

#[test]
fn identical_sketches_are_fully_similar() {
    let a = sketch(0..1000);
    let b = sketch(0..1000);
    let bounds = jaccard_ffi::tuple_jaccard_sketch_sketch(&a, &b);
    assert_eq!(bounds.estimate, 1.0);
    assert_eq!(bounds.lower_bound, 1.0);
    assert_eq!(bounds.upper_bound, 1.0);
}

#[test]
fn disjoint_sketches_are_dissimilar() {
    let a = sketch(0..1000);
    let b = sketch(2000..3000);
    let bounds = jaccard_ffi::tuple_jaccard_sketch_sketch(&a, &b);
    assert_eq!(bounds.estimate, 0.0);
}

#[test]
fn half_overlap_is_about_one_third_all_four_combinations() {
    let a = sketch(0..1000);
    let b = sketch(500..1500);
    let ca = a.compact(true);
    let cb = b.compact(true);

    // |A ∩ B| / |A ∪ B| = 500 / 1500 = 1/3
    for bounds in [
        jaccard_ffi::tuple_jaccard_sketch_sketch(&a, &b),
        jaccard_ffi::tuple_jaccard_sketch_compact(&a, &cb),
        jaccard_ffi::tuple_jaccard_compact_sketch(&ca, &b),
        jaccard_ffi::tuple_jaccard_compact_compact(&ca, &cb),
    ] {
        assert!(
            (bounds.estimate - 1.0 / 3.0).abs() < 0.01,
            "estimate was {}",
            bounds.estimate
        );
        assert!(bounds.lower_bound <= bounds.estimate);
        assert!(bounds.upper_bound >= bounds.estimate);
    }
}
