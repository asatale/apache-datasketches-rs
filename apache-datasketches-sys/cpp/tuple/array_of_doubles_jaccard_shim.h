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

TupleJaccardBoundsFfi jaccard_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi jaccard_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi jaccard_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b);
TupleJaccardBoundsFfi jaccard_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b);

} // namespace apache_datasketches_rs
