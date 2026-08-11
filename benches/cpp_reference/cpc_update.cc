// Native C++ counterpart to apache-datasketches/examples/bench_cpc_update.rs.
//
// Establishes the floor: what the vendored datasketches-cpp costs for this
// workload with no Rust, no FFI, no shim. The Rust bench divided by this is the
// binding's overhead.
//
// Keep the parameters identical to the Rust side. Both print the estimate, and
// the estimates must match -- a mismatch means the two are no longer measuring
// the same thing. Build and run via run.sh.

#include "cpc_sketch.hpp"

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <memory>

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

datasketches::cpc_sketch build() {
  return datasketches::cpc_sketch(LG_K);
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
  datasketches::cpc_init<std::allocator<uint8_t>>();
  printf("lg_k=%u items=%llu\n", LG_K, static_cast<unsigned long long>(items));
  bench_distinct(items);
  bench_hot(items);
  return 0;
}
