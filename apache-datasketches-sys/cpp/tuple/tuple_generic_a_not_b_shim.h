#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"

namespace apache_datasketches_rs {

// Like the union and intersection shims, nothing here mentions RustSummary --
// a-not-b only ever passes shim types across the bridge -- so this gets its
// own bridge file (src/tuple_generic_a_not_b.rs) and aliases the two sketch
// shim types the ordinary `extern "C++"` way.
//
// Unlike those two, a-not-b has NO policy: datasketches::tuple_a_not_b is
// templated on <Summary, Allocator> only (tuple_a_not_b.hpp:29-33), and
// theta_set_difference_base::compute never invokes a callback
// (theta_set_difference_base_impl.hpp:39-82) -- retained entries are
// copy-constructed straight out of A. That is why the constructor below is a
// bare `a_not_b_()` rather than intersection's
// `(DEFAULT_SEED, DynIntersectionPolicy())`; it matches
// ArrayOfDoublesAnotBShim.
//
// Upstream's compute() is a template over both operand types, so the four
// concrete overloads below are the whole reason this shim exists. Their
// bodies are identical by necessity, which makes them easy to cross-wire;
// tests/tuple_generic_a_not_b_link_test.rs pins operand order with an
// asymmetric fixture.
//
// The compute_* methods are declared non-Result on the bridge side. compute()
// throws only on a seed-hash mismatch (theta_set_difference_base_impl.hpp:
// 41-42), and no generic-Tuple API exposes a seed -- every sketch and set op
// in this family is built with datasketches::DEFAULT_SEED -- so the mismatch
// is unreachable. Same reasoning as array_of_doubles_a_not_b_shim.h.
class TupleGenericAnotBShim {
public:
  TupleGenericAnotBShim();

  std::unique_ptr<CompactTupleGenericSketchShim> compute_sketch_sketch(const TupleGenericSketchShim& a, const TupleGenericSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactTupleGenericSketchShim> compute_sketch_compact(const TupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactTupleGenericSketchShim> compute_compact_sketch(const CompactTupleGenericSketchShim& a, const TupleGenericSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactTupleGenericSketchShim> compute_compact_compact(const CompactTupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b, bool ordered) const;

private:
  dyn_a_not_b a_not_b_;
};

std::unique_ptr<TupleGenericAnotBShim> new_tuple_generic_a_not_b();

} // namespace apache_datasketches_rs
