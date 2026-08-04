#pragma once
#include <cstdint>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "tuple_jaccard_similarity.hpp"
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"
#include "array_of_doubles_intersection_shim.h"

namespace apache_datasketches_rs {

// Named TupleJaccardBoundsFfi rather than JaccardBoundsFfi because cxx emits
// one C++ definition per shared type into this namespace, and the theta bridge
// already emits apache_datasketches_rs::JaccardBoundsFfi there.
struct TupleJaccardBoundsFfi;

// Named tuple_jaccard_* rather than jaccard_* because cxx's generated extern
// "C" trampoline symbol is derived from the C++ namespace and function name
// only; the theta bridge's jaccard_* overloads (see ../theta/theta_jaccard_shim.h)
// would otherwise collide at link time when both bridges are compiled
// together (--all-features).
TupleJaccardBoundsFfi tuple_jaccard_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi tuple_jaccard_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi tuple_jaccard_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi tuple_jaccard_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b);

} // namespace apache_datasketches_rs
