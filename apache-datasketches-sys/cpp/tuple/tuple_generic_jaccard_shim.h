#pragma once
#include <cstdint>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_jaccard_similarity.hpp"
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"

namespace apache_datasketches_rs {

struct TupleGenericJaccardBoundsFfi;

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_sketch_sketch(const TupleGenericSketchShim& a, const TupleGenericSketchShim& b);
TupleGenericJaccardBoundsFfi tuple_generic_jaccard_sketch_compact(const TupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b);
TupleGenericJaccardBoundsFfi tuple_generic_jaccard_compact_sketch(const CompactTupleGenericSketchShim& a, const TupleGenericSketchShim& b);
TupleGenericJaccardBoundsFfi tuple_generic_jaccard_compact_compact(const CompactTupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b);

} // namespace apache_datasketches_rs
