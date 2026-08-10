use apache_datasketches::tuple::generic::{
    tuple_jaccard_similarity, TupleSketch, TupleSketchBuilder, TupleSummary,
};

#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sketch(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<Sum> {
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

#[test]
fn identical_sketches_are_fully_similar() {
    let bounds = tuple_jaccard_similarity(&sketch(0..1000, 1), &sketch(0..1000, 1));
    assert_eq!(bounds.estimate, 1.0);
}

#[test]
fn disjoint_sketches_are_dissimilar() {
    assert_eq!(
        tuple_jaccard_similarity(&sketch(0..1000, 1), &sketch(2000..3000, 1)).estimate,
        0.0
    );
}

#[test]
fn half_overlap_accepts_all_four_combinations() {
    let a = sketch(0..1000, 1);
    let b = sketch(500..1500, 1);
    let ca = a.compact(true);
    let cb = b.compact(true);
    for bounds in [
        tuple_jaccard_similarity(&a, &b),
        tuple_jaccard_similarity(&a, &cb),
        tuple_jaccard_similarity(&ca, &b),
        tuple_jaccard_similarity(&ca, &cb),
    ] {
        assert!((bounds.estimate - 1.0 / 3.0).abs() < 0.01);
        assert!(bounds.lower_bound <= bounds.estimate);
        assert!(bounds.estimate <= bounds.upper_bound);
    }
}

#[test]
fn summary_values_do_not_affect_the_result() {
    let baseline = tuple_jaccard_similarity(&sketch(0..1000, 1), &sketch(500..1500, 1));
    let different = tuple_jaccard_similarity(&sketch(0..1000, 99), &sketch(500..1500, -7));
    assert_eq!(baseline, different);
}
