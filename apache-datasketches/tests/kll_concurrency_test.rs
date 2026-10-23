use apache_datasketches::kll::KllSketch;
use std::sync::Arc;
use std::thread;

#[test]
fn sketch_moves_across_threads() {
    let mut sketch = KllSketch::<f64>::default();
    for i in 0..1000 {
        sketch.update(f64::from(i));
    }
    let n = thread::spawn(move || {
        sketch.update(1000.0);
        sketch.get_n()
    })
    .join()
    .unwrap();
    assert_eq!(n, 1001);
}

#[test]
fn per_thread_sketches_merge() {
    let handles: Vec<_> = (0..4)
        .map(|t| {
            thread::spawn(move || {
                let mut sketch = KllSketch::<f32>::default();
                for i in 0..10_000u32 {
                    sketch.update((t * 10_000 + i) as f32);
                }
                sketch
            })
        })
        .collect();
    let mut merged = KllSketch::<f32>::default();
    for h in handles {
        merged.merge(&h.join().unwrap());
    }
    assert_eq!(merged.get_n(), 40_000);
    assert_eq!(merged.get_min_item(), Some(0.0));
    assert_eq!(merged.get_max_item(), Some(39_999.0));
}

#[test]
fn sorted_view_is_shared_across_threads() {
    let mut sketch = KllSketch::<f64>::default();
    for i in 0..100_000 {
        sketch.update(f64::from(i));
    }
    let view = Arc::new(sketch.sorted_view());
    let expected = view.get_quantile(0.5, true).unwrap();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let view = Arc::clone(&view);
            thread::spawn(move || {
                (0..1000)
                    .map(|_| view.get_quantile(0.5, true).unwrap())
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    for h in handles {
        assert!(h.join().unwrap().iter().all(|&q| q == expected));
    }
}
