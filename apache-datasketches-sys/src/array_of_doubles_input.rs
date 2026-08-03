use crate::array_of_doubles_compact::ffi::CompactArrayOfDoublesSketchShim;
use crate::array_of_doubles_sketch::ffi::ArrayOfDoublesSketchShim;

/// A borrowed reference to one of the two ArrayOfDoubles sketch shim types,
/// used to dispatch set-operation and Jaccard-similarity calls to the correct
/// concrete C++ overload. This is a plain Rust enum rather than a
/// `#[cxx::bridge]` type: `cxx` only supports C-like (payload-free) shared
/// enums, and this enum's variants carry borrowed references to two unrelated
/// opaque C++ types, which cxx cannot express directly.
pub enum ArrayOfDoublesInputRef<'a> {
    /// A mutable, update-only sketch.
    Sketch(&'a ArrayOfDoublesSketchShim),
    /// An immutable, compact sketch.
    Compact(&'a CompactArrayOfDoublesSketchShim),
}
