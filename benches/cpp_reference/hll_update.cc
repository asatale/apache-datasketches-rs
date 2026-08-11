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

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>

namespace {

constexpr uint8_t LG_K = 12;
constexpr uint64_t HOT_KEY_SPACE = 1 << 10;

void report(const char* label, uint64_t items, double secs, double estimate) {
  printf("%-9s %12llu items  %8.3f s  %7.2f ns/op  %8.1f M/s  (estimate %.0f)\n",
         label, static_cast<unsigned long long>(items), secs,
         secs * 1e9 / static_cast<double>(items),
         static_cast<double>(items) / secs / 1e6, estimate);
}

double seconds_since(std::chrono::steady_clock::time_point start) {
  return std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
}

// Hll8 matches the Rust harness: it is the fastest target type to update, so
// it puts the most weight on per-call overhead rather than bucket packing.
datasketches::hll_sketch build() {
  return datasketches::hll_sketch(LG_K, datasketches::target_hll_type::HLL_8);
}

void bench_distinct(uint64_t items) {
  auto sketch = build();
  const auto start = std::chrono::steady_clock::now();
  for (uint64_t key = 0; key < items; ++key) sketch.update(key);
  const double secs = seconds_since(start);
  report("distinct", items, secs, sketch.get_estimate());
}

void bench_hot(uint64_t items) {
  auto sketch = build();
  const auto start = std::chrono::steady_clock::now();
  for (uint64_t i = 0; i < items; ++i) sketch.update(i % HOT_KEY_SPACE);
  const double secs = seconds_since(start);
  report("hot", items, secs, sketch.get_estimate());
}

} // namespace

int main(int argc, char** argv) {
  const uint64_t items = argc > 1 ? strtoull(argv[1], nullptr, 10) : 10000000ULL;
  printf("lg_config_k=%u target=Hll8 items=%llu\n", LG_K, static_cast<unsigned long long>(items));
  bench_distinct(items);
  bench_hot(items);
  return 0;
}
