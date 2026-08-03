#include "array_of_doubles_union_shim.h"

namespace apache_datasketches_rs {

ArrayOfDoublesUnionShim::ArrayOfDoublesUnionShim(uint8_t lg_k, datasketches::resize_factor rf, float p, uint8_t num_values)
  : union_(datasketches::array_of_doubles_union::builder(
               datasketches::default_array_of_doubles_union_policy(num_values))
               .set_lg_k(lg_k)
               .set_resize_factor(rf)
               .set_p(p)
               .build()) {}

void ArrayOfDoublesUnionShim::update_with_sketch(const ArrayOfDoublesSketchShim& sketch) {
  union_.update(sketch.inner());
}

void ArrayOfDoublesUnionShim::update_with_compact(const CompactArrayOfDoublesSketchShim& sketch) {
  union_.update(sketch.inner());
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesUnionShim::get_result(bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(union_.get_result(ordered));
}

void ArrayOfDoublesUnionShim::reset() { union_.reset(); }

std::unique_ptr<ArrayOfDoublesUnionShim> new_array_of_doubles_union(uint8_t lg_k, TupleResizeFactor rf, float p, uint8_t num_values) {
  return std::make_unique<ArrayOfDoublesUnionShim>(lg_k, to_cpp_tuple_resize_factor(rf), p, num_values);
}

} // namespace apache_datasketches_rs
