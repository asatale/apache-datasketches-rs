// Native C++ counterpart to apache-datasketches/examples/bench_theta_update.rs.
//
// Establishes the floor: what the vendored datasketches-cpp costs for this
// workload with no Rust, no FFI, no shim. The Rust bench divided by this is the
// binding's overhead.
//
// Keep the parameters identical to the Rust side. Both print the estimate, and
// the estimates must match -- a mismatch means the two are no longer measuring
// the same thing. Build and run via run.sh.

#include <array>
#include <utility>

#include "theta_sketch.hpp"
#include "theta_union.hpp"
#include "theta_intersection.hpp"
#include "theta_jaccard_similarity.hpp"

#include "bench_common.h"

namespace {

constexpr uint8_t LG_K = 12;
constexpr uint64_t HOT_KEY_SPACE = 1 << 10;

datasketches::update_theta_sketch build() {
  auto builder = datasketches::update_theta_sketch::builder();
  builder.set_lg_k(LG_K);
  return builder.build();
}

// Each rep rebuilds the sketch: a reused one would already be full, so every
// rep after the first would measure a different workload. bench::report asserts
// the per-rep estimates agree, which is what catches that if it regresses.
void bench_distinct(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t key = 0; key < items; ++key) sketch.update(key);
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    estimates.push_back(sketch.get_estimate());
  }
  bench::report("distinct", items, ns_per_op, estimates);
}

void bench_hot(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) sketch.update(i % HOT_KEY_SPACE);
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    estimates.push_back(sketch.get_estimate());
  }
  bench::report("hot", items, ns_per_op, estimates);
}

// Calls the same (data, length) overload the shim calls, so the difference
// against Rust is binding overhead rather than a choice of overload.
void bench_str(uint64_t items, uint64_t reps) {
  const auto keys = bench::string_keys();
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) {
      const std::string& key = keys[i % bench::STR_KEY_SPACE];
      sketch.update(key.data(), key.size());
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    estimates.push_back(sketch.get_estimate());
  }
  bench::report("str", items, ns_per_op, estimates);
}

// Serialization, measured per call rather than per item: its cost tracks the
// serialized size, which at lg_k = 12 is the same at every ladder rung.
//
// The sketch is built once and shared by both directions and every rep.
// Serializing does not mutate it, so unlike the update scenarios there is no
// state that a second rep would find already dirtied -- and rebuilding at the
// 100M rung would cost more than the measurement itself.
void bench_serde(uint64_t items, uint64_t reps) {
  auto update_sketch = build();
  for (uint64_t key = 0; key < items; ++key) update_sketch.update(key);
  const auto sketch = update_sketch.compact(true);
  const auto reference = sketch.serialize();

  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    uint64_t total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::SER_CALLS; ++i) total += sketch.serialize().size();
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::SER_CALLS));
    bench::keep(static_cast<double>(total));
    estimates.push_back(sketch.get_estimate());
  }
  bench::report_bytes("ser", items, ns_per_op, estimates, reference.size());

  ns_per_op.clear();
  estimates.clear();
  const auto deserialize = [&reference] {
    return datasketches::compact_theta_sketch::deserialize(reference.data(), reference.size());
  };
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::DESER_CALLS; ++i) total += deserialize().get_estimate();
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::DESER_CALLS));
    bench::keep(total);
    estimates.push_back(deserialize().get_estimate());
  }
  bench::report_bytes("deser", items, ns_per_op, estimates, reference.size());
}

// Two operands with 50% overlap, built once outside every timed region: the
// operand-construction cost belongs to the setup, not to the union/
// intersection/jaccard call being measured.
std::pair<datasketches::compact_theta_sketch, datasketches::compact_theta_sketch>
build_operands(uint64_t items) {
  auto a = build();
  for (uint64_t key = 0; key < items; ++key) a.update(key);
  auto b = build();
  for (uint64_t key = items / 2; key < items + items / 2; ++key) b.update(key);
  return {a.compact(true), b.compact(true)};
}

// A fresh union is built inside the timed loop, so the figure is
// construct + two updates + get_result, not the merge alone -- reusing one
// accumulator across OP_CALLS iterations would have each iteration merge into
// an ever-growing result, measuring a different workload every time.
void bench_union(uint64_t items, uint64_t reps) {
  const auto [a, b] = build_operands(items);
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0, estimate = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::OP_CALLS; ++i) {
      auto un = datasketches::theta_union::builder().set_lg_k(LG_K).build();
      un.update(a);
      un.update(b);
      estimate = un.get_result(true).get_estimate();
      total += estimate;
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::OP_CALLS));
    bench::keep(total);
    estimates.push_back(estimate);
  }
  bench::report("union", items, ns_per_op, estimates);
}

// As bench_union: a fresh intersection per iteration, so the figure is
// construct + two updates + get_result.
void bench_intersect(uint64_t items, uint64_t reps) {
  const auto [a, b] = build_operands(items);
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0, estimate = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::OP_CALLS; ++i) {
      datasketches::theta_intersection isect;
      isect.update(a);
      isect.update(b);
      estimate = isect.get_result(true).get_estimate();
      total += estimate;
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::OP_CALLS));
    bench::keep(total);
    estimates.push_back(estimate);
  }
  bench::report("intersect", items, ns_per_op, estimates);
}

// jaccard() is a pure function of its two operands -- no accumulator to
// rebuild, unlike union and intersection.
void bench_jaccard(uint64_t items, uint64_t reps) {
  const auto [a, b] = build_operands(items);
  std::vector<double> ns_per_op, lower_bounds, estimates, upper_bounds;
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0;
    std::array<double, 3> bounds{};
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::OP_CALLS; ++i) {
      bounds = datasketches::theta_jaccard_similarity::jaccard(a, b);
      total += bounds[1];
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::OP_CALLS));
    bench::keep(total);
    lower_bounds.push_back(bounds[0]);
    estimates.push_back(bounds[1]);
    upper_bounds.push_back(bounds[2]);
  }
  bench::report_jaccard("jaccard", items, ns_per_op, lower_bounds, estimates, upper_bounds);
}

} // namespace

int main(int argc, char** argv) {
  const bench::Config cfg = bench::parse_args(argc, argv);
  for (const uint64_t items : cfg.counts) {
    printf("lg_k=%u items=%llu reps=%llu\n", LG_K, static_cast<unsigned long long>(items),
           static_cast<unsigned long long>(cfg.reps));
    bench_distinct(items, cfg.reps);
    bench_hot(items, cfg.reps);
    bench_str(items, cfg.reps);
    bench_serde(items, cfg.reps);
    bench_union(items, cfg.reps);
    bench_intersect(items, cfg.reps);
    bench_jaccard(items, cfg.reps);
  }
  return 0;
}
