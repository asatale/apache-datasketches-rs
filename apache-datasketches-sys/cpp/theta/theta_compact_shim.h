#pragma once
#include <cstdint>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "theta_sketch.hpp"
#include "theta_sketch_shim.h"

namespace apache_datasketches_rs {

class CompactThetaSketchShim {
public:
  explicit CompactThetaSketchShim(datasketches::compact_theta_sketch sketch);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  std::unique_ptr<std::vector<uint8_t>> serialize_compact() const;
  std::unique_ptr<std::vector<uint8_t>> serialize_compressed() const;

  const datasketches::compact_theta_sketch& inner() const { return sketch_; }

private:
  datasketches::compact_theta_sketch sketch_;
};

// Used by theta_sketch_shim.cc (Task 5) to implement ThetaSketchShim::compact().
std::unique_ptr<CompactThetaSketchShim> theta_sketch_compact(const ThetaSketchShim& sketch, bool ordered);

std::unique_ptr<CompactThetaSketchShim> compact_theta_sketch_deserialize(rust::Slice<const uint8_t> bytes);

} // namespace apache_datasketches_rs
