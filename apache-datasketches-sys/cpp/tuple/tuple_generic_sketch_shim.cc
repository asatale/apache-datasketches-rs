#include "tuple_generic_sketch_shim.h"
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

// The sketch's Update type is DynSummary, so every update wraps the borrowed
// RustSummary in a DynSummary. assign_clone_of() below clones unconditionally
// -- one Box allocation on every update() call, whether or not the key
// already exists -- and on the insert path there is a *second* clone,
// because DynUpdatePolicy::create() returns a disengaged DynSummary and
// DynUpdatePolicy::update() then calls assign_clone_of() again to populate
// it (tuple_sketch_impl.hpp:218-219). This is a deliberate correctness-first
// simplification, not the intended end state: the optimization, not
// attempted here, is a DynSummary variant that holds a non-owning
// `const RustSummary*` for the update-value slot instead of an owned Box, so
// this wrapper allocates only when the policy actually clones into a new or
// existing entry.
DynSummary borrow_as_update(const RustSummary& value) {
  DynSummary wrapper;
  wrapper.assign_clone_of(value);
  return wrapper;
}

} // namespace

TupleGenericSketchShim::TupleGenericSketchShim(uint8_t lg_k, uint8_t rf, float p)
  : sketch_(build_sketch(lg_k, rf, p)) {}

void TupleGenericSketchShim::update_u64(uint64_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_i64(int64_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_u32(uint32_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_i32(int32_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_u16(uint16_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_i16(int16_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_u8(uint8_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_i8(int8_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_f64(double key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_str(rust::Str key, const RustSummary& value) {
  sketch_.update(std::string(key), borrow_as_update(value));
}
void TupleGenericSketchShim::update_bytes(rust::Slice<const uint8_t> key, const RustSummary& value) {
  sketch_.update(key.data(), key.size(), borrow_as_update(value));
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

std::unique_ptr<TupleGenericSketchShim> new_tuple_generic_sketch(uint8_t lg_k, uint8_t rf, float p) {
  return std::make_unique<TupleGenericSketchShim>(lg_k, rf, p);
}

} // namespace apache_datasketches_rs
