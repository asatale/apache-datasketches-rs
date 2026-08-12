// Native C++ counterpart to apache-datasketches/examples/bench_hll_update.rs.
//
// Establishes the floor: what the vendored datasketches-cpp costs for this
// workload with no Rust, no FFI, no shim. The Rust bench divided by this is the
// binding's overhead.
//
// Keep the parameters identical to the Rust side. Both print the estimate, and
// the estimates must match -- a mismatch means the two are no longer measuring
// the same thing. Build and run via run.sh.

#include "hll.hpp"

#include "bench_common.h"

namespace {

constexpr uint8_t LG_K = 12;
constexpr uint64_t HOT_KEY_SPACE = 1 << 10;

// Hll8 matches the Rust harness: it is the fastest target type to update, so
// it puts the most weight on per-call overhead rather than bucket packing.
datasketches::hll_sketch build() {
  return datasketches::hll_sketch(LG_K, datasketches::target_hll_type::HLL_8);
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

} // namespace

int main(int argc, char** argv) {
  const bench::Config cfg = bench::parse_args(argc, argv);
  for (const uint64_t items : cfg.counts) {
    printf("lg_config_k=%u target=Hll8 items=%llu reps=%llu\n", LG_K,
           static_cast<unsigned long long>(items), static_cast<unsigned long long>(cfg.reps));
    bench_distinct(items, cfg.reps);
    bench_hot(items, cfg.reps);
    bench_str(items, cfg.reps);
  }
  return 0;
}
