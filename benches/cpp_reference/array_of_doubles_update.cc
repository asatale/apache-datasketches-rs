// Native C++ counterpart to apache-datasketches/examples/bench_tuple_update.rs.
//
// Establishes the floor: what the vendored datasketches-cpp costs for this
// workload with no Rust, no FFI, no shim. The Rust bench divided by this is
// the binding's overhead, which is the number worth tracking.
//
// Keep the parameters below identical to the Rust side. Both harnesses print
// the sketch estimate; the estimates must match exactly, since both feed the
// same keys through the same hashing. A mismatch means the two are no longer
// measuring the same thing and the ratio is junk.
//
// Build and run via run.sh, which supplies the include paths.

#include "array_of_doubles_sketch.hpp"

#include "bench_common.h"

namespace {

constexpr uint8_t LG_K = 12;
constexpr uint8_t NUM_VALUES = 3;
constexpr uint64_t HOT_KEY_SPACE = 1 << 10;
constexpr double VALUES[NUM_VALUES] = {1.0, 2.0, 3.0};

datasketches::update_array_of_doubles_sketch build() {
  datasketches::update_array_of_doubles_sketch::builder builder{
      datasketches::default_array_of_doubles_update_policy(NUM_VALUES)};
  builder.set_lg_k(LG_K);
  return builder.build();
}

// Every key is new. Once theta drops below 1.0 most keys are rejected by
// hash_and_screen, which returns before the values are ever read.
//
// Each rep rebuilds the sketch: a reused one would already be full, so every
// rep after the first would measure a different workload. bench::report asserts
// the per-rep estimates agree, which is what catches that if it regresses.
void bench_distinct(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t key = 0; key < items; ++key) sketch.update(key, VALUES);
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    estimates.push_back(sketch.get_estimate());
  }
  bench::report("distinct", items, ns_per_op, estimates);
}

// Keys drawn from a space small enough to stay fully retained, so every call
// reaches the summary-combine path.
void bench_hot(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op, estimates;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) sketch.update(i % HOT_KEY_SPACE, VALUES);
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
      sketch.update(key.data(), key.size(), VALUES);
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
    printf("lg_k=%u num_values=%u items=%llu reps=%llu\n", LG_K, NUM_VALUES,
           static_cast<unsigned long long>(items), static_cast<unsigned long long>(cfg.reps));
    bench_distinct(items, cfg.reps);
    bench_hot(items, cfg.reps);
    bench_str(items, cfg.reps);
  }
  return 0;
}
