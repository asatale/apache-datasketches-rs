#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_a_not_b.hpp"
#include "theta_sketch_shim.h"
#include "theta_compact_shim.h"
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

class ThetaAnotBShim {
public:
  ThetaAnotBShim();

  std::unique_ptr<CompactThetaSketchShim> compute_sketch_sketch(const ThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_sketch_compact(const ThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_sketch_wrapped(const ThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_compact_sketch(const CompactThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_compact_compact(const CompactThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_compact_wrapped(const CompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_wrapped_sketch(const WrappedCompactThetaSketchShim& a, const ThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_wrapped_compact(const WrappedCompactThetaSketchShim& a, const CompactThetaSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactThetaSketchShim> compute_wrapped_wrapped(const WrappedCompactThetaSketchShim& a, const WrappedCompactThetaSketchShim& b, bool ordered) const;

private:
  datasketches::theta_a_not_b a_not_b_;
};

std::unique_ptr<ThetaAnotBShim> new_theta_a_not_b();

} // namespace apache_datasketches_rs
