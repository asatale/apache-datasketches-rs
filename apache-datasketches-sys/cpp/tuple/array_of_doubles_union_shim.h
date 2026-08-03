#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"

namespace apache_datasketches_rs {

// Forward declarations only — TupleResizeFactor's definition comes from the
// cxx-generated header, and to_cpp_tuple_resize_factor is defined once in
// array_of_doubles_sketch_shim.cc. Both translation units end up in the same
// static library, so one definition satisfies the ODR. Same pattern as
// theta_union_shim.h.
enum class TupleResizeFactor : std::uint8_t;
datasketches::resize_factor to_cpp_tuple_resize_factor(TupleResizeFactor rf);

class ArrayOfDoublesUnionShim {
public:
  ArrayOfDoublesUnionShim(uint8_t lg_k, datasketches::resize_factor rf, float p, uint8_t num_values);

  void update_with_sketch(const ArrayOfDoublesSketchShim& sketch);
  void update_with_compact(const CompactArrayOfDoublesSketchShim& sketch);

  std::unique_ptr<CompactArrayOfDoublesSketchShim> get_result(bool ordered) const;
  void reset();

private:
  datasketches::array_of_doubles_union union_;
};

std::unique_ptr<ArrayOfDoublesUnionShim> new_array_of_doubles_union(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values);

} // namespace apache_datasketches_rs
