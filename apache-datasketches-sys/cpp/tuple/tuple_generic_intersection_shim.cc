#include "tuple_generic_intersection_shim.h"

namespace apache_datasketches_rs {

TupleGenericIntersectionShim::TupleGenericIntersectionShim()
  : intersection_(datasketches::DEFAULT_SEED, DynIntersectionPolicy()) {}

void TupleGenericIntersectionShim::update_with_sketch(const TupleGenericSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

void TupleGenericIntersectionShim::update_with_compact(const CompactTupleGenericSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericIntersectionShim::get_result(bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(intersection_.get_result(ordered));
}

bool TupleGenericIntersectionShim::has_result() const { return intersection_.has_result(); }

std::unique_ptr<TupleGenericIntersectionShim> new_tuple_generic_intersection() {
  return std::make_unique<TupleGenericIntersectionShim>();
}

} // namespace apache_datasketches_rs
