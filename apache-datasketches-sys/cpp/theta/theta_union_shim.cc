#include "theta_union_shim.h"

namespace apache_datasketches_rs {

ThetaUnionShim::ThetaUnionShim(uint8_t lg_k, datasketches::resize_factor rf, float p)
  : union_(datasketches::theta_union::builder()
               .set_lg_k(lg_k)
               .set_resize_factor(rf)
               .set_p(p)
               .build()) {}

void ThetaUnionShim::update_with_sketch(const ThetaSketchShim& sketch) {
  union_.update(sketch.inner());
}

void ThetaUnionShim::update_with_compact(const CompactThetaSketchShim& sketch) {
  union_.update(sketch.inner());
}

void ThetaUnionShim::update_with_wrapped(const WrappedCompactThetaSketchShim& sketch) {
  union_.update(sketch.inner());
}

std::unique_ptr<CompactThetaSketchShim> ThetaUnionShim::get_result(bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(union_.get_result(ordered));
}

void ThetaUnionShim::reset() { union_.reset(); }

std::unique_ptr<ThetaUnionShim> new_theta_union(uint8_t lg_k, ResizeFactor rf, float p) {
  return std::make_unique<ThetaUnionShim>(lg_k, to_cpp_resize_factor(rf), p);
}

} // namespace apache_datasketches_rs
