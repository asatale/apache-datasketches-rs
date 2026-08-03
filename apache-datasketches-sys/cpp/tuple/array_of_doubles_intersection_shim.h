#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"

namespace apache_datasketches_rs {

// Upstream's array_of_doubles_intersection has NO default Policy template
// argument ("no default policy since it is not clear in general", per its own
// header). v1 picks sum-on-collision, reusing the union's policy — the same
// choice upstream's own array_of_doubles_sketch_test.cpp makes. This alias is
// also reused by the jaccard shim.
using aod_intersection =
    datasketches::array_of_doubles_intersection<datasketches::default_array_of_doubles_union_policy>;

class ArrayOfDoublesIntersectionShim {
public:
  explicit ArrayOfDoublesIntersectionShim(uint8_t num_values);

  void update_with_sketch(const ArrayOfDoublesSketchShim& sketch);
  void update_with_compact(const CompactArrayOfDoublesSketchShim& sketch);

  std::unique_ptr<CompactArrayOfDoublesSketchShim> get_result(bool ordered) const;
  bool has_result() const;

private:
  aod_intersection intersection_;
};

std::unique_ptr<ArrayOfDoublesIntersectionShim> new_array_of_doubles_intersection(uint8_t num_values);

} // namespace apache_datasketches_rs
