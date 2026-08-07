#include "tuple_generic_compact_shim.h"
#include "tuple_generic.rs.h"

namespace apache_datasketches_rs {

CompactTupleGenericSketchShim::CompactTupleGenericSketchShim(dyn_compact_sketch sketch)
  : sketch_(std::move(sketch)) {}

double CompactTupleGenericSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double CompactTupleGenericSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double CompactTupleGenericSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool CompactTupleGenericSketchShim::is_empty() const { return sketch_.is_empty(); }
bool CompactTupleGenericSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool CompactTupleGenericSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double CompactTupleGenericSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t CompactTupleGenericSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

const std::vector<dyn_compact_sketch::Entry>& CompactTupleGenericSketchShim::entries() const {
  if (!entries_built_) {
    entries_.reserve(sketch_.get_num_retained());
    for (const auto& entry : sketch_) entries_.push_back(entry);
    entries_built_ = true;
  }
  return entries_;
}

uint32_t CompactTupleGenericSketchShim::entry_count() const {
  return static_cast<uint32_t>(entries().size());
}

uint64_t CompactTupleGenericSketchShim::entry_hash(uint32_t index) const {
  return entries().at(index).first;
}

rust::Box<RustSummary> CompactTupleGenericSketchShim::entry_summary(uint32_t index) const {
  return rust_summary_clone(entries().at(index).second.get());
}

std::unique_ptr<CompactTupleGenericSketchShim> tuple_generic_sketch_compact(
    const TupleGenericSketchShim& sketch, bool ordered) {
  return std::make_unique<CompactTupleGenericSketchShim>(sketch.inner().compact(ordered));
}

} // namespace apache_datasketches_rs
