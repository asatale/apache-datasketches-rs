#include "bloom_filter_shim.h"

#include <string>
#include <utility>

namespace apache_datasketches_rs {

BloomFilterShim::BloomFilterShim(datasketches::bloom_filter filter) : filter_(std::move(filter)) {}

std::unique_ptr<BloomFilterShim> BloomFilterShim::clone_shim() const {
  return std::make_unique<BloomFilterShim>(filter_);
}

// Every update_* calls upstream's query_and_update and discards the answer.
// Upstream's update (internal_update, bloom_filter_impl.hpp:644-654) marks the
// cached bits-set count dirty; query_and_update (internal_query_and_update,
// :742-755) instead adds to that cached count and clears the dirty flag via
// update_num_bits_set (:552-558) -- without recounting first. So an update
// followed by a query_and_update left a stale count: get_bits_used reported
// too few bits, serialize wrote a count that upstream's own deserialize then
// rejected ("invalid number of bits set"), and when the stale count was 0
// is_empty() turned true, so internal_query (:843) answered false for items
// that had been added -- a false negative. query_and_update sets the same
// bits as update, and keeps the count exact without ever going dirty.
void BloomFilterShim::update_u64(uint64_t value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_u64(uint64_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_u64(uint64_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_i64(int64_t value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_i64(int64_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_i64(int64_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_u32(uint32_t value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_u32(uint32_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_u32(uint32_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_i32(int32_t value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_i32(int32_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_i32(int32_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_u16(uint16_t value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_u16(uint16_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_u16(uint16_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_i16(int16_t value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_i16(int16_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_i16(int16_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_u8(uint8_t value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_u8(uint8_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_u8(uint8_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_i8(int8_t value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_i8(int8_t value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_i8(int8_t value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_f64(double value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_f64(double value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_f64(double value) { return filter_.query_and_update(value); }
void BloomFilterShim::update_f32(float value) { (void)filter_.query_and_update(value); }
bool BloomFilterShim::query_f32(float value) const { return filter_.query(value); }
bool BloomFilterShim::query_and_update_f32(float value) { return filter_.query_and_update(value); }

// Strings forward the borrowed bytes to the (data, length) overload. The
// empty guard is replicated because upstream's std::string overloads return
// early on an empty string (bloom_filter_impl.hpp update/query/
// query_and_update(const std::string&)) and the (data, length) overload
// would hash it instead.
void BloomFilterShim::update_str(rust::Str value) {
  if (value.empty()) return;
  (void)filter_.query_and_update(value.data(), value.size());
}
bool BloomFilterShim::query_str(rust::Str value) const {
  if (value.empty()) return false;
  return filter_.query(value.data(), value.size());
}
bool BloomFilterShim::query_and_update_str(rust::Str value) {
  if (value.empty()) return false;
  return filter_.query_and_update(value.data(), value.size());
}
void BloomFilterShim::update_bytes(rust::Slice<const uint8_t> value) {
  (void)filter_.query_and_update(value.data(), value.size());
}
bool BloomFilterShim::query_bytes(rust::Slice<const uint8_t> value) const {
  return filter_.query(value.data(), value.size());
}
bool BloomFilterShim::query_and_update_bytes(rust::Slice<const uint8_t> value) {
  return filter_.query_and_update(value.data(), value.size());
}

void BloomFilterShim::union_with_filter(const BloomFilterShim& other) { filter_.union_with(other.inner()); }
void BloomFilterShim::union_with_wrapped(const WrappedBloomFilterShim& other) { filter_.union_with(other.inner()); }
void BloomFilterShim::intersect_with_filter(const BloomFilterShim& other) { filter_.intersect(other.inner()); }
void BloomFilterShim::intersect_with_wrapped(const WrappedBloomFilterShim& other) { filter_.intersect(other.inner()); }
bool BloomFilterShim::is_compatible_with_filter(const BloomFilterShim& other) const { return filter_.is_compatible(other.inner()); }
bool BloomFilterShim::is_compatible_with_wrapped(const WrappedBloomFilterShim& other) const { return filter_.is_compatible(other.inner()); }
void BloomFilterShim::invert() { filter_.invert(); }
void BloomFilterShim::reset() { filter_.reset(); }

bool BloomFilterShim::is_empty() const { return filter_.is_empty(); }
uint64_t BloomFilterShim::get_capacity() const { return filter_.get_capacity(); }
uint16_t BloomFilterShim::get_num_hashes() const { return filter_.get_num_hashes(); }
uint64_t BloomFilterShim::get_seed() const { return filter_.get_seed(); }
uint64_t BloomFilterShim::get_bits_used() { return filter_.get_bits_used(); }

// Whole buffer handed over at once; see cpc_sketch_shim.cc for why not a
// rust::Vec built element by element.
std::unique_ptr<std::vector<uint8_t>> BloomFilterShim::serialize() const {
  return std::make_unique<std::vector<uint8_t>>(filter_.serialize());
}

rust::String BloomFilterShim::to_string_summary() const {
  return rust::String(std::string(filter_.to_string().c_str()));
}

WrappedBloomFilterShim::WrappedBloomFilterShim(datasketches::bloom_filter filter) : filter_(std::move(filter)) {}

bool WrappedBloomFilterShim::query_u64(uint64_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_i64(int64_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_u32(uint32_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_i32(int32_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_u16(uint16_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_i16(int16_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_u8(uint8_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_i8(int8_t value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_f64(double value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_f32(float value) const { return filter_.query(value); }
bool WrappedBloomFilterShim::query_str(rust::Str value) const {
  if (value.empty()) return false;
  return filter_.query(value.data(), value.size());
}
bool WrappedBloomFilterShim::query_bytes(rust::Slice<const uint8_t> value) const {
  return filter_.query(value.data(), value.size());
}
bool WrappedBloomFilterShim::is_compatible_with_filter(const BloomFilterShim& other) const { return filter_.is_compatible(other.inner()); }
bool WrappedBloomFilterShim::is_compatible_with_wrapped(const WrappedBloomFilterShim& other) const { return filter_.is_compatible(other.inner()); }
bool WrappedBloomFilterShim::is_empty() const { return filter_.is_empty(); }
uint64_t WrappedBloomFilterShim::get_capacity() const { return filter_.get_capacity(); }
uint16_t WrappedBloomFilterShim::get_num_hashes() const { return filter_.get_num_hashes(); }
uint64_t WrappedBloomFilterShim::get_seed() const { return filter_.get_seed(); }
// Read-only wrapped filters are never dirty, so upstream's non-const
// get_bits_used returns the cached count without writing; the const_cast is
// therefore a read.
uint64_t WrappedBloomFilterShim::get_bits_used() const {
  return const_cast<datasketches::bloom_filter&>(filter_).get_bits_used();
}

std::unique_ptr<BloomFilterShim> WrappedBloomFilterShim::to_owned() const {
  const auto bytes = filter_.serialize();
  return std::make_unique<BloomFilterShim>(datasketches::bloom_filter::deserialize(bytes.data(), bytes.size()));
}

rust::String WrappedBloomFilterShim::to_string_summary() const {
  return rust::String(std::string(filter_.to_string().c_str()));
}

std::unique_ptr<BloomFilterShim> bloom_new_by_accuracy(uint64_t max_distinct_items, double target_fpp, uint64_t seed) {
  return std::make_unique<BloomFilterShim>(
      datasketches::bloom_filter::builder::create_by_accuracy(max_distinct_items, target_fpp, seed));
}

std::unique_ptr<BloomFilterShim> bloom_new_by_size(uint64_t num_bits, uint16_t num_hashes, uint64_t seed) {
  return std::make_unique<BloomFilterShim>(
      datasketches::bloom_filter::builder::create_by_size(num_bits, num_hashes, seed));
}

std::unique_ptr<BloomFilterShim> bloom_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<BloomFilterShim>(datasketches::bloom_filter::deserialize(bytes.data(), bytes.size()));
}

std::unique_ptr<WrappedBloomFilterShim> bloom_wrap(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<WrappedBloomFilterShim>(datasketches::bloom_filter::wrap(bytes.data(), bytes.size()));
}

uint16_t bloom_suggest_num_hashes(uint64_t max_distinct_items, uint64_t num_filter_bits) {
  return datasketches::bloom_filter::builder::suggest_num_hashes(max_distinct_items, num_filter_bits);
}

uint16_t bloom_suggest_num_hashes_for_fpp(double target_fpp) {
  return datasketches::bloom_filter::builder::suggest_num_hashes(target_fpp);
}

uint64_t bloom_suggest_num_filter_bits(uint64_t max_distinct_items, double target_fpp) {
  return datasketches::bloom_filter::builder::suggest_num_filter_bits(max_distinct_items, target_fpp);
}

} // namespace apache_datasketches_rs
