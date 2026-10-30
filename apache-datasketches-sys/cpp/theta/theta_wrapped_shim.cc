#include "theta_wrapped_shim.h"

#include <cstdint>
#include <stdexcept>

namespace apache_datasketches_rs {

WrappedCompactThetaSketchShim::WrappedCompactThetaSketchShim(datasketches::wrapped_compact_theta_sketch sketch)
  : sketch_(std::move(sketch)) {}

double WrappedCompactThetaSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double WrappedCompactThetaSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double WrappedCompactThetaSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool WrappedCompactThetaSketchShim::is_empty() const { return sketch_.is_empty(); }
bool WrappedCompactThetaSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool WrappedCompactThetaSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double WrappedCompactThetaSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t WrappedCompactThetaSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

std::unique_ptr<WrappedCompactThetaSketchShim> wrapped_compact_theta_sketch_wrap(rust::Slice<const uint8_t> bytes) {
  // The view keeps pointers into the caller's bytes, and both the parser
  // (u16/u64 header fields) and the entry iterator (u64 entries) read them
  // through typed pointers, so the base must be 8-byte aligned for every
  // image kind, empty and compressed included. Anything shorter than the
  // 8-byte preamble is left to upstream's size check, which reports it
  // before reading (and a zero-length slice has an arbitrary address).
  if (bytes.size() >= sizeof(uint64_t) &&
      reinterpret_cast<uintptr_t>(bytes.data()) % alignof(uint64_t) != 0) {
    throw std::invalid_argument(
        "Invalid compact theta sketch bytes: wrapping requires the bytes to be 8-byte aligned");
  }
  return std::make_unique<WrappedCompactThetaSketchShim>(
      datasketches::wrapped_compact_theta_sketch::wrap(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
