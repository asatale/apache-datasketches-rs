#include "theta_intersection_shim.h"

namespace apache_datasketches_rs {

ThetaIntersectionShim::ThetaIntersectionShim() : intersection_() {}

void ThetaIntersectionShim::update_with_sketch(const ThetaSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

void ThetaIntersectionShim::update_with_compact(const CompactThetaSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

void ThetaIntersectionShim::update_with_wrapped(const WrappedCompactThetaSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

std::unique_ptr<CompactThetaSketchShim> ThetaIntersectionShim::get_result(bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(intersection_.get_result(ordered));
}

bool ThetaIntersectionShim::has_result() const { return intersection_.has_result(); }

std::unique_ptr<ThetaIntersectionShim> new_theta_intersection() {
  return std::make_unique<ThetaIntersectionShim>();
}

} // namespace apache_datasketches_rs
