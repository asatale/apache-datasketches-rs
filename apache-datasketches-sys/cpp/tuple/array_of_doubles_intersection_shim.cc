#include "array_of_doubles_intersection_shim.h"

namespace apache_datasketches_rs {

// The policy — and therefore num_values — must be supplied at construction
// time; unlike the sketch and union there is no builder for this type.
ArrayOfDoublesIntersectionShim::ArrayOfDoublesIntersectionShim(uint8_t num_values)
  : intersection_(datasketches::DEFAULT_SEED,
                  datasketches::default_array_of_doubles_union_policy(num_values)) {}

void ArrayOfDoublesIntersectionShim::update_with_sketch(const ArrayOfDoublesSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

void ArrayOfDoublesIntersectionShim::update_with_compact(const CompactArrayOfDoublesSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> ArrayOfDoublesIntersectionShim::get_result(bool ordered) const {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(intersection_.get_result(ordered));
}

bool ArrayOfDoublesIntersectionShim::has_result() const { return intersection_.has_result(); }

std::unique_ptr<ArrayOfDoublesIntersectionShim> new_array_of_doubles_intersection(uint8_t num_values) {
  return std::make_unique<ArrayOfDoublesIntersectionShim>(num_values);
}

} // namespace apache_datasketches_rs
