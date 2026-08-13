#pragma once
#include <cstddef>
#include <cstdint>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "cpc_sketch.hpp"

namespace apache_datasketches_rs {

class CpcSketchShim {
public:
  explicit CpcSketchShim(uint8_t lg_k);
  explicit CpcSketchShim(datasketches::cpc_sketch sketch);

  void update_u64(uint64_t value);
  void update_i64(int64_t value);
  void update_u32(uint32_t value);
  void update_i32(int32_t value);
  void update_u16(uint16_t value);
  void update_i16(int16_t value);
  void update_u8(uint8_t value);
  void update_i8(int8_t value);
  void update_f64(double value);
  void update_f32(float value);
  void update_str(rust::Str value);
  void update_bytes(rust::Slice<const uint8_t> value);

  bool is_empty() const;
  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  uint8_t get_lg_k() const;
  rust::String to_string_summary() const;

  std::unique_ptr<std::vector<uint8_t>> serialize() const;

  const datasketches::cpc_sketch& inner() const { return sketch_; }

private:
  datasketches::cpc_sketch sketch_;
};

std::unique_ptr<CpcSketchShim> new_cpc_sketch(uint8_t lg_k);
std::unique_ptr<CpcSketchShim> cpc_sketch_deserialize(rust::Slice<const uint8_t> bytes);
size_t cpc_sketch_max_serialized_size_bytes(uint8_t lg_k);
void cpc_init();

} // namespace apache_datasketches_rs
