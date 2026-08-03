#include "array_of_doubles_a_not_b_shim.h"

namespace apache_datasketches_rs {

ArrayOfDoublesAnotBShim::ArrayOfDoublesAnotBShim() : a_not_b_() {}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesAnotBShim::compute_sketch_sketch(const ArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b, bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesAnotBShim::compute_sketch_compact(const ArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b, bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesAnotBShim::compute_compact_sketch(const CompactArrayOfDoublesSketchShim& a, const ArrayOfDoublesSketchShim& b, bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesAnotBShim::compute_compact_compact(const CompactArrayOfDoublesSketchShim& a, const CompactArrayOfDoublesSketchShim& b, bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<ArrayOfDoublesAnotBShim> new_array_of_doubles_a_not_b() {
  return std::make_unique<ArrayOfDoublesAnotBShim>();
}

} // namespace apache_datasketches_rs
