use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder};
use std::sync::Arc;
use std::thread;

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn types_are_send_and_sync() {
    assert_send_sync::<BloomFilter>();
    assert_send_sync::<apache_datasketches::bloom::WrappedBloomFilter<'static>>();
}

#[test]
fn shared_filter_is_queried_concurrently() {
    let mut f = BloomFilterBuilder::by_accuracy(100_000, 0.01)
        .build()
        .unwrap();
    for i in 0..50_000u64 {
        f.update_u64(i);
    }
    let f = Arc::new(f);
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let f = Arc::clone(&f);
            thread::spawn(move || (0..50_000u64).all(|i| f.query_u64(i)))
        })
        .collect();
    for h in handles {
        assert!(h.join().unwrap());
    }
}

#[test]
fn per_thread_filters_union() {
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            thread::spawn(move || {
                let mut f = BloomFilterBuilder::by_accuracy(100_000, 0.01)
                    .build()
                    .unwrap();
                for i in 0..10_000u64 {
                    f.update_u64(t * 10_000 + i);
                }
                f
            })
        })
        .collect();
    let mut merged = BloomFilterBuilder::by_accuracy(100_000, 0.01)
        .build()
        .unwrap();
    for h in handles {
        merged.union_with(&h.join().unwrap()).unwrap();
    }
    assert!((0..40_000u64).all(|i| merged.query_u64(i)));
}
