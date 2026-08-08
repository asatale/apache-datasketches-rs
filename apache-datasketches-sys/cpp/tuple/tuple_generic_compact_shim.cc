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

const std::vector<const dyn_compact_sketch::Entry*>& CompactTupleGenericSketchShim::entries() const {
  if (!entries_built_) {
    // The cache holds pointers into sketch_'s own entry storage, so building
    // it copies no summaries at all; entry_summary does the single clone the
    // caller's ownership requires.
    //
    // Build into a local vector and swap it in at the end rather than
    // appending directly into entries_. Storing a pointer cannot throw, so
    // the only thing that can throw here is push_back's reallocation
    // (std::bad_alloc). If that happened while appending into entries_
    // directly, entries_built_ would stay false with entries_ holding a
    // partial result -- the next call would re-reserve and re-append on top
    // of that partial state, so entry_count() would exceed
    // get_num_retained() and entries would be duplicated. Building locally
    // means a throw here leaves entries_/entries_built_ untouched, and the
    // swap only happens once the local build has fully succeeded.
    std::vector<const dyn_compact_sketch::Entry*> built;
    built.reserve(sketch_.get_num_retained());
    for (const auto& entry : sketch_) built.push_back(&entry);
    entries_.swap(built);
    entries_built_ = true;
  }
  return entries_;
}

uint32_t CompactTupleGenericSketchShim::entry_count() const {
  return static_cast<uint32_t>(entries().size());
}

uint64_t CompactTupleGenericSketchShim::entry_hash(uint32_t index) const {
  return entries().at(index)->first;
}

rust::Box<RustSummary> CompactTupleGenericSketchShim::entry_summary(uint32_t index) const {
  return rust_summary_clone(entries().at(index)->second.get());
}

std::unique_ptr<CompactTupleGenericSketchShim> tuple_generic_sketch_compact(
    const TupleGenericSketchShim& sketch, bool ordered) {
  return std::make_unique<CompactTupleGenericSketchShim>(sketch.inner().compact(ordered));
}

} // namespace apache_datasketches_rs
