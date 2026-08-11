#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"
#include "tuple_generic.rs.h"

namespace apache_datasketches_rs {

datasketches::resize_factor tuple_generic_resize_factor(uint8_t rf) {
  switch (rf) {
    case 1: return datasketches::resize_factor::X1;
    case 2: return datasketches::resize_factor::X2;
    case 4: return datasketches::resize_factor::X4;
    case 8: return datasketches::resize_factor::X8;
    default: throw std::invalid_argument("resize factor must be 1, 2, 4 or 8");
  }
}

namespace {

dyn_update_sketch build_sketch(uint8_t lg_k, uint8_t rf, float p) {
  // Brace-init, not parentheses: `builder b(Policy(x))` is a function
  // declaration under the most vexing parse. The ArrayOfDoubles shim hit
  // exactly this.
  dyn_update_sketch::builder builder{DynUpdatePolicy()};
  builder.set_lg_k(lg_k);
  builder.set_resize_factor(tuple_generic_resize_factor(rf));
  builder.set_p(p);
  return builder.build();
}

} // namespace

TupleGenericSketchShim::TupleGenericSketchShim(uint8_t lg_k, uint8_t rf, float p)
  : sketch_(build_sketch(lg_k, rf, p)) {}

// The update_* methods below hand the borrowed `const RustSummary&` straight to
// sketch_.update(), where DynUpdatePolicy's matching overload consumes it.
//
// This used to wrap the value in a DynSummary first, via a
// `borrow_as_update()` helper that called assign_clone_of(). That cloned a Box
// on *every* update, ahead of upstream's `if (hash == 0) return;` screen, so a
// key rejected by theta paid for a clone that was dropped immediately; and the
// insert path cloned twice, once into the wrapper and again to populate the
// entry. The wrapper is gone, so the only clone left is the one that actually
// populates a new entry -- a rejected key now costs zero clones and a repeated
// key costs none beyond the combine.
//
// The correctness argument lives on DynUpdatePolicy::update in dyn_summary.h:
// upstream only ever reads the update value, never stores it, so a borrow
// cannot outlive the call.
void TupleGenericSketchShim::update_u64(uint64_t key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_i64(int64_t key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_u32(uint32_t key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_i32(int32_t key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_u16(uint16_t key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_i16(int16_t key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_u8(uint8_t key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_i8(int8_t key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_f64(double key, const RustSummary& value) {
  sketch_.update(key, value);
}
void TupleGenericSketchShim::update_str(rust::Str key, const RustSummary& value) {
  sketch_.update(std::string(key), value);
}
void TupleGenericSketchShim::update_bytes(rust::Slice<const uint8_t> key, const RustSummary& value) {
  sketch_.update(key.data(), key.size(), value);
}

void TupleGenericSketchShim::trim() { sketch_.trim(); }
void TupleGenericSketchShim::reset() { sketch_.reset(); }

double TupleGenericSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double TupleGenericSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double TupleGenericSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool TupleGenericSketchShim::is_empty() const { return sketch_.is_empty(); }
bool TupleGenericSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool TupleGenericSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double TupleGenericSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t TupleGenericSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericSketchShim::compact(bool ordered) const {
  return tuple_generic_sketch_compact(*this, ordered);
}

std::unique_ptr<TupleGenericSketchShim> new_tuple_generic_sketch(uint8_t lg_k, uint8_t rf, float p) {
  return std::make_unique<TupleGenericSketchShim>(lg_k, rf, p);
}

} // namespace apache_datasketches_rs
