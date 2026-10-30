#pragma once
#include <cstddef>
#include <cstdint>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "bloom_filter.hpp"

namespace apache_datasketches_rs {

class WrappedBloomFilterShim;

// Owns a datasketches::bloom_filter.
class BloomFilterShim {
public:
  explicit BloomFilterShim(datasketches::bloom_filter filter);

  std::unique_ptr<BloomFilterShim> clone_shim() const;

  void update_u64(uint64_t value);
  bool query_u64(uint64_t value) const;
  bool query_and_update_u64(uint64_t value);
  void update_i64(int64_t value);
  bool query_i64(int64_t value) const;
  bool query_and_update_i64(int64_t value);
  void update_u32(uint32_t value);
  bool query_u32(uint32_t value) const;
  bool query_and_update_u32(uint32_t value);
  void update_i32(int32_t value);
  bool query_i32(int32_t value) const;
  bool query_and_update_i32(int32_t value);
  void update_u16(uint16_t value);
  bool query_u16(uint16_t value) const;
  bool query_and_update_u16(uint16_t value);
  void update_i16(int16_t value);
  bool query_i16(int16_t value) const;
  bool query_and_update_i16(int16_t value);
  void update_u8(uint8_t value);
  bool query_u8(uint8_t value) const;
  bool query_and_update_u8(uint8_t value);
  void update_i8(int8_t value);
  bool query_i8(int8_t value) const;
  bool query_and_update_i8(int8_t value);
  void update_f64(double value);
  bool query_f64(double value) const;
  bool query_and_update_f64(double value);
  void update_f32(float value);
  bool query_f32(float value) const;
  bool query_and_update_f32(float value);
  void update_str(rust::Str value);
  bool query_str(rust::Str value) const;
  bool query_and_update_str(rust::Str value);
  void update_bytes(rust::Slice<const uint8_t> value);
  bool query_bytes(rust::Slice<const uint8_t> value) const;
  bool query_and_update_bytes(rust::Slice<const uint8_t> value);

  void union_with_filter(const BloomFilterShim& other);
  void union_with_wrapped(const WrappedBloomFilterShim& other);
  void intersect_with_filter(const BloomFilterShim& other);
  void intersect_with_wrapped(const WrappedBloomFilterShim& other);
  bool is_compatible_with_filter(const BloomFilterShim& other) const;
  bool is_compatible_with_wrapped(const WrappedBloomFilterShim& other) const;
  void invert();
  void reset();

  bool is_empty() const;
  uint64_t get_capacity() const;
  uint16_t get_num_hashes() const;
  uint64_t get_seed() const;
  uint64_t get_bits_used();  // not const upstream: recounts and caches after updates

  uint64_t serialized_size_bytes() const;
  // Writes the image into `out`, which must be exactly serialized_size_bytes()
  // long; throws (surfaced as Err) otherwise.
  void serialize_into(rust::Slice<uint8_t> out) const;
  rust::String to_string_summary() const;

  const datasketches::bloom_filter& inner() const { return filter_; }

private:
  datasketches::bloom_filter filter_;
};

// A read-only bloom_filter wrapping bytes owned by Rust. The safe layer's
// lifetime guarantees the bytes outlive this object. A read-only wrapped
// filter cannot be updated, so it is never "dirty" and get_bits_used never
// writes its cache — which is why it is const here.
class WrappedBloomFilterShim {
public:
  explicit WrappedBloomFilterShim(datasketches::bloom_filter filter);

  bool query_u64(uint64_t value) const;
  bool query_i64(int64_t value) const;
  bool query_u32(uint32_t value) const;
  bool query_i32(int32_t value) const;
  bool query_u16(uint16_t value) const;
  bool query_i16(int16_t value) const;
  bool query_u8(uint8_t value) const;
  bool query_i8(int8_t value) const;
  bool query_f64(double value) const;
  bool query_f32(float value) const;
  bool query_str(rust::Str value) const;
  bool query_bytes(rust::Slice<const uint8_t> value) const;

  bool is_compatible_with_filter(const BloomFilterShim& other) const;
  bool is_compatible_with_wrapped(const WrappedBloomFilterShim& other) const;

  bool is_empty() const;
  uint64_t get_capacity() const;
  uint16_t get_num_hashes() const;
  uint64_t get_seed() const;
  uint64_t get_bits_used() const;

  std::unique_ptr<BloomFilterShim> to_owned() const;
  rust::String to_string_summary() const;

  const datasketches::bloom_filter& inner() const { return filter_; }

private:
  datasketches::bloom_filter filter_;
};

std::unique_ptr<BloomFilterShim> bloom_new_by_accuracy(uint64_t max_distinct_items, double target_fpp, uint64_t seed);
std::unique_ptr<BloomFilterShim> bloom_new_by_size(uint64_t num_bits, uint16_t num_hashes, uint64_t seed);
std::unique_ptr<BloomFilterShim> bloom_deserialize(rust::Slice<const uint8_t> bytes);
std::unique_ptr<WrappedBloomFilterShim> bloom_wrap(rust::Slice<const uint8_t> bytes);
uint16_t bloom_suggest_num_hashes(uint64_t max_distinct_items, uint64_t num_filter_bits);
uint16_t bloom_suggest_num_hashes_for_fpp(double target_fpp);
uint64_t bloom_suggest_num_filter_bits(uint64_t max_distinct_items, double target_fpp);

} // namespace apache_datasketches_rs
