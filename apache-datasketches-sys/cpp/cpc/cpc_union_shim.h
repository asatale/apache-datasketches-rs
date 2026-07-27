#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "cpc_union.hpp"
#include "cpc_sketch_shim.h"

namespace apache_datasketches_rs {

class CpcUnionShim {
public:
  explicit CpcUnionShim(uint8_t lg_k);

  void update_sketch(const CpcSketchShim& sketch);

  std::unique_ptr<CpcSketchShim> get_result() const;

private:
  datasketches::cpc_union u_;
};

std::unique_ptr<CpcUnionShim> new_cpc_union(uint8_t lg_k);

} // namespace apache_datasketches_rs
