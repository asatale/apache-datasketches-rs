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

// Returned as a heap std::vector rather than a rust::Vec built by hand.
// rust::Vec::push_back goes through emplace_back, which makes two
// non-inlinable extern "C" calls back into Rust per element; over a
// serialized buffer that is two boundary crossings per byte, and it dominated
// everything else the call does. A unique_ptr hands the buffer over whole and
// leaves the Rust side one memcpy to do.
std::unique_ptr<std::vector<uint8_t>> CompactThetaSketchShim::serialize_compact() const {
  return std::make_unique<std::vector<uint8_t>>(sketch_.serialize());
}

std::unique_ptr<std::vector<uint8_t>> CompactThetaSketchShim::serialize_compressed() const {
  return std::make_unique<std::vector<uint8_t>>(sketch_.serialize_compressed());
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
