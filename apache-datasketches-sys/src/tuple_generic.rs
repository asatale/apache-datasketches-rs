//! The `extern "Rust"` surface the generic Tuple sketch's C++ layer calls
//! back into, plus the `extern "C++"` shims whose signatures mention a
//! summary.
//!
//! Those have to share one bridge module: cxx allows an `extern "Rust"`
//! opaque type to be declared in exactly one bridge per crate (a second
//! declaration is a conflicting `RustType` impl), and `extern "Rust"` blocks
//! do not support the `type X = crate::path::X;` aliasing that `extern "C++"`
//! blocks use to share a type across bridges. So the sketch and compact
//! shims — the only ones taking or returning a `RustSummary` — live here.
//! Union, intersection, a-not-b, and Jaccard pass only shim types and get
//! their own bridge files.
//!
//! Later tasks add the `extern "C++"` blocks below; this task creates only
//! the `extern "Rust"` one.

use std::any::Any;

/// The operations the C++ layer invokes on a type-erased Rust summary.
///
/// This is the sys crate's minimal internal trait. Users implement
/// `apache_datasketches::tuple::generic::TupleSummary` instead; the safe
/// crate adapts one to the other.
pub trait RawSummaryOps: Any + Send {
    /// Deep-copy this summary. Called when C++ copies a sketch or converts
    /// an update sketch to a compact one.
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send>;

    /// Merge `other` into `self` with union semantics.
    fn union_combine(&mut self, other: &dyn RawSummaryOps);

    /// Merge `other` into `self` with intersection semantics.
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps);

    /// Upcast for the concrete-type recovery the safe crate's adapter does.
    fn as_any(&self) -> &dyn Any;
}

/// A concrete, `Sized + Unpin` newtype around a boxed [`RawSummaryOps`].
///
/// cxx cannot expose a bare `dyn Trait` as an opaque `extern "Rust"` type —
/// opaque types must be `Sized` and `Unpin` — so the trait object is wrapped
/// in this struct, which is what actually crosses the boundary.
pub struct RustSummary {
    ops: Box<dyn RawSummaryOps + Send>,
}

impl RustSummary {
    /// Wraps a boxed summary implementation.
    pub fn new(ops: Box<dyn RawSummaryOps + Send>) -> Self {
        Self { ops }
    }

    /// Borrows the underlying operations object.
    pub fn ops(&self) -> &dyn RawSummaryOps {
        &*self.ops
    }

    /// Mutably borrows the underlying operations object.
    pub fn ops_mut(&mut self) -> &mut dyn RawSummaryOps {
        &mut *self.ops
    }
}

/// Runs `f`, converting any panic into a deliberate abort with a message
/// naming the operation.
///
/// cxx already prevents a panic from unwinding into C++ (it turns one into a
/// deterministic abort via a double-panic guard), so this does not add
/// safety — it adds diagnosis, replacing an unexplained fatal signal with an
/// actionable message. Upstream's combine policy returns `void`, so there is
/// no way to report failure to C++ and have it unwind the insert; continuing
/// would leave the sketch logically undefined, which is worse than stopping.
fn abort_on_panic<F, R>(what: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "apache-datasketches: a TupleSummary::{what} implementation panicked \
                 while called from C++. Panics cannot cross the FFI boundary, and the \
                 sketch cannot be left in a consistent state, so the process is aborting."
            );
            std::process::abort();
        }
    }
}

fn rust_summary_clone(summary: &RustSummary) -> Box<RustSummary> {
    abort_on_panic("clone", || {
        Box::new(RustSummary {
            ops: summary.ops.clone_boxed(),
        })
    })
}

fn rust_summary_union_combine(target: &mut RustSummary, other: &RustSummary) {
    abort_on_panic("union_combine", || {
        target.ops.union_combine(&*other.ops)
    })
}

fn rust_summary_intersection_combine(target: &mut RustSummary, other: &RustSummary) {
    abort_on_panic("intersection_combine", || {
        target.ops.intersection_combine(&*other.ops)
    })
}

/// Test-only wrapper around the clone trampoline.
#[doc(hidden)]
pub fn clone_for_test(summary: &RustSummary) -> Box<RustSummary> {
    rust_summary_clone(summary)
}

/// Test-only wrapper around the union trampoline.
#[doc(hidden)]
pub fn union_for_test(target: &mut RustSummary, other: &RustSummary) {
    rust_summary_union_combine(target, other)
}

/// Test-only wrapper around the intersection trampoline.
#[doc(hidden)]
pub fn intersection_for_test(target: &mut RustSummary, other: &RustSummary) {
    rust_summary_intersection_combine(target, other)
}

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    extern "Rust" {
        type RustSummary;

        fn rust_summary_clone(summary: &RustSummary) -> Box<RustSummary>;
        fn rust_summary_union_combine(target: &mut RustSummary, other: &RustSummary);
        fn rust_summary_intersection_combine(target: &mut RustSummary, other: &RustSummary);
    }
}
