#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_union.hpp"
#include "theta_sketch_shim.h"
#include "theta_compact_shim.h"
#include "theta_wrapped_shim.h"

namespace apache_datasketches_rs {

enum class ResizeFactor : std::uint8_t;
datasketches::resize_factor to_cpp_resize_factor(ResizeFactor rf);

class ThetaUnionShim {
public:
  ThetaUnionShim(uint8_t lg_k, datasketches::resize_factor rf, float p);

  void update_with_sketch(const ThetaSketchShim& sketch);
  void update_with_compact(const CompactThetaSketchShim& sketch);
  void update_with_wrapped(const WrappedCompactThetaSketchShim& sketch);

  std::unique_ptr<CompactThetaSketchShim> get_result(bool ordered) const;
  void reset();

private:
  datasketches::theta_union union_;
};

std::unique_ptr<ThetaUnionShim> new_theta_union(uint8_t lg_k, ResizeFactor rf, float p);

} // namespace apache_datasketches_rs
