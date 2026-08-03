#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "array_of_doubles_sketch.hpp"
#include "array_of_doubles_sketch_shim.h"
#include "array_of_doubles_compact_shim.h"

namespace apache_datasketches_rs {

class ArrayOfDoublesAnotBShim {
public:
  ArrayOfDoublesAnotBShim();

  std::unique_ptr<CompactArrayOfDoublesSketchShim> compute_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactArrayOfDoublesSketchShim> compute_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactArrayOfDoublesSketchShim> compute_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactArrayOfDoublesSketchShim> compute_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b, bool ordered) const;

private:
  datasketches::array_of_doubles_a_not_b a_not_b_;
};

std::unique_ptr<ArrayOfDoublesAnotBShim> new_array_of_doubles_a_not_b();

} // namespace apache_datasketches_rs
