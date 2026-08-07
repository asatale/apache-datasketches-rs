#pragma once
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include "rust/cxx.h"
#include "dyn_summary.h"

namespace apache_datasketches_rs {

class CompactTupleGenericSketchShim;

// The resize factor crosses as its literal multiplier (1, 2, 4, or 8) rather
// than as a shared cxx enum, so this bridge needs no cross-bridge type
// sharing. Throws std::invalid_argument on any other value, which cxx turns
// into Result::Err.
datasketches::resize_factor tuple_generic_resize_factor(uint8_t rf);

class TupleGenericSketchShim {
public:
  TupleGenericSketchShim(uint8_t lg_k, uint8_t rf, float p);

  void update_u64(uint64_t key, const RustSummary& value);
  void update_i64(int64_t key, const RustSummary& value);
  void update_u32(uint32_t key, const RustSummary& value);
  void update_i32(int32_t key, const RustSummary& value);
  void update_u16(uint16_t key, const RustSummary& value);
  void update_i16(int16_t key, const RustSummary& value);
  void update_u8(uint8_t key, const RustSummary& value);
  void update_i8(int8_t key, const RustSummary& value);
  void update_f64(double key, const RustSummary& value);
  void update_str(rust::Str key, const RustSummary& value);
  void update_bytes(rust::Slice<const uint8_t> key, const RustSummary& value);

  void trim();
  void reset();

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  std::unique_ptr<CompactTupleGenericSketchShim> compact(bool ordered) const;

  const dyn_update_sketch& inner() const { return sketch_; }

private:
  dyn_update_sketch sketch_;
};

std::unique_ptr<TupleGenericSketchShim> new_tuple_generic_sketch(uint8_t lg_k, uint8_t rf, float p);

} // namespace apache_datasketches_rs
