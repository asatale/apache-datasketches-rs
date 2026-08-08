use crate::tuple_generic::ffi::{CompactTupleGenericSketchShim, TupleGenericSketchShim};

/// A borrowed reference to one of the two generic Tuple sketch shim types,
/// used to dispatch set-operation calls to the right C++ overload.
///
/// A plain Rust enum rather than a bridge type: cxx supports only
/// payload-free shared enums, and these variants carry references to opaque
/// C++ types.
pub enum TupleGenericInputRef<'a> {
    /// A mutable, update-only sketch.
    Sketch(&'a TupleGenericSketchShim),
    /// An immutable, compact sketch.
    Compact(&'a CompactTupleGenericSketchShim),
}
