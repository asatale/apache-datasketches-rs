#include "tuple_generic_a_not_b_shim.h"

namespace apache_datasketches_rs {

TupleGenericAnotBShim::TupleGenericAnotBShim() : a_not_b_() {}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericAnotBShim::compute_sketch_sketch(const TupleGenericSketchShim& a, const TupleGenericSketchShim& b, bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericAnotBShim::compute_sketch_compact(const TupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b, bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericAnotBShim::compute_compact_sketch(const CompactTupleGenericSketchShim& a, const TupleGenericSketchShim& b, bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericAnotBShim::compute_compact_compact(const CompactTupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b, bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<TupleGenericAnotBShim> new_tuple_generic_a_not_b() {
  return std::make_unique<TupleGenericAnotBShim>();
}

} // namespace apache_datasketches_rs
