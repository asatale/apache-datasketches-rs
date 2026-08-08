use apache_datasketches::tuple::generic::{
    TupleSketch, TupleSketchBuilder, TupleSummary, TupleUnion, TupleUnionBuilder,
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
fn union_half_overlap() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().lg_k(12).build().unwrap();
    u.update(&sketch(0..1000, 1));
    u.update(&sketch(500..1500, 1));
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);
}

#[test]
fn union_accepts_both_input_types() {
    let a = sketch(0..100, 1);
    let b = sketch(50..150, 1).compact(true);
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&a);
    u.update(&b);
    assert_eq!(u.get_result(true).get_estimate(), 150.0);
}

#[test]
fn union_sums_summaries_on_collision() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&sketch(7..8, 10));
    u.update(&sketch(7..8, 32));
    let entries: Vec<(u64, Sum)> = u.get_result(true).entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, Sum(42));
}

#[test]
fn union_reset_empties_result() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&sketch(0..100, 1));
    assert!(!u.get_result(true).is_empty());
    u.reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn invalid_config_is_err() {
    assert!(TupleUnionBuilder::<Sum>::new().lg_k(4).build().is_err());
    assert!(TupleUnionBuilder::<Sum>::new().p(1.5).build().is_err());
}

#[test]
fn union_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleUnion<Sum>>();
}
