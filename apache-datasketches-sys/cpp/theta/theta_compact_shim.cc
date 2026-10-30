#include "theta_compact_shim.h"

#include <cstdint>
#include <cstring>
#include <vector>

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

// Upstream's parser reads the image through typed pointers (u16 seed hash,
// u64 theta and entries, u32 counts), which is undefined behavior unless the
// base is 8-byte aligned. The result is an owned sketch whose entries are
// copied into a std::vector (v1-v3) or unpacked byte-wise into one (v4), so
// it keeps no pointer into the input and an aligned copy is harmless.
std::unique_ptr<CompactThetaSketchShim> compact_theta_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  if (reinterpret_cast<uintptr_t>(bytes.data()) % alignof(uint64_t) == 0) {
    return std::make_unique<CompactThetaSketchShim>(
        datasketches::compact_theta_sketch::deserialize(bytes.data(), bytes.size()));
  }
  std::vector<uint64_t> aligned((bytes.size() + sizeof(uint64_t) - 1) / sizeof(uint64_t));
  std::memcpy(aligned.data(), bytes.data(), bytes.size());
  return std::make_unique<CompactThetaSketchShim>(
      datasketches::compact_theta_sketch::deserialize(aligned.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
