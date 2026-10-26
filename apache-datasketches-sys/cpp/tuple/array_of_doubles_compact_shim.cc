#include "array_of_doubles_compact_shim.h"

#include <cstring>
#include <stdexcept>
#include <string>

namespace apache_datasketches_rs {

CompactArrayOfDoublesSketchShim::CompactArrayOfDoublesSketchShim(datasketches::compact_array_of_doubles_sketch sketch)
  : sketch_(std::move(sketch)) {}

double CompactArrayOfDoublesSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double CompactArrayOfDoublesSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double CompactArrayOfDoublesSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool CompactArrayOfDoublesSketchShim::is_empty() const { return sketch_.is_empty(); }
bool CompactArrayOfDoublesSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool CompactArrayOfDoublesSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double CompactArrayOfDoublesSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t CompactArrayOfDoublesSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }
uint8_t CompactArrayOfDoublesSketchShim::get_num_values() const { return sketch_.get_num_values(); }

// Returned as heap std::vectors rather than rust::Vecs built by hand.
// rust::Vec::push_back goes through emplace_back, which makes two
// non-inlinable extern "C" calls back into Rust per element; over a full
// sketch's worth of entries that dominated everything else the call does. A
// unique_ptr hands each buffer over whole and leaves the Rust side one memcpy.
std::unique_ptr<std::vector<uint64_t>> CompactArrayOfDoublesSketchShim::entry_hashes() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(sketch_.get_num_retained());
  for (const auto& entry : sketch_) out->push_back(entry.first);
  return out;
}

std::unique_ptr<std::vector<double>> CompactArrayOfDoublesSketchShim::entry_values() const {
  auto out = std::make_unique<std::vector<double>>();
  out->reserve(static_cast<size_t>(sketch_.get_num_retained()) * sketch_.get_num_values());
  for (const auto& entry : sketch_) {
    for (uint8_t i = 0; i < entry.second.size(); ++i) out->push_back(entry.second[i]);
  }
  return out;
}

std::unique_ptr<std::vector<uint8_t>> CompactArrayOfDoublesSketchShim::serialize() const {
  return std::make_unique<std::vector<uint8_t>>(sketch_.serialize());
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> array_of_doubles_sketch_compact(const ArrayOfDoublesSketchShim& sketch, bool ordered) {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(sketch.inner().compact(ordered));
}

namespace {

// Upstream compact_array_tuple_sketch::deserialize checks the seed hash only
// when the image is flagged HAS_ENTRIES (array_tuple_sketch_impl.hpp:199, and
// :156 for the stream overload). An image that is not flagged empty but has
// no entries (theta < 1, every hash above it) is accepted with whatever seed
// hash it carries. The set operations do check it -- theta_union_base_impl.hpp
// :43, theta_intersection_base_impl.hpp:40, theta_set_difference_base_impl.hpp
// :41 -- and throw from bridge functions that are not declared Result, which
// aborts the process. This crate never exposes a seed (everything is built
// with DEFAULT_SEED), so any non-empty image with another seed hash is invalid
// input: reject it here, before upstream sees it. An empty image is exempt, as
// it is everywhere upstream: it has no hashes, so its seed hash is meaningless
// (5.3.0 serializes it as 0).
void validate_aod_seed_hash(const uint8_t* data, size_t size) {
  using sketch = datasketches::compact_array_of_doubles_sketch;
  // Byte layout, from compact_array_tuple_sketch::serialize
  // (array_tuple_sketch_impl.hpp:112-128): 0 preamble_longs, 1 serial_version,
  // 2 family, 3 type, 4 flags, 5 num_values, 6-7 seed_hash, 8-15 theta.
  // Upstream requires those 16 bytes (ensure_minimum_memory(size, 16), :180)
  // and checks version, family and type itself (:195-197); leave anything that
  // fails those to upstream so its error messages are unchanged.
  constexpr size_t HEADER_BYTES = 16;
  if (size < HEADER_BYTES) return;
  if (data[1] != sketch::SERIAL_VERSION || data[2] != sketch::SKETCH_FAMILY
      || data[3] != sketch::SKETCH_TYPE) {
    return;
  }
  const uint8_t flags = data[4];
  if (flags & (1 << sketch::flags::IS_EMPTY)) return;
  uint16_t seed_hash;
  std::memcpy(&seed_hash, data + 6, sizeof(seed_hash));
  // compute_seed_hash is declared in the global namespace (MurmurHash3.h).
  const uint16_t expected = ::compute_seed_hash(datasketches::DEFAULT_SEED);
  if (seed_hash != expected) {
    throw std::invalid_argument("ArrayOfDoubles sketch seed hash mismatch: expected "
        + std::to_string(expected) + ", actual " + std::to_string(seed_hash));
  }
}

}  // namespace

std::unique_ptr<CompactArrayOfDoublesSketchShim> compact_array_of_doubles_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  validate_aod_seed_hash(bytes.data(), bytes.size());
  return std::make_unique<CompactArrayOfDoublesSketchShim>(
      datasketches::compact_array_of_doubles_sketch::deserialize(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
