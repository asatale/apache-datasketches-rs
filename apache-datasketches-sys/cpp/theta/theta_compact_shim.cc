#include "theta_compact_shim.h"

namespace apache_datasketches_rs {

CompactThetaSketchShim::CompactThetaSketchShim(datasketches::compact_theta_sketch sketch)
  : sketch_(std::move(sketch)) {}

double CompactThetaSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double CompactThetaSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double CompactThetaSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool CompactThetaSketchShim::is_empty() const { return sketch_.is_empty(); }
bool CompactThetaSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool CompactThetaSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double CompactThetaSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t CompactThetaSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

rust::Vec<uint8_t> CompactThetaSketchShim::serialize_compact() const {
  auto bytes = sketch_.serialize();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

rust::Vec<uint8_t> CompactThetaSketchShim::serialize_compressed() const {
  auto bytes = sketch_.serialize_compressed();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

std::unique_ptr<CompactThetaSketchShim> theta_sketch_compact(const ThetaSketchShim& sketch, bool ordered) {
  return std::make_unique<CompactThetaSketchShim>(
      datasketches::compact_theta_sketch(sketch.inner(), ordered));
}

std::unique_ptr<CompactThetaSketchShim> compact_theta_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<CompactThetaSketchShim>(
      datasketches::compact_theta_sketch::deserialize(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
