//! Cross-checks the generic framework against the shipped ArrayOfDoubles
//! family. A summary of `Vec<f64>` summed per index is precisely what
//! ArrayOfDoubles implements as a concrete C++ instantiation, so the two must
//! agree on every observable quantity for the same inputs. Any divergence is
//! a bug in the callback core, since the reference side is already tested.
//!
//! The per-component smoke tests can only check that the generic side is
//! self-consistent; nothing there says the callback core produces the *same*
//! sketch a hand-written C++ instantiation would. That is what this file adds.
//!
//! Both families default to `lg_k = 12`, `resize_factor = X8` and `p = 1.0`,
//! so the tests that use bare `new()` really do compare like with like.

use apache_datasketches::tuple::generic::{
    TupleSketch, TupleSketchBuilder, TupleSummary, TupleUnionBuilder,
};
use apache_datasketches::tuple::{ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder};

#[derive(Clone, Debug, PartialEq)]
struct Doubles(Vec<f64>);

impl TupleSummary for Doubles {
    type Update = [f64];
    fn create(update: &[f64]) -> Self {
        Doubles(update.to_vec())
    }
    fn union_combine(&mut self, other: &Self) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a += *b;
        }
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.union_combine(other);
    }
}

#[test]
fn sketch_matches_array_of_doubles_in_exact_mode() {
    let mut generic: TupleSketch<Doubles> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    let mut reference = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();

    for i in 0..1000u64 {
        generic.update_u64(i, &[1.0, 2.0][..]);
        reference.update_u64(i, &[1.0, 2.0]).unwrap();
    }

    assert_eq!(generic.get_estimate(), reference.get_estimate());
    assert_eq!(generic.get_num_retained(), reference.get_num_retained());
    assert_eq!(generic.get_theta(), reference.get_theta());
    assert_eq!(generic.is_estimation_mode(), reference.is_estimation_mode());
}

#[test]
fn sketch_matches_array_of_doubles_in_estimation_mode() {
    let mut generic: TupleSketch<Doubles> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    let mut reference = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();

    for i in 0..50_000u64 {
        generic.update_u64(i, &[1.0, 2.0][..]);
        reference.update_u64(i, &[1.0, 2.0]).unwrap();
    }

    assert!(generic.is_estimation_mode());
    assert_eq!(generic.get_estimate(), reference.get_estimate());
    assert_eq!(generic.get_num_retained(), reference.get_num_retained());
    assert_eq!(generic.get_theta(), reference.get_theta());

    // Same keys retained, same summed values.
    let mut g: Vec<(u64, Vec<f64>)> = generic
        .compact(true)
        .entries()
        .map(|(h, s)| (h, s.0))
        .collect();
    let mut r: Vec<(u64, Vec<f64>)> = reference.compact(true).entries().collect();
    g.sort_by_key(|(h, _)| *h);
    r.sort_by_key(|(h, _)| *h);
    assert_eq!(g, r);
}

#[test]
fn repeated_keys_sum_the_same_way() {
    let mut generic: TupleSketch<Doubles> = TupleSketchBuilder::new().build().unwrap();
    let mut reference = ArrayOfDoublesSketchBuilder::new()
        .num_values(2)
        .build()
        .unwrap();
    for _ in 0..5 {
        generic.update_u64(42, &[1.0, 2.0][..]);
        reference.update_u64(42, &[1.0, 2.0]).unwrap();
    }
    let g: Vec<(u64, Vec<f64>)> = generic
        .compact(true)
        .entries()
        .map(|(h, s)| (h, s.0))
        .collect();
    let r: Vec<(u64, Vec<f64>)> = reference.compact(true).entries().collect();
    assert_eq!(g, r);
    assert_eq!(g[0].1, vec![5.0, 10.0]);
}

#[test]
fn union_matches_array_of_doubles_in_estimation_mode() {
    let mut ga: TupleSketch<Doubles> = TupleSketchBuilder::new().build().unwrap();
    let mut gb: TupleSketch<Doubles> = TupleSketchBuilder::new().build().unwrap();
    let mut ra = ArrayOfDoublesSketchBuilder::new()
        .num_values(2)
        .build()
        .unwrap();
    let mut rb = ArrayOfDoublesSketchBuilder::new()
        .num_values(2)
        .build()
        .unwrap();

    for i in 0..30_000u64 {
        ga.update_u64(i, &[1.0, 2.0][..]);
        ra.update_u64(i, &[1.0, 2.0]).unwrap();
    }
    for i in 15_000..45_000u64 {
        gb.update_u64(i, &[1.0, 2.0][..]);
        rb.update_u64(i, &[1.0, 2.0]).unwrap();
    }

    let mut gu = TupleUnionBuilder::<Doubles>::new().build().unwrap();
    gu.update(&ga);
    gu.update(&gb);
    let mut ru = ArrayOfDoublesUnionBuilder::new()
        .num_values(2)
        .build()
        .unwrap();
    ru.update(&ra).unwrap();
    ru.update(&rb).unwrap();

    let g = gu.get_result(true);
    let r = ru.get_result(true);
    assert!(g.is_estimation_mode());
    assert_eq!(g.get_estimate(), r.get_estimate());
    assert_eq!(g.get_num_retained(), r.get_num_retained());

    // Cardinality alone would pass even with the union combine callback
    // broken, since the 15k-key overlap changes no key. Compare the summed
    // values too: overlapping keys must carry [2.0, 4.0], the rest [1.0, 2.0],
    // on both sides.
    let mut ge: Vec<(u64, Vec<f64>)> = g.entries().map(|(h, s)| (h, s.0)).collect();
    let mut re: Vec<(u64, Vec<f64>)> = r.entries().collect();
    ge.sort_by_key(|(h, _)| *h);
    re.sort_by_key(|(h, _)| *h);
    assert_eq!(ge, re);
    assert!(ge.iter().any(|(_, v)| *v == vec![2.0, 4.0]));
}
