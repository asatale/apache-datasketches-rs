#pragma once
#include <cstdint>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "array_of_doubles_sketch_shim.h"

namespace apache_datasketches_rs {

class CompactArrayOfDoublesSketchShim {
public:
  explicit CompactArrayOfDoublesSketchShim(datasketches::compact_array_of_doubles_sketch sketch);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;
  uint8_t get_num_values() const;

  std::unique_ptr<std::vector<uint64_t>> entry_hashes() const;
  std::unique_ptr<std::vector<double>> entry_values() const;

  std::unique_ptr<std::vector<uint8_t>> serialize() const;

  const datasketches::compact_array_of_doubles_sketch& inner() const { return sketch_; }

private:
  datasketches::compact_array_of_doubles_sketch sketch_;
};

// Used by array_of_doubles_sketch_shim.cc to implement
// ArrayOfDoublesSketchShim::compact().
std::unique_ptr<CompactArrayOfDoublesSketchShim> array_of_doubles_sketch_compact(const ArrayOfDoublesSketchShim& sketch, bool ordered);

std::unique_ptr<CompactArrayOfDoublesSketchShim> compact_array_of_doubles_sketch_deserialize(rust::Slice<const uint8_t> bytes);

} // namespace apache_datasketches_rs
