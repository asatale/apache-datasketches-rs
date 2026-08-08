#pragma once
#include <cstdint>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_generic_sketch_shim.h"

namespace apache_datasketches_rs {

class CompactTupleGenericSketchShim {
public:
  explicit CompactTupleGenericSketchShim(dyn_compact_sketch sketch);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  // Per-entry access. entry_summary clones exactly once, because the caller
  // owns the result and the sketch keeps its own copy.
  uint32_t entry_count() const;
  uint64_t entry_hash(uint32_t index) const;
  rust::Box<RustSummary> entry_summary(uint32_t index) const;

  const dyn_compact_sketch& inner() const { return sketch_; }

  // Non-copyable: entries_ holds pointers into sketch_'s own storage, so a
  // copied shim's cache would alias the source's entries. Deleting the copy
  // operations makes that invariant structural rather than a comment.
  CompactTupleGenericSketchShim(const CompactTupleGenericSketchShim&) = delete;
  CompactTupleGenericSketchShim& operator=(const CompactTupleGenericSketchShim&) = delete;

private:
  // Materialised once so entry_hash/entry_summary are O(1) rather than
  // walking the sketch's iterator on every call.
  const std::vector<const dyn_compact_sketch::Entry*>& entries() const;

  dyn_compact_sketch sketch_;

  // `mutable` because entries() lazily populates this from const methods.
  // That is only sound because nothing on the Rust side ever grants
  // concurrent `&`-access to the same CompactTupleSketch<S>: the wrapper in
  // apache-datasketches/src/tuple/generic/compact.rs is `Send` but
  // deliberately NOT `Sync`. Do not add a `Sync` impl there, and do not
  // reach these members through any path that could run concurrently with
  // another call on the same instance.
  //
  // Non-owning pointers into sketch_'s own entry vector (compact_tuple_sketch
  // stores its entries in a std::vector<Entry> member and its const_iterator
  // yields `const Entry&` into it). sketch_ outlives this cache -- both are
  // members of the same object -- and is never mutated after construction,
  // so the pointers stay valid. Caching by value instead would clone every
  // summary through the rust_summary_clone trampoline, doubling the sketch's
  // summary memory and making entry_summary cost two clones per call.
  mutable std::vector<const dyn_compact_sketch::Entry*> entries_;
  mutable bool entries_built_ = false;
};

std::unique_ptr<CompactTupleGenericSketchShim> tuple_generic_sketch_compact(
    const TupleGenericSketchShim& sketch, bool ordered);

} // namespace apache_datasketches_rs
