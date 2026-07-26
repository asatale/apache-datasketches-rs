use apache_datasketches::theta::{ThetaInput, ThetaSketchBuilder, WrappedCompactThetaSketch};

fn accepts_theta_input(input: &impl ThetaInput) -> f64 {
    // Exercises the trait bound generically, as every set-op signature will.
    match input.as_theta_input() {
        apache_datasketches_sys::theta_input::ThetaInputRef::Sketch(s) => s.get_estimate(),
        apache_datasketches_sys::theta_input::ThetaInputRef::Compact(c) => c.get_estimate(),
        apache_datasketches_sys::theta_input::ThetaInputRef::Wrapped(w) => w.get_estimate(),
    }
}

#[test]
fn theta_sketch_implements_theta_input() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    sketch.update_u64(42);
    assert!(accepts_theta_input(&sketch) > 0.0);
}

#[test]
fn compact_theta_sketch_implements_theta_input() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    sketch.update_u64(42);
    let compact = sketch.compact(true);
    assert!(accepts_theta_input(&compact) > 0.0);
}

#[test]
fn wrapped_compact_theta_sketch_implements_theta_input() {
    let mut sketch = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
    sketch.update_u64(42);
    let compact = sketch.compact(true);
    let bytes = compact.serialize_compact();
    let wrapped = WrappedCompactThetaSketch::wrap(&bytes).unwrap();
    assert!(accepts_theta_input(&wrapped) > 0.0);
}
