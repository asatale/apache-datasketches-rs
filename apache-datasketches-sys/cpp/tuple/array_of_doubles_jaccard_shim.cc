#include "array_of_doubles_jaccard_shim.h"
#include "array_of_doubles_jaccard.rs.h"

namespace apache_datasketches_rs {

namespace {

// Upstream ships tuple_jaccard_similarity<Summary, IntersectionPolicy,
// UnionPolicy, Allocator> but no array-of-doubles alias for it, so we
// instantiate the same underlying generic template directly with this
// family's concrete union/intersection types.
//
// Note on num_values: jaccard_similarity_base::jaccard() internally builds a
// scratch union via `typename Union::builder()` and a scratch intersection via
// `Intersection(seed)`, both of which get a default-constructed policy whose
// num_values is 1 regardless of the operand sketches' actual width. That is
// harmless here: jaccard() derives its result solely from
// get_num_retained()/get_theta64()/is_empty() on those scratch results and
// never reads a summary array, so the scratch objects' incomplete per-index
// summing cannot affect the returned bounds.
using aod_jaccard = datasketches::jaccard_similarity_base<
    datasketches::array_of_doubles_union,
    aod_intersection,
    datasketches::pair_extract_key<uint64_t, datasketches::array<double>>>;

TupleJaccardBoundsFfi to_ffi(const std::array<double, 3>& result) {
  return TupleJaccardBoundsFfi{result[0], result[1], result[2]};
}

} // namespace

TupleJaccardBoundsFfi jaccard_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b) {
  return to_ffi(aod_jaccard::jaccard(a.inner(), b.inner()));
}

TupleJaccardBoundsFfi jaccard_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b) {
  return to_ffi(aod_jaccard::jaccard(a.inner(), b.inner()));
}

TupleJaccardBoundsFfi jaccard_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b) {
  return to_ffi(aod_jaccard::jaccard(a.inner(), b.inner()));
}

TupleJaccardBoundsFfi jaccard_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b) {
  return to_ffi(aod_jaccard::jaccard(a.inner(), b.inner()));
}

} // namespace apache_datasketches_rs
