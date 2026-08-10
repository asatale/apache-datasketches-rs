#include "tuple_generic_jaccard_shim.h"
#include "tuple_generic_jaccard.rs.h"

namespace apache_datasketches_rs {

namespace {

// Upstream provides no jaccard alias for a generic Tuple summary, so we
// instantiate the same generic template it uses internally.
//
// jaccard() builds a scratch union via `typename Union::builder()` and a
// scratch intersection via `Intersection(seed)`, both with default-constructed
// policies. DynUnionPolicy and DynIntersectionPolicy are stateless, so that is
// correct -- do not give them fields. jaccard() also reads only
// get_num_retained()/get_theta64()/is_empty() from those scratch results, never
// a summary's contents, so the callbacks it triggers cannot affect the bounds.
using dyn_jaccard = datasketches::jaccard_similarity_base<
    dyn_union,
    dyn_intersection,
    datasketches::pair_extract_key<uint64_t, DynSummary>>;

TupleGenericJaccardBoundsFfi to_ffi(const std::array<double, 3>& result) {
  return TupleGenericJaccardBoundsFfi{result[0], result[1], result[2]};
}

} // namespace

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_sketch_sketch(const TupleGenericSketchShim& a, const TupleGenericSketchShim& b) {
  return to_ffi(dyn_jaccard::jaccard(a.inner(), b.inner()));
}

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_sketch_compact(const TupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b) {
  return to_ffi(dyn_jaccard::jaccard(a.inner(), b.inner()));
}

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_compact_sketch(const CompactTupleGenericSketchShim& a, const TupleGenericSketchShim& b) {
  return to_ffi(dyn_jaccard::jaccard(a.inner(), b.inner()));
}

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_compact_compact(const CompactTupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b) {
  return to_ffi(dyn_jaccard::jaccard(a.inner(), b.inner()));
}

} // namespace apache_datasketches_rs
