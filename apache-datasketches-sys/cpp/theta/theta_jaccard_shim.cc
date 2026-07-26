#include "theta_jaccard_shim.h"
#include "theta_jaccard.rs.h"

namespace apache_datasketches_rs {

namespace {
JaccardBoundsFfi to_ffi(const std::array<double, 3>& result) {
  return JaccardBoundsFfi{result[0], result[1], result[2]};
}
} // namespace

JaccardBoundsFfi jaccard_sketch_sketch(const ThetaSketchShim& a, const ThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_sketch_compact(const ThetaSketchShim& a, const CompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_sketch_wrapped(const ThetaSketchShim& a, const WrappedCompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_compact_sketch(const CompactThetaSketchShim& a, const ThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_compact_compact(const CompactThetaSketchShim& a, const CompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_compact_wrapped(const CompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_wrapped_sketch(const WrappedCompactThetaSketchShim& a, const ThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_wrapped_compact(const WrappedCompactThetaSketchShim& a, const CompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

JaccardBoundsFfi jaccard_wrapped_wrapped(const WrappedCompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b) {
  return to_ffi(datasketches::theta_jaccard_similarity::jaccard(a.inner(), b.inner()));
}

} // namespace apache_datasketches_rs
