#include "tuple_generic_union_shim.h"

namespace apache_datasketches_rs {

namespace {
dyn_union build_union(uint8_t lg_k, uint8_t rf, float p) {
  // Brace-init: `builder b(Policy())` is a function declaration.
  dyn_union::builder builder{DynUnionPolicy()};
  builder.set_lg_k(lg_k);
  builder.set_resize_factor(tuple_generic_resize_factor(rf));
  builder.set_p(p);
  return builder.build();
}
} // namespace

TupleGenericUnionShim::TupleGenericUnionShim(uint8_t lg_k, uint8_t rf, float p)
  : union_(build_union(lg_k, rf, p)) {}

void TupleGenericUnionShim::update_with_sketch(const TupleGenericSketchShim& sketch) {
  union_.update(sketch.inner());
}

void TupleGenericUnionShim::update_with_compact(const CompactTupleGenericSketchShim& sketch) {
  union_.update(sketch.inner());
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericUnionShim::get_result(bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(union_.get_result(ordered));
}

void TupleGenericUnionShim::reset() { union_.reset(); }

std::unique_ptr<TupleGenericUnionShim> new_tuple_generic_union(uint8_t lg_k, uint8_t rf, float p) {
  return std::make_unique<TupleGenericUnionShim>(lg_k, rf, p);
}

} // namespace apache_datasketches_rs
