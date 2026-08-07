#pragma once
#include <optional>
#include <stdexcept>
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
//
// Class invariant: engaged() is true if and only if get() is valid. Everything
// below exists to keep that biconditional true on every path, because
// engaged() is the validity predicate the update policy branches on.
class DynSummary {
public:
  DynSummary() = default;

  explicit DynSummary(rust::Box<RustSummary> inner) : inner_(std::move(inner)) {}

  // Move must leave the source *disengaged*, so it cannot be defaulted.
  // std::optional's move leaves the source engaged, and rust::Box's own move
  // constructor nulls the moved-from pointer -- so a defaulted move would
  // leave the source reporting engaged() == true while holding a null Box,
  // making get() a null dereference and breaking the invariant above.
  //
  // Both stay noexcept: upstream's entry table move-constructs every entry on
  // rehash and resize, and DynSummary must remain nothrow-move-constructible
  // for that (pinned by a static_assert in dyn_summary.cc). Nothing here can
  // throw -- rust::Box's move constructor, move assignment, and destructor are
  // all noexcept, and its destructor tolerates a null pointer.
  DynSummary(DynSummary&& other) noexcept : inner_(std::move(other.inner_)) {
    other.inner_.reset();
  }

  DynSummary& operator=(DynSummary&& other) noexcept {
    if (this != &other) {
      inner_ = std::move(other.inner_);
      other.inner_.reset();
    }
    return *this;
  }

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

  // Accessing a disengaged DynSummary is a programming error, not a runtime
  // condition -- but it must be diagnosable rather than undefined. A throw is
  // preferred to assert() because assert() vanishes under NDEBUG, exactly
  // where a silent null dereference is hardest to diagnose. Whether a caller
  // actually gets a recoverable Rust Result::Err depends on how it crosses
  // the cxx boundary: only shims declared to return `Result<..>` get that
  // conversion. The update_* shims in tuple_generic_sketch_shim.cc are
  // deliberately not declared that way, so a throw reaching them becomes a
  // deterministic abort, not an Err -- see the note on DynUpdatePolicy below.
  // The branch itself is negligible next to the Rust-side allocation these
  // paths already perform.
  //
  // No noexcept member of this header calls get(): the move operations and
  // engaged() do not, and the copy operations dereference inner_ directly
  // after checking it and are not noexcept. The policies below are not
  // noexcept either.
  RustSummary& get() {
    if (!inner_) throw_disengaged();
    return **inner_;
  }

  const RustSummary& get() const {
    if (!inner_) throw_disengaged();
    return **inner_;
  }

  void assign_clone_of(const RustSummary& other) {
    inner_.emplace(rust_summary_clone(other));
  }

private:
  [[noreturn]] static void throw_disengaged() {
    throw std::logic_error(
        "apache_datasketches_rs::DynSummary::get() called on a disengaged summary. "
        "DynSummary's invariant is that engaged() is true if and only if get() is "
        "valid; a disengaged summary holds no Rust summary, so there is nothing to "
        "return. Check engaged() first, or assign_clone_of() a summary into it.");
  }

  std::optional<rust::Box<RustSummary>> inner_;
};

// Stateless by design: upstream's jaccard_similarity_base default-constructs
// scratch union and intersection policies internally, so a policy that carried
// configuration would silently misbehave there. These carry nothing -- they
// dispatch through the summary object itself.
//
// None of these policies is noexcept, and none checks engaged() beyond what it
// needs semantically: DynSummary::get() enforces the precondition itself and
// throws std::logic_error on a disengaged summary. That throw only becomes a
// Rust Result::Err for callers that cross the cxx boundary through a shim
// declared `-> Result<..>`; DynUpdatePolicy::update() is reached from
// TupleGenericSketchShim::update_*(), which are declared to return plain
// `void` (not Result) for ergonomics, so a throw there is a deterministic
// abort, not a recoverable Err. Either way, a disengaged summary reaching any
// of these -- including the `value` argument of update(), which has no
// semantic reason to be disengaged -- is diagnosable rather than a silent
// null dereference.

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
