use apache_datasketches::hll::{HllSketch, HllUnion, TargetHllType};
use std::thread;

fn assert_send<T: Send>() {}

#[test]
fn hll_sketch_is_send() {
    assert_send::<HllSketch>();
}

#[test]
fn hll_union_is_send() {
    assert_send::<HllUnion>();
}

#[test]
fn hll_sketch_moves_across_thread_boundary() {
    let mut sketch = HllSketch::new(8, TargetHllType::Hll8).unwrap();
    for i in 0..50u64 {
        sketch.update_u64(i);
    }

    let handle = thread::spawn(move || sketch.get_estimate());
    let estimate = handle.join().unwrap();
    assert!((estimate - 50.0).abs() < 5.0);
}
