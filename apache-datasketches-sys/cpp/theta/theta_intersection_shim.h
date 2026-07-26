#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_intersection.hpp"
#include "theta_sketch_shim.h"
#include "theta_compact_shim.h"
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

class ThetaIntersectionShim {
public:
  ThetaIntersectionShim();

  void update_with_sketch(const ThetaSketchShim& sketch);
  void update_with_compact(const CompactThetaSketchShim& sketch);
  void update_with_wrapped(const WrappedCompactThetaSketchShim& sketch);

  std::unique_ptr<CompactThetaSketchShim> get_result(bool ordered) const;
  bool has_result() const;

private:
  datasketches::theta_intersection intersection_;
};

std::unique_ptr<ThetaIntersectionShim> new_theta_intersection();

} // namespace apache_datasketches_rs
