#include "bloom_filter_shim.h"

#include <cstring>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>
#include <cstdint>

namespace apache_datasketches_rs {

namespace {

// Reads a native-endian value with memcpy (the bytes need not be aligned),
// after checking it lies within the buffer. This matches upstream, whose
// copy_from_mem also memcpys native values (little-endian on every platform
// this crate builds for).
template<typename V>
V read_at(const uint8_t* data, size_t size, size_t offset, const char* what) {
  if (offset > size || size - offset < sizeof(V)) {
    throw std::invalid_argument(std::string("Bloom filter bytes too short to hold ") + what);
  }
  V value;
  std::memcpy(&value, data + offset, sizeof(V));
  return value;
}

[[noreturn]] void reject(const std::string& why) {
  throw std::invalid_argument("Invalid Bloom filter bytes: " + why);
}

// Checks a serialized image before upstream's deserialize or wrap sees it.
// Line numbers are bloom_filter_impl.hpp at 5.3.0, internal_deserialize_or_wrap
// (:346-421) and serialize (:424-478). Layout: 0 preamble_longs, 1 serial
// version, 2 family, 3 flags, 4-5 num_hashes, 6-7 unused, 8-15 seed, 16-19
// num_longs, 20-23 unused; if non-empty, 24-31 bits-set count (or the dirty
// sentinel) and then num_longs * 8 bytes of bit array. Trailing bytes are
// accepted, as upstream does.
// Upstream's constants (bloom_filter.hpp:578-587) are private; these copy
// them.
constexpr uint8_t PREAMBLE_LONGS_EMPTY = 3;
constexpr uint8_t PREAMBLE_LONGS_STANDARD = 4;
constexpr uint8_t FAMILY_ID = 21;
constexpr uint8_t SER_VER = 1;
constexpr uint8_t EMPTY_FLAG_MASK = 4;
constexpr size_t NUM_BITS_SET_OFFSET_BYTES = 24;
constexpr size_t BIT_ARRAY_OFFSET_BYTES = 32;
constexpr uint64_t MAX_FILTER_SIZE_BITS = (uint64_t{INT32_MAX} - 32) * 8;

void validate_bloom_layout(rust::Slice<const uint8_t> bytes) {
  const uint8_t* data = bytes.data();
  const size_t size = bytes.size();

  const auto preamble_longs = read_at<uint8_t>(data, size, 0, "the preamble size");
  const auto serial_version = read_at<uint8_t>(data, size, 1, "the serial version");
  const auto family = read_at<uint8_t>(data, size, 2, "the family");
  const auto flags = read_at<uint8_t>(data, size, 3, "the flags");

  // Upstream checks these three too (:364-372); checking them first keeps the
  // message right for bytes of another family.
  if (family != FAMILY_ID) {
    reject("family " + std::to_string(family) + ", expected " + std::to_string(FAMILY_ID));
  }
  if (serial_version != SER_VER) {
    reject("serial version " + std::to_string(serial_version) + ", expected "
        + std::to_string(SER_VER));
  }
  // Upstream writes only the empty flag (:432, :462). Any other bit means the
  // bytes are not an image upstream produced.
  if ((flags & static_cast<uint8_t>(~EMPTY_FLAG_MASK)) != 0) {
    reject("unknown flag bits " + std::to_string(flags));
  }
  const bool is_empty = (flags & EMPTY_FLAG_MASK) != 0;
  // Upstream writes 3 preamble longs exactly when empty (:426, :456) but only
  // checks the value is 3 or 4 (:364) and requires prelongs * 8 bytes (:376).
  // A non-empty image claiming 3 passes that with 24 bytes, after which
  // upstream reads the bits-set count at 24..32 and the bit array from 32
  // (:396-399; `end_ptr - ptr` is then negative and converts to a huge
  // size_t, so ensure_minimum_memory passes): reads past the buffer, and a
  // wrapped view whose bit array lies outside the borrowed bytes.
  const uint8_t expected_preamble =
      is_empty ? PREAMBLE_LONGS_EMPTY : PREAMBLE_LONGS_STANDARD;
  if (preamble_longs != expected_preamble) {
    reject("preamble size " + std::to_string(preamble_longs) + " for a"
        + (is_empty ? "n empty" : " non-empty") + " filter, expected "
        + std::to_string(expected_preamble));
  }

  const auto num_hashes = read_at<uint16_t>(data, size, 4, "the number of hashes");
  read_at<uint64_t>(data, size, 8, "the seed");
  // Upstream serializes the word count as int32_t (:438, :468) and reads it
  // back as uint32_t (:385), so a negative count reads as 2^31 or more.
  const auto num_longs = read_at<uint32_t>(data, size, 16, "the bit array length");
  read_at<uint32_t>(data, size, 20, "the unused header bytes");

  // The empty path rejects 0 hashes in the constructor (:64-66); the non-empty
  // path passes num_hashes straight to the unchecked private constructor
  // (:420, :157), and a filter with 0 hashes answers every query true.
  if (num_hashes == 0) {
    reject("number of hashes must be at least 1");
  }
  // Every hashing loop is `for (uint16_t i = 1; i <= num_hashes_; i++)`
  // (internal_update :650, internal_query_and_update :748, internal_query
  // :845): with num_hashes = 65535, i wraps to 0 and the loop never ends, so
  // any update hangs (and so does a query once every probed bit is set).
  if (num_hashes == UINT16_MAX) {
    reject("number of hashes must be at most " + std::to_string(UINT16_MAX - 1));
  }
  // Upstream evaluates `num_longs << 6` (capacity in bits, :392, :420) and
  // `num_longs << 3` (bit array bytes, :398) in 32 bits. From 2^26 words the
  // capacity wraps: a multiple of 2^26 gives capacity 0 (and, from 2^29, a
  // 0-byte bit array), on which update and query_and_update compute
  // `hash % 0` (internal_update :651, internal_query_and_update :749);
  // others give a smaller capacity than the bytes claim. num_longs = 0
  // reaches the same `% 0`. So 1 <= num_longs < 2^26, which also keeps the
  // capacity within MAX_FILTER_SIZE_BITS. (A filter built with more than
  // 2^32 - 64 bits serializes, but upstream cannot read it back.)
  constexpr uint32_t MAX_NUM_LONGS = (UINT32_MAX >> 6);  // 2^26 - 1
  static_assert(uint64_t{MAX_NUM_LONGS} * 64 <= MAX_FILTER_SIZE_BITS,
                "the 32-bit bound is the tighter one");
  if (num_longs == 0 || num_longs > MAX_NUM_LONGS) {
    reject("bit array length " + std::to_string(num_longs) + " words, expected 1 to "
        + std::to_string(MAX_NUM_LONGS));
  }
  if (is_empty) return;

  // The bits-set count itself is checked by upstream against the bit array
  // (validate_bloom_filter_num_bits_set, :40-52), which is safe once the
  // array is known to be in bounds.
  read_at<uint64_t>(data, size, NUM_BITS_SET_OFFSET_BYTES, "the bits-set count");
  const uint64_t num_bytes = uint64_t{num_longs} * sizeof(uint64_t);
  if (size - BIT_ARRAY_OFFSET_BYTES < num_bytes) {
    reject("bit array of " + std::to_string(num_longs) + " words needs "
        + std::to_string(BIT_ARRAY_OFFSET_BYTES + num_bytes) + " bytes, got "
        + std::to_string(size));
  }
}

}  // namespace

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

// Upstream reads a non-empty image's bit array through a
// reinterpret_cast<const uint64_t*> (bit_array_ops.hpp count_num_bits_set, and
// every query on a wrapped view), which is undefined behavior unless the base
// is 8-byte aligned. Both headers (24 and 32 bytes) are multiples of 8, so
// aligning the base aligns the bit array.
bool is_aligned_for_words(const uint8_t* data) {
  return reinterpret_cast<uintptr_t>(data) % alignof(uint64_t) == 0;
}

std::unique_ptr<BloomFilterShim> bloom_deserialize(rust::Slice<const uint8_t> bytes) {
  // The validator reads byte-wise (memcpy), so it is safe on misaligned input.
  validate_bloom_layout(bytes);
  if (is_aligned_for_words(bytes.data())) {
    return std::make_unique<BloomFilterShim>(
        datasketches::bloom_filter::deserialize(bytes.data(), bytes.size()));
  }
  // Deserialize copies into an owned filter, so an aligned copy is harmless.
  std::vector<uint64_t> aligned((bytes.size() + sizeof(uint64_t) - 1) / sizeof(uint64_t));
  std::memcpy(aligned.data(), bytes.data(), bytes.size());
  return std::make_unique<BloomFilterShim>(
      datasketches::bloom_filter::deserialize(aligned.data(), bytes.size()));
}

std::unique_ptr<WrappedBloomFilterShim> bloom_wrap(rust::Slice<const uint8_t> bytes) {
  validate_bloom_layout(bytes);
  // An empty image is rebuilt as an owned filter (only the header is read,
  // byte-wise), so its alignment does not matter. A non-empty one is viewed in
  // place and every query reads the caller's bytes as uint64_t.
  const bool is_empty = (bytes.data()[3] & EMPTY_FLAG_MASK) != 0;
  if (!is_empty && !is_aligned_for_words(bytes.data())) {
    throw std::invalid_argument(
        "Invalid Bloom filter bytes: wrapping requires the bytes to be 8-byte aligned");
  }
  return std::make_unique<WrappedBloomFilterShim>(
      datasketches::bloom_filter::wrap(bytes.data(), bytes.size()));
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
