use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};

/// Sums on union, takes the minimum on intersection — deliberately different
/// so a cross-wired trampoline is detectable.
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

#[test]
fn construct_update_estimate() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i, &1);
    }
    assert!((sketch.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(sketch.get_num_retained(), 1000);
}

#[test]
fn repeated_key_unions_its_summaries() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for _ in 0..4 {
        sketch.update_u64(7, &10);
    }
    assert_eq!(sketch.get_num_retained(), 1);
}

#[test]
fn invalid_config_is_err() {
    assert!(TupleSketchBuilder::<Sum>::new().lg_k(4).build().is_err());
    assert!(TupleSketchBuilder::<Sum>::new().p(0.0).build().is_err());
    assert!(TupleSketchBuilder::<Sum>::new().p(1.5).build().is_err());
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    sketch.update_u64(1, &1);
    assert!(!sketch.is_empty());
    sketch.reset();
    assert!(sketch.is_empty());
}

#[test]
fn every_update_key_type_works() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    sketch.update_u64(1, &1);
    sketch.update_i64(2, &1);
    sketch.update_u32(3, &1);
    sketch.update_i32(4, &1);
    sketch.update_u16(5, &1);
    sketch.update_i16(6, &1);
    sketch.update_u8(7, &1);
    sketch.update_i8(8, &1);
    sketch.update_f64(9.0, &1);
    sketch.update_str("ten", &1);
    sketch.update_bytes(b"eleven", &1);
    assert_eq!(sketch.get_num_retained(), 11);
}

#[test]
fn sketch_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleSketch<Sum>>();
}
