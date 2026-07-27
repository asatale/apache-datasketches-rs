// apache-datasketches/tests/cpc_concurrency_test.rs
//! New, non-upstream tests: `Send` verification (matching this plan's
//! HLL/Theta precedent) and a concurrent smoke test that exercises real
//! concurrent `CpcSketch` usage, including the recommended pattern of
//! calling `cpc::init()` eagerly (see `apache_datasketches::cpc::init` for
//! why that's a latency optimization, not a correctness fix).
use apache_datasketches::cpc::{self, CpcSketch, CpcSketchBuilder, CpcUnion};
use std::thread;

fn assert_send<T: Send>() {}

#[test]
fn cpc_sketch_is_send() {
    assert_send::<CpcSketch>();
}

#[test]
fn cpc_union_is_send() {
    assert_send::<CpcUnion>();
}

#[test]
fn cpc_sketch_moves_across_thread_boundary() {
    let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
    for i in 0..50u64 {
        sketch.update_u64(i);
    }

    let handle = thread::spawn(move || sketch.get_estimate());
    let estimate = handle.join().unwrap();
    assert!((estimate - 50.0).abs() < 10.0);
}

#[test]
fn concurrent_serialize_deserialize_across_threads() {
    // Not testing a correctness hazard (see cpc::init()'s doc comment: the
    // lazy table init is safe under concurrent access via C++11 magic
    // statics). This demonstrates the recommended pattern of calling
    // init() eagerly to avoid a first-use latency stall, and exercises
    // real concurrent CpcSketch usage across threads.
    cpc::init();

    let handles: Vec<_> = (0..8u64)
        .map(|t| {
            thread::spawn(move || {
                let mut sketch = CpcSketchBuilder::new().lg_k(11).build().unwrap();
                for i in 0..1000u64 {
                    sketch.update_u64(i + t * 1000);
                }
                let bytes = sketch.serialize();
                let restored = CpcSketch::deserialize(&bytes).unwrap();
                assert_eq!(sketch.get_estimate(), restored.get_estimate());
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
