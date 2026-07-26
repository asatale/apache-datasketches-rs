#include "theta_a_not_b_shim.h"

namespace apache_datasketches_rs {

ThetaAnotBShim::ThetaAnotBShim() : a_not_b_() {}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_sketch_sketch(
    const ThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_sketch_compact(
    const ThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_sketch_wrapped(
    const ThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_compact_sketch(
    const CompactThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_compact_compact(
    const CompactThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_compact_wrapped(
    const CompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_wrapped_sketch(
    const WrappedCompactThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_wrapped_compact(
    const WrappedCompactThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> ThetaAnotBShim::compute_wrapped_wrapped(
    const WrappedCompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const {
  return std::make_unique<CompactThetaSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<ThetaAnotBShim> new_theta_a_not_b() {
  return std::make_unique<ThetaAnotBShim>();
}

} // namespace apache_datasketches_rs
