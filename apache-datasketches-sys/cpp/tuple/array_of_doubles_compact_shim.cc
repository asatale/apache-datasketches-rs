#include "array_of_doubles_compact_shim.h"

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

std::unique_ptr<CompactArrayOfDoublesSketchShim> compact_array_of_doubles_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(
      datasketches::compact_array_of_doubles_sketch::deserialize(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
