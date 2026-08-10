//! Every generic Tuple type is `Send` but not `Sync`, matching the rest of
//! the crate. These tests move real sketches across threads rather than only
//! asserting the bound compiles.
//!
//! The per-component smoke tests each assert `Send` at compile time, but
//! nothing there actually moves a generic sketch between threads. Since the
//! summary is a boxed trait object owned by C++, a `Send` bound that compiles
//! but is unsound would only show up at runtime — which is what this file
//! reaches.

use apache_datasketches::tuple::generic::{
    CompactTupleSketch, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnion, TupleUnionBuilder,
};

#[derive(Clone, Debug, PartialEq)]
struct Tally {
    hits: u64,
    label: String,
}

impl TupleSummary for Tally {
    type Update = str;
    fn create(update: &str) -> Self {
        Tally {
            hits: 1,
            label: update.to_string(),
        }
    }
    fn union_combine(&mut self, other: &Self) {
        self.hits += other.hits;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.hits = self.hits.min(other.hits);
    }
}

#[test]
fn all_types_are_send() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleSketch<Tally>>();
    assert_send::<CompactTupleSketch<Tally>>();
    assert_send::<TupleUnion<Tally>>();
    assert_send::<TupleIntersection<Tally>>();
    assert_send::<TupleAnotB<Tally>>();
}

#[test]
fn a_sketch_built_on_one_thread_is_usable_on_another() {
    let handle = std::thread::spawn(|| {
        let mut sketch: TupleSketch<Tally> = TupleSketchBuilder::new().build().unwrap();
        for i in 0..1_000u64 {
            sketch.update_u64(i, "worker");
        }
        sketch.compact(true)
    });
    let compact = handle.join().unwrap();

    // The heap-owning summaries must survive the move intact.
    assert_eq!(compact.entries().count(), 1_000);
    assert!(compact.entries().all(|(_, t)| t
        == Tally {
            hits: 1,
            label: "worker".to_string()
        }));
}

#[test]
fn per_thread_sketches_merge_correctly() {
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            std::thread::spawn(move || {
                let mut sketch: TupleSketch<Tally> = TupleSketchBuilder::new().build().unwrap();
                for i in (t * 2_500)..((t + 1) * 2_500) {
                    sketch.update_u64(i, "shard");
                }
                sketch.compact(true)
            })
        })
        .collect();

    let mut union = TupleUnionBuilder::<Tally>::new().build().unwrap();
    for handle in handles {
        union.update(&handle.join().unwrap());
    }
    let result = union.get_result(true);
    assert!((result.get_estimate() - 10_000.0).abs() < 10_000.0 * 0.03);
}
