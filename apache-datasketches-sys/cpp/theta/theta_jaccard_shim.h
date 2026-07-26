#pragma once
#include <cstdint>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_jaccard_similarity.hpp"
#include "theta_sketch_shim.h"
#include "theta_compact_shim.h"
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

struct JaccardBoundsFfi;

JaccardBoundsFfi jaccard_sketch_sketch(const ThetaSketchShim& a, const ThetaSketchShim& b);
JaccardBoundsFfi jaccard_sketch_compact(const ThetaSketchShim& a, const CompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_sketch_wrapped(const ThetaSketchShim& a, const WrappedCompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_compact_sketch(const CompactThetaSketchShim& a, const ThetaSketchShim& b);
JaccardBoundsFfi jaccard_compact_compact(const CompactThetaSketchShim& a, const CompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_compact_wrapped(const CompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_wrapped_sketch(const WrappedCompactThetaSketchShim& a, const ThetaSketchShim& b);
JaccardBoundsFfi jaccard_wrapped_compact(const WrappedCompactThetaSketchShim& a, const CompactThetaSketchShim& b);
JaccardBoundsFfi jaccard_wrapped_wrapped(const WrappedCompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b);

} // namespace apache_datasketches_rs
