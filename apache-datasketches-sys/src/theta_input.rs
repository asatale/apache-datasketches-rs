use crate::theta_compact::ffi::CompactThetaSketchShim;
use crate::theta_sketch::ffi::ThetaSketchShim;
use crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;

/// A borrowed reference to one of the three theta sketch shim types,
/// used to dispatch set-operation and Jaccard-similarity calls to the
/// correct concrete C++ overload. This is a plain Rust enum rather than
/// a `#[cxx::bridge]` type: `cxx` only supports C-like (payload-free)
/// shared enums, and this enum's variants carry borrowed references to
/// three unrelated opaque C++ types, which cxx cannot express directly.
pub enum ThetaInputRef<'a> {
    Sketch(&'a ThetaSketchShim),
    Compact(&'a CompactThetaSketchShim),
    Wrapped(&'a WrappedCompactThetaSketchShim),
}
