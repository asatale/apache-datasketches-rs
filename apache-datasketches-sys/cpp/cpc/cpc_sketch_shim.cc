#include "cpc_sketch_shim.h"

namespace apache_datasketches_rs {

CpcSketchShim::CpcSketchShim(uint8_t lg_k) : sketch_(lg_k) {}

CpcSketchShim::CpcSketchShim(datasketches::cpc_sketch sketch)
  : sketch_(std::move(sketch)) {}

void CpcSketchShim::update_u64(uint64_t value) { sketch_.update(value); }
void CpcSketchShim::update_i64(int64_t value) { sketch_.update(value); }
void CpcSketchShim::update_u32(uint32_t value) { sketch_.update(value); }
void CpcSketchShim::update_i32(int32_t value) { sketch_.update(value); }
void CpcSketchShim::update_u16(uint16_t value) { sketch_.update(value); }
void CpcSketchShim::update_i16(int16_t value) { sketch_.update(value); }
void CpcSketchShim::update_u8(uint8_t value) { sketch_.update(value); }
void CpcSketchShim::update_i8(int8_t value) { sketch_.update(value); }
void CpcSketchShim::update_f64(double value) { sketch_.update(value); }
void CpcSketchShim::update_f32(float value) { sketch_.update(value); }
void CpcSketchShim::update_str(rust::Str value) {
  sketch_.update(std::string(value));
}
void CpcSketchShim::update_bytes(rust::Slice<const uint8_t> value) {
  sketch_.update(value.data(), value.size());
}

bool CpcSketchShim::is_empty() const { return sketch_.is_empty(); }
double CpcSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double CpcSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double CpcSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
uint8_t CpcSketchShim::get_lg_k() const { return sketch_.get_lg_k(); }

rust::String CpcSketchShim::to_string_summary() const {
  return rust::String(std::string(sketch_.to_string().c_str()));
}

rust::Vec<uint8_t> CpcSketchShim::serialize() const {
  auto bytes = sketch_.serialize();
  rust::Vec<uint8_t> out;
  for (auto b : bytes) out.push_back(b);
  return out;
}

std::unique_ptr<CpcSketchShim> new_cpc_sketch(uint8_t lg_k) {
  return std::make_unique<CpcSketchShim>(lg_k);
}

std::unique_ptr<CpcSketchShim> cpc_sketch_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<CpcSketchShim>(
      datasketches::cpc_sketch::deserialize(bytes.data(), bytes.size()));
}

size_t cpc_sketch_max_serialized_size_bytes(uint8_t lg_k) {
  return datasketches::cpc_sketch::get_max_serialized_size_bytes(lg_k);
}

void cpc_init() {
  datasketches::cpc_init<std::allocator<uint8_t>>();
}

} // namespace apache_datasketches_rs
