#include "cpc_union_shim.h"

namespace apache_datasketches_rs {

CpcUnionShim::CpcUnionShim(uint8_t lg_k) : u_(lg_k) {}

void CpcUnionShim::update_sketch(const CpcSketchShim& sketch) {
  u_.update(sketch.inner());
}

std::unique_ptr<CpcSketchShim> CpcUnionShim::get_result() const {
  return std::make_unique<CpcSketchShim>(u_.get_result());
}

std::unique_ptr<CpcUnionShim> new_cpc_union(uint8_t lg_k) {
  return std::make_unique<CpcUnionShim>(lg_k);
}

} // namespace apache_datasketches_rs
