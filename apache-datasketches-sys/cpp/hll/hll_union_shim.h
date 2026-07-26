#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "hll.hpp"
#include "hll_sketch_shim.h"

// We deliberately do NOT `#include "hll.rs.h"` here, for the same reason
// documented in hll_sketch_shim.h: the generated header's `include!`
// directives re-enter this header while it's still being processed, and its
// `using HllUnionShim = ...` type alias requires the HllUnionShim class
// (defined below) to already be complete. hll_sketch_shim.h's forward
// declaration of `TargetHllType` (included above) is sufficient for this
// header's declarations; the full generated header is pulled in by
// hll_union_shim.cc after this header.

namespace apache_datasketches_rs {

class HllUnionShim {
public:
  explicit HllUnionShim(uint8_t lg_max_k);

  void update_sketch(const HllSketchShim& sketch);
  void update_u64(uint64_t value);
  void update_i64(int64_t value);
  void update_f64(double value);
  void update_str(rust::Str value);
  void update_bytes(rust::Slice<const uint8_t> value);

  std::unique_ptr<HllSketchShim> get_result(TargetHllType tgt_type) const;

  // hll_union has no native serialize/deserialize (only hll_sketch does, per
  // vendor/datasketches-cpp/hll/include/hll.hpp): a union's accumulation
  // state isn't independently serializable, only its result sketch is. These
  // convenience methods serialize get_result(tgt_type) directly so callers
  // don't need to round-trip through get_result() themselves.
  rust::Vec<uint8_t> serialize_compact(TargetHllType tgt_type) const;
  rust::Vec<uint8_t> serialize_updatable(TargetHllType tgt_type) const;

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  void reset();

private:
  datasketches::hll_union u_;
};

std::unique_ptr<HllUnionShim> new_hll_union(uint8_t lg_max_k);

} // namespace apache_datasketches_rs
