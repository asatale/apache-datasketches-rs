#pragma once
#include <optional>
#include <type_traits>
#include <utility>
#include "rust/cxx.h"
#include "tuple_sketch.hpp"
#include "tuple_union.hpp"
#include "tuple_intersection.hpp"
#include "tuple_a_not_b.hpp"

namespace apache_datasketches_rs {

// Forward declarations of the cxx-generated `extern "Rust"` surface.
//
// We deliberately do NOT `#include "tuple_generic.rs.h"` here. That header
// carries an `include!` back to the shim headers, and including it from a
// shim header produces a genuine cycle: the generated header re-enters this
// one while its include guard is already set, and then fails with
// "no type named 'TupleGenericSketchShim' in namespace". Same rationale as
// the ResizeFactor forward declaration in theta_sketch_shim.h. The
// definitions arrive in dyn_summary.cc, which includes the generated header
// after this one is complete.
//
// These signatures must match cxx's output exactly, INCLUDING `noexcept` —
// a mismatch is "exception specification in declaration does not match
// previous declaration".
struct RustSummary;
::rust::Box<RustSummary> rust_summary_clone(RustSummary const& summary) noexcept;
void rust_summary_union_combine(RustSummary& target, RustSummary const& other) noexcept;
void rust_summary_intersection_combine(RustSummary& target, RustSummary const& other) noexcept;

// A C++ value type wrapping an owned, type-erased Rust summary.
//
// Move and destroy are rust::Box's own. Only the copy constructor needs a
// trampoline, and upstream copy-constructs a summary only when a whole sketch
// is copied or an update sketch is converted to a compact one -- never on the
// update or rehash path, which move.
//
// The optional is required, not defensive. Upstream's update path calls
// policy_.create() with no arguments before policy_.update(), and there is no
// universal identity element for an arbitrary user-defined summary. create()
// therefore returns a disengaged DynSummary and the update policy clones into
// it. The disengaged state is transient: it exists only between those two
// calls (tuple_sketch_impl.hpp:218-220) and is never stored in the table.
class DynSummary {
public:
  DynSummary() = default;

  explicit DynSummary(rust::Box<RustSummary> inner) : inner_(std::move(inner)) {}

  DynSummary(DynSummary&&) noexcept = default;
  DynSummary& operator=(DynSummary&&) noexcept = default;
  ~DynSummary() = default;

  DynSummary(const DynSummary& other) {
    if (other.inner_) inner_.emplace(rust_summary_clone(**other.inner_));
  }

  DynSummary& operator=(const DynSummary& other) {
    if (this != &other) {
      if (other.inner_) inner_.emplace(rust_summary_clone(**other.inner_));
      else inner_.reset();
    }
    return *this;
  }

  bool engaged() const { return inner_.has_value(); }

  RustSummary& get() { return **inner_; }
  const RustSummary& get() const { return **inner_; }

  void assign_clone_of(const RustSummary& other) {
    inner_.emplace(rust_summary_clone(other));
  }

private:
  std::optional<rust::Box<RustSummary>> inner_;
};

// Stateless by design: upstream's jaccard_similarity_base default-constructs
// scratch union and intersection policies internally, so a policy that carried
// configuration would silently misbehave there. These carry nothing -- they
// dispatch through the summary object itself.

struct DynUpdatePolicy {
  DynSummary create() const { return DynSummary(); }

  void update(DynSummary& summary, const DynSummary& value) const {
    if (!summary.engaged()) {
      summary.assign_clone_of(value.get());
    } else {
      rust_summary_union_combine(summary.get(), value.get());
    }
  }
};

struct DynUnionPolicy {
  void operator()(DynSummary& summary, const DynSummary& other) const {
    rust_summary_union_combine(summary.get(), other.get());
  }
  void operator()(DynSummary& summary, DynSummary&& other) const {
    rust_summary_union_combine(summary.get(), other.get());
  }
};

struct DynIntersectionPolicy {
  void operator()(DynSummary& summary, const DynSummary& other) const {
    rust_summary_intersection_combine(summary.get(), other.get());
  }
  void operator()(DynSummary& summary, DynSummary&& other) const {
    rust_summary_intersection_combine(summary.get(), other.get());
  }
};

using dyn_update_sketch =
    datasketches::update_tuple_sketch<DynSummary, DynSummary, DynUpdatePolicy>;
using dyn_compact_sketch = datasketches::compact_tuple_sketch<DynSummary>;
using dyn_union = datasketches::tuple_union<DynSummary, DynUnionPolicy>;
using dyn_intersection =
    datasketches::tuple_intersection<DynSummary, DynIntersectionPolicy>;
using dyn_a_not_b = datasketches::tuple_a_not_b<DynSummary>;

} // namespace apache_datasketches_rs
