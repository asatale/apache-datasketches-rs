#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"

namespace apache_datasketches_rs {

// Like the union shim, nothing here mentions RustSummary -- the intersection
// only ever passes shim types across the bridge -- so this gets its own bridge
// file (src/tuple_generic_intersection.rs) and aliases the two sketch shim
// types the ordinary `extern "C++"` way.
//
// get_result() is declared `Result<..>` on the bridge side: upstream throws
// std::invalid_argument("calling get_result() before calling update() is
// undefined") when has_result() is false (theta_intersection_base_impl.hpp,
// get_result). An intersection with no operands is the infinite "universe",
// which is NOT the same as an empty result -- disjoint operands do produce a
// valid, empty result.
class TupleGenericIntersectionShim {
public:
  TupleGenericIntersectionShim();

  void update_with_sketch(const TupleGenericSketchShim& sketch);
  void update_with_compact(const CompactTupleGenericSketchShim& sketch);

  std::unique_ptr<CompactTupleGenericSketchShim> get_result(bool ordered) const;
  bool has_result() const;

private:
  dyn_intersection intersection_;
};

std::unique_ptr<TupleGenericIntersectionShim> new_tuple_generic_intersection();

} // namespace apache_datasketches_rs
