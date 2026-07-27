// apache-datasketches/tests/cpc_concurrency_test.rs
//! New, non-upstream tests: `Send` verification (matching this plan's
//! HLL/Theta precedent) and a concurrent-use scenario for `cpc::init()`,
//! which addresses CPC's global-decompression-table initialization
//! hazard documented in `apache_datasketches::cpc::init`.
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
fn init_then_concurrent_serialize_deserialize() {
    // cpc::init() must be called single-threaded before any concurrent
    // first-use of serialize/deserialize, since upstream's lazy
    // self-initialization of the global decompression tables is not
    // thread-safe. Calling it here, before spawning, avoids the race.
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
