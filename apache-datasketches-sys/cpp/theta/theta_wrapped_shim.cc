#include "theta_wrapped_shim.h"

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
  return std::make_unique<WrappedCompactThetaSketchShim>(
      datasketches::wrapped_compact_theta_sketch::wrap(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
