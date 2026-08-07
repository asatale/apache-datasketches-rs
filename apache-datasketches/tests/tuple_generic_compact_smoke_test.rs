use apache_datasketches::tuple::generic::{
    CompactTupleSketch, TupleSketch, TupleSketchBuilder, TupleSummary,
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

fn sketch(keys: std::ops::Range<u64>, value: i64) -> TupleSketch<Sum> {
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &value);
    }
    s
}

#[test]
fn compact_preserves_estimate_and_summaries() {
    let compact = sketch(0..500, 3).compact(true);
    assert!((compact.get_estimate() - 500.0).abs() < 1.0);
    assert_eq!(compact.get_num_retained(), 500);
    assert!(compact.is_ordered());
    assert!(compact.entries().all(|(_, s)| s == Sum(3)));
}

#[test]
fn entries_are_hash_ordered_when_compacted_ordered() {
    let compact = sketch(0..200, 1).compact(true);
    let hashes: Vec<u64> = compact.entries().map(|(h, _)| h).collect();
    assert_eq!(hashes.len(), 200);
    let mut sorted = hashes.clone();
    sorted.sort_unstable();
    assert_eq!(hashes, sorted);
}

#[test]
fn unordered_compaction_reports_itself_unordered() {
    let compact = sketch(0..50, 1).compact(false);
    assert!(!compact.is_ordered());
    assert_eq!(compact.entries().count(), 50);
}

#[test]
fn empty_sketch_compacts_to_empty() {
    let s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    let compact = s.compact(true);
    assert!(compact.is_empty());
    assert_eq!(compact.entries().count(), 0);
}

#[test]
fn compact_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<CompactTupleSketch<Sum>>();
}
