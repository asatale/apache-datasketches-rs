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
    // On unix, `code().is_none()` means the process was killed by a signal
    // (SIGABRT, from `std::process::abort()`) rather than merely exiting
    // with a nonzero status -- e.g. from `unreachable!()` firing because the
    // abort never happened. Without this, a child that reaches
    // `unreachable!()` and exits 101 would satisfy `!success()` above for
    // the wrong reason, though the stderr check below would still catch it.
    #[cfg(unix)]
    assert!(
        output.status.code().is_none(),
        "child should have been killed by a signal (aborted), not exited \
         with a status code; status was {:?}",
        output.status
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("TupleSummary::union_combine"),
        "abort message should name the panicking method; stderr was:\n{stderr}"
    );
}

/// A summary whose `Clone` panics, forced through an insert.
///
/// Every new-key insert makes C++ call `assign_clone_of`, which reaches
/// `rust_summary_clone` (`apache-datasketches-sys/src/tuple_generic.rs:91`)
/// before any combine callback is ever involved, so a single `update_u64`
/// call on a fresh sketch is enough to drive this path -- no second update
/// needed, unlike the `union_combine` test above.
#[derive(Debug)]
struct ExplodesOnClone;

impl Clone for ExplodesOnClone {
    fn clone(&self) -> Self {
        panic!("deliberate panic from Clone::clone");
    }
}

impl TupleSummary for ExplodesOnClone {
    type Update = ();
    fn create(_: &()) -> Self {
        ExplodesOnClone
    }
    fn union_combine(&mut self, _other: &Self) {}
    fn intersection_combine(&mut self, _other: &Self) {}
}

const CLONE_CHILD_ENV: &str = "TUPLE_GENERIC_PANIC_CLONE_CHILD";

#[test]
fn panicking_clone_aborts_with_a_diagnostic() {
    if std::env::var(CLONE_CHILD_ENV).is_ok() {
        let mut s: TupleSketch<ExplodesOnClone> = TupleSketchBuilder::new().build().unwrap();
        s.update_u64(1, &());
        unreachable!("the update should have aborted the process via the clone trampoline");
    }

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "panicking_clone_aborts_with_a_diagnostic",
            "--nocapture",
        ])
        .env(CLONE_CHILD_ENV, "1")
        .output()
        .expect("failed to spawn the child test process");

    assert!(
        !output.status.success(),
        "child was expected to abort, but exited successfully"
    );
    #[cfg(unix)]
    assert!(
        output.status.code().is_none(),
        "child should have been killed by a signal (aborted), not exited \
         with a status code; status was {:?}",
        output.status
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // NOTE: this asserts the string `abort_on_panic("clone", ...)` actually
    // emits TODAY (apache-datasketches-sys/src/tuple_generic.rs:91-97), not
    // a corrected one. `TupleSummary::clone` does not name a resolvable
    // method -- `TupleSummary` has no `clone` method of its own; cloning
    // comes from the `Clone` supertrait -- so the emitted path is wrong.
    // That is an open, tracked naming defect in library code
    // (`abort_on_panic("clone", ...)`), out of scope for this test-only
    // fix pass and left for the whole-branch review; this assertion is
    // deliberately pinned to the current (defective) output so it does not
    // paper over the defect.
    assert!(
        stderr.contains("TupleSummary::clone"),
        "abort message should name the panicking method (per the open \
         naming defect, this is the unresolvable `TupleSummary::clone`, not \
         a corrected path); stderr was:\n{stderr}"
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
