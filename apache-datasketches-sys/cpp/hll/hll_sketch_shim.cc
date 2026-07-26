#include "hll_sketch_shim.h"
#include <vector>

namespace apache_datasketches_rs {

datasketches::target_hll_type to_cpp_target_type(TargetHllType t) {
  switch (t) {
    case TargetHllType::Hll4: return datasketches::HLL_4;
    case TargetHllType::Hll6: return datasketches::HLL_6;
    case TargetHllType::Hll8: return datasketches::HLL_8;
    default: throw std::invalid_argument("unknown TargetHllType");
  }
}

TargetHllType to_rust_target_type(datasketches::target_hll_type t) {
  switch (t) {
    case datasketches::HLL_4: return TargetHllType::Hll4;
    case datasketches::HLL_6: return TargetHllType::Hll6;
    case datasketches::HLL_8: return TargetHllType::Hll8;
    default: throw std::invalid_argument("unknown target_hll_type");
  }
}

HllSketchShim::HllSketchShim(uint8_t lg_config_k, TargetHllType tgt_type)
  : sketch_(lg_config_k, to_cpp_target_type(tgt_type)) {}

HllSketchShim::HllSketchShim(datasketches::hll_sketch sketch)
  : sketch_(std::move(sketch)) {}

void HllSketchShim::update_u64(uint64_t value) { sketch_.update(value); }
void HllSketchShim::update_i64(int64_t value) { sketch_.update(value); }
void HllSketchShim::update_f64(double value) { sketch_.update(value); }
void HllSketchShim::update_str(rust::Str value) {
  sketch_.update(std::string(value));
}
void HllSketchShim::update_bytes(rust::Slice<const uint8_t> value) {
  sketch_.update(value.data(), value.size());
}

double HllSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double HllSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double HllSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
uint8_t HllSketchShim::get_lg_config_k() const { return sketch_.get_lg_config_k(); }
TargetHllType HllSketchShim::get_target_type() const {
  return to_rust_target_type(sketch_.get_target_type());
}
bool HllSketchShim::is_empty() const { return sketch_.is_empty(); }
void HllSketchShim::reset() { sketch_.reset(); }

rust::String HllSketchShim::to_string_summary() const {
  return rust::String(std::string(sketch_.to_string().c_str()));
}

rust::Vec<uint8_t> HllSketchShim::serialize_compact() const {
  auto bytes = sketch_.serialize_compact();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

rust::Vec<uint8_t> HllSketchShim::serialize_updatable() const {
  auto bytes = sketch_.serialize_updatable();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

std::unique_ptr<HllSketchShim> new_hll_sketch(uint8_t lg_config_k, TargetHllType tgt_type) {
  return std::make_unique<HllSketchShim>(lg_config_k, tgt_type);
}

std::unique_ptr<HllSketchShim> hll_sketch_copy_as(const HllSketchShim& sketch, TargetHllType tgt_type) {
  return std::make_unique<HllSketchShim>(
      datasketches::hll_sketch(sketch.inner(), to_cpp_target_type(tgt_type)));
}

std::unique_ptr<HllSketchShim> hll_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<HllSketchShim>(
      datasketches::hll_sketch::deserialize(bytes.data(), bytes.size()));
}

} // namespace apache_datasketches_rs
