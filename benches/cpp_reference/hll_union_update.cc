// Native C++ counterpart to
// apache-datasketches/examples/bench_hll_union_update.rs.
//
// Establishes the floor: what the vendored datasketches-cpp costs for this
// workload with no Rust, no FFI, no shim. The Rust bench divided by this is
// the binding's overhead.
//
// Keep the parameters identical to the Rust side. Both print the estimate,
// and the estimates must match -- a mismatch means the two are no longer
// measuring the same thing. Build and run via run.sh.
//
// Exercises hll_union's direct-update path (update_u64/update_str), not
// update(hll_sketch): that is the path the CHANGELOG's open caveat named --
// update_str's no-alloc guard applies to HllUnionShim::update_str too, and it
// had no dedicated bench on either side.
//
// No deser scenario: a union has no serialize/deserialize round trip of its
// own upstream -- only its result sketch does -- so ser here measures
// get_result(HLL_8).serialize_compact() together, the one round trip a caller
// of hll_union can actually make.

#include "hll.hpp"

#include "bench_common.h"

namespace {

constexpr uint8_t LG_K = 12;
constexpr uint64_t HOT_KEY_SPACE = 1 << 10;

datasketches::hll_union build() {
  return datasketches::hll_union(LG_K);
}

// Each rep rebuilds the union: a reused one would already be full, so every
// rep after the first would measure a different workload. bench::report
// asserts the per-rep estimates agree, which is what catches that if it
// regresses.
void bench_distinct(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    auto u = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t key = 0; key < items; ++key) u.update(key);
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    estimates.push_back(u.get_estimate());
  }
  bench::report("distinct", items, ns_per_op, estimates);
}

void bench_hot(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    auto u = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) u.update(i % HOT_KEY_SPACE);
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    estimates.push_back(u.get_estimate());
  }
  bench::report("hot", items, ns_per_op, estimates);
}

// Calls the same (data, length) overload the shim calls, so the difference
// against Rust is binding overhead rather than a choice of overload.
void bench_str(uint64_t items, uint64_t reps) {
  const auto keys = bench::string_keys();
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    auto u = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) {
      const std::string& key = keys[i % bench::STR_KEY_SPACE];
      u.update(key.data(), key.size());
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    estimates.push_back(u.get_estimate());
  }
  bench::report("str", items, ns_per_op, estimates);
}

// Serialization, measured per call rather than per item: its cost tracks the
// serialized size, which at lg_k = 12 is the same at every ladder rung.
//
// The union is built once and shared by every rep. Serializing does not
// mutate it, so unlike the update scenarios there is no state a second rep
// would find already dirtied -- and rebuilding at the 100M rung would cost
// more than the measurement itself.
void bench_ser(uint64_t items, uint64_t reps) {
  auto u = build();
  for (uint64_t key = 0; key < items; ++key) u.update(key);
  const auto reference = u.get_result(datasketches::target_hll_type::HLL_8).serialize_compact();

  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    uint64_t total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::SER_CALLS; ++i) {
      total += u.get_result(datasketches::target_hll_type::HLL_8).serialize_compact().size();
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::SER_CALLS));
    bench::keep(static_cast<double>(total));
    estimates.push_back(u.get_estimate());
  }
  bench::report_bytes("ser", items, ns_per_op, estimates, reference.size());
}

} // namespace

int main(int argc, char** argv) {
  const bench::Config cfg = bench::parse_args(argc, argv);
  for (const uint64_t items : cfg.counts) {
    printf("lg_max_k=%u result_type=Hll8 items=%llu reps=%llu\n", LG_K,
           static_cast<unsigned long long>(items), static_cast<unsigned long long>(cfg.reps));
    bench_distinct(items, cfg.reps);
    bench_hot(items, cfg.reps);
    bench_str(items, cfg.reps);
    bench_ser(items, cfg.reps);
  }
  return 0;
}
