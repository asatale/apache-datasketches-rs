use apache_datasketches::tuple::{
    array_of_doubles_jaccard_similarity, ArrayOfDoublesSketch, ArrayOfDoublesSketchBuilder,
};

fn sketch(num_values: u8, keys: std::ops::Range<u64>, values: &[f64]) -> ArrayOfDoublesSketch {
    let mut s = ArrayOfDoublesSketchBuilder::new()
        .num_values(num_values)
        .build()
        .unwrap();
    for key in keys {
        s.update_u64(key, values).unwrap();
    }
    s
}

#[test]
fn identical_sketches_are_fully_similar() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 0..1000, &[1.0]);
    let bounds = array_of_doubles_jaccard_similarity(&a, &b).unwrap();
    assert_eq!(bounds.estimate, 1.0);
    assert_eq!(bounds.lower_bound, 1.0);
    assert_eq!(bounds.upper_bound, 1.0);
}

#[test]
fn disjoint_sketches_are_dissimilar() {
    let a = sketch(1, 0..1000, &[1.0]);
    let b = sketch(1, 2000..3000, &[1.0]);
    assert_eq!(
        array_of_doubles_jaccard_similarity(&a, &b)
            .unwrap()
            .estimate,
        0.0
    );
}

#[test]
fn half_overlap_accepts_all_four_combinations() {
    let a = sketch(2, 0..1000, &[1.0, 2.0]);
    let b = sketch(2, 500..1500, &[1.0, 2.0]);
    let ca = a.compact(true);
    let cb = b.compact(true);

    for bounds in [
        array_of_doubles_jaccard_similarity(&a, &b).unwrap(),
        array_of_doubles_jaccard_similarity(&a, &cb).unwrap(),
        array_of_doubles_jaccard_similarity(&ca, &b).unwrap(),
        array_of_doubles_jaccard_similarity(&ca, &cb).unwrap(),
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

#[test]
fn mismatched_num_values_is_err() {
    let a = sketch(1, 0..10, &[1.0]);
    let b = sketch(2, 0..10, &[1.0, 2.0]);
    assert!(array_of_doubles_jaccard_similarity(&a, &b).is_err());
}
