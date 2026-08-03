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

rust::Vec<uint64_t> CompactArrayOfDoublesSketchShim::entry_hashes() const {
  rust::Vec<uint64_t> out;
  for (const auto& entry : sketch_) out.push_back(entry.first);
  return out;
}

rust::Vec<double> CompactArrayOfDoublesSketchShim::entry_values() const {
  rust::Vec<double> out;
  for (const auto& entry : sketch_) {
    for (uint8_t i = 0; i < entry.second.size(); ++i) out.push_back(entry.second[i]);
  }
  return out;
}

rust::Vec<uint8_t> CompactArrayOfDoublesSketchShim::serialize() const {
  auto bytes = sketch_.serialize();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> array_of_doubles_sketch_compact(const ArrayOfDoublesSketchShim& sketch, bool ordered) {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(sketch.inner().compact(ordered));
}

std::unique_ptr<CompactArrayOfDoublesSketchShim> compact_array_of_doubles_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<CompactArrayOfDoublesSketchShim>(
      datasketches::compact_array_of_doubles_sketch::deserialize(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
