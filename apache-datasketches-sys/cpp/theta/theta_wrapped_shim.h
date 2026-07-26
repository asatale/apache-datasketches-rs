#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "theta_sketch.hpp"

namespace apache_datasketches_rs {

class WrappedCompactThetaSketchShim {
public:
  explicit WrappedCompactThetaSketchShim(datasketches::wrapped_compact_theta_sketch sketch);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  const datasketches::wrapped_compact_theta_sketch& inner() const { return sketch_; }

private:
  datasketches::wrapped_compact_theta_sketch sketch_;
};

std::unique_ptr<WrappedCompactThetaSketchShim> wrapped_compact_theta_sketch_wrap(rust::Slice<const uint8_t> bytes);

} // namespace apache_datasketches_rs
