#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"

namespace apache_datasketches_rs {

// Unlike the sketch and compact shims, nothing here mentions RustSummary --
// the union only ever passes shim types across the bridge -- so this shim gets
// its own bridge file (src/tuple_generic_union.rs) and aliases the two sketch
// shim types the ordinary `extern "C++"` way.
class TupleGenericUnionShim {
public:
  TupleGenericUnionShim(uint8_t lg_k, uint8_t rf, float p);

  void update_with_sketch(const TupleGenericSketchShim& sketch);
  void update_with_compact(const CompactTupleGenericSketchShim& sketch);

  std::unique_ptr<CompactTupleGenericSketchShim> get_result(bool ordered) const;
  void reset();

private:
  dyn_union union_;
};

std::unique_ptr<TupleGenericUnionShim> new_tuple_generic_union(uint8_t lg_k, uint8_t rf, float p);

} // namespace apache_datasketches_rs
