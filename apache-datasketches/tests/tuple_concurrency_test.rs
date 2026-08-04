//! Every ArrayOfDoubles type is `Send` but not `Sync`, matching every other
//! sketch family in this crate: the underlying C++ objects can be moved
//! between threads but have no internal synchronisation for shared access.

use apache_datasketches::tuple::{
    ArrayOfDoublesAnotB, ArrayOfDoublesIntersection, ArrayOfDoublesSketch,
    ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnion, ArrayOfDoublesUnionBuilder,
    CompactArrayOfDoublesSketch,
};

fn assert_send<T: Send>() {}

#[test]
fn all_types_are_send() {
    assert_send::<ArrayOfDoublesSketch>();
    assert_send::<CompactArrayOfDoublesSketch>();
    assert_send::<ArrayOfDoublesUnion>();
    assert_send::<ArrayOfDoublesIntersection>();
    assert_send::<ArrayOfDoublesAnotB>();
}

#[test]
fn sketch_can_be_built_on_one_thread_and_used_on_another() {
    let handle = std::thread::spawn(|| {
        let mut sketch = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
        for i in 0..1000u64 {
            sketch.update_u64(i, &[1.0, 2.0]).unwrap();
        }
        sketch.compact(true)
    });
    let compact = handle.join().unwrap();

    let mut u = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    u.update(&compact).unwrap();
    assert!((u.get_result(true).get_estimate() - 1000.0).abs() < 1.0);
}

#[test]
fn per_thread_sketches_merge_correctly() {
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            std::thread::spawn(move || {
                let mut sketch = ArrayOfDoublesSketchBuilder::new().build().unwrap();
                for i in (t * 250)..((t + 1) * 250) {
                    sketch.update_u64(i, &[1.0]).unwrap();
                }
                sketch.compact(true)
            })
        })
        .collect();

    let mut u = ArrayOfDoublesUnionBuilder::new().build().unwrap();
    for handle in handles {
        u.update(&handle.join().unwrap()).unwrap();
    }
    assert_eq!(u.get_result(true).get_estimate(), 1000.0);
}
