#![cfg(feature = "tuple")]

use apache_datasketches_sys::array_of_doubles_sketch::ffi;

const SHORT_SLICE_CHILD: &str = "AOD_SHORT_SLICE_CHILD";

#[test]
fn construct_update_estimate() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &[1.0]);
    }
    let estimate = sketch.get_estimate();
    assert!((estimate - 1000.0).abs() < 1.0, "estimate was {estimate}");
    assert_eq!(sketch.get_num_values(), 1);
    assert_eq!(sketch.get_num_retained(), 1000);
}

#[test]
fn invalid_lg_k_returns_err() {
    let result = ffi::new_array_of_doubles_sketch(4, ffi::TupleResizeFactor::X8, 1.0, 1);
    assert!(result.is_err());
}

#[test]
fn entries_expose_hashes_and_values() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 2).unwrap();
    sketch.pin_mut().update_u64(1, &[3.0, 4.0]);
    sketch.pin_mut().update_u64(1, &[3.0, 4.0]);
    assert_eq!(sketch.get_num_retained(), 1);
    let hashes = sketch.entry_hashes();
    let values = sketch.entry_values();
    assert_eq!(hashes.len(), 1);
    // Two updates of the same key sum their values.
    assert_eq!(values.len(), 2);
    assert_eq!(values[0], 6.0);
    assert_eq!(values[1], 8.0);
}

/// The shim hands upstream a bare `const double*`, which carries no length, so
/// `check_values_len` in the shim is the only thing preventing an
/// out-of-bounds read when a caller passes too few values. The update_* bridge
/// fns are declared without `Result`, so that check terminates the process
/// rather than returning an Err.
///
/// Verified by re-invoking this binary as a child, since the behaviour under
/// test kills the process. Without this, the guard could be deleted or
/// reordered after the update call and nothing would fail.
#[test]
fn short_values_slice_aborts_instead_of_reading_out_of_bounds() {
    if std::env::var(SHORT_SLICE_CHILD).is_ok() {
        let mut sketch =
            ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 3).unwrap();
        // num_values is 3; supply 1. Upstream would index [0], [1], [2].
        sketch.pin_mut().update_u64(1, &[1.0]);
        unreachable!("the update should have terminated the process");
    }

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "short_values_slice_aborts_instead_of_reading_out_of_bounds",
            "--nocapture",
        ])
        .env(SHORT_SLICE_CHILD, "1")
        .output()
        .expect("failed to spawn the child test process");

    assert!(
        !output.status.success(),
        "child was expected to terminate, but exited successfully"
    );

    // `code().is_none()` means killed by a signal (SIGABRT, from the
    // std::terminate that cxx's noexcept trampoline performs) rather than
    // merely exiting nonzero -- which is what `unreachable!()` firing would
    // produce if the guard were missing and the read silently succeeded.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(
            output.status.code().is_none(),
            "expected death by signal, got exit code {:?} (signal {:?}); \
             the length guard may have been removed",
            output.status.code(),
            output.status.signal()
        );
    }
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch =
        ffi::new_array_of_doubles_sketch(12, ffi::TupleResizeFactor::X8, 1.0, 1).unwrap();
    sketch.pin_mut().update_u64(1, &[1.0]);
    assert!(!sketch.is_empty());
    sketch.pin_mut().reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 0);
}
