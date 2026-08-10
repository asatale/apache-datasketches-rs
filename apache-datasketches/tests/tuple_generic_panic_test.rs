//! A panic inside a trampolined TupleSummary method must abort the process
//! with a message naming the method, rather than unwinding into C++.
//!
//! No other test can reach this: the behaviour under test terminates the
//! process, so it is verified by re-invoking this test binary as a child.
//! Without it, the documented abort-with-a-diagnostic guarantee is untested
//! prose.

use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};

const CHILD_ENV: &str = "TUPLE_GENERIC_PANIC_CHILD";

#[derive(Clone, Debug)]
struct Exploding;

impl TupleSummary for Exploding {
    type Update = ();
    fn create(_: &()) -> Self {
        Exploding
    }
    fn union_combine(&mut self, _other: &Self) {
        panic!("deliberate panic from union_combine");
    }
    fn intersection_combine(&mut self, _other: &Self) {
        panic!("deliberate panic from intersection_combine");
    }
}

#[test]
fn panicking_union_combine_aborts_with_a_diagnostic() {
    if std::env::var(CHILD_ENV).is_ok() {
        let mut s: TupleSketch<Exploding> = TupleSketchBuilder::new().build().unwrap();
        // First update inserts (clone path); the second hits the same key and
        // therefore calls union_combine from C++.
        s.update_u64(1, &());
        s.update_u64(1, &());
        unreachable!("the second update should have aborted the process");
    }

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "panicking_union_combine_aborts_with_a_diagnostic",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1")
        .output()
        .expect("failed to spawn the child test process");

    assert!(
        !output.status.success(),
        "child was expected to abort, but exited successfully"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("TupleSummary::union_combine"),
        "abort message should name the panicking method; stderr was:\n{stderr}"
    );
}

/// `create` runs entirely Rust-side before any C++ call, so a panic there is
/// an ordinary catchable Rust panic — the documented contrast with the three
/// trampolined methods.
#[test]
fn panicking_create_is_an_ordinary_rust_panic() {
    #[derive(Clone, Debug)]
    struct PanicsOnCreate;

    impl TupleSummary for PanicsOnCreate {
        type Update = ();
        fn create(_: &()) -> Self {
            panic!("deliberate panic from create");
        }
        fn union_combine(&mut self, _: &Self) {}
        fn intersection_combine(&mut self, _: &Self) {}
    }

    let mut s: TupleSketch<PanicsOnCreate> = TupleSketchBuilder::new().build().unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        s.update_u64(1, &());
    }));
    assert!(result.is_err(), "a panic in create must be catchable");
}
