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

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

namespace {

constexpr uint8_t LG_K = 12;
constexpr uint8_t NUM_VALUES = 3;
constexpr uint64_t HOT_KEY_SPACE = 1 << 10;
constexpr uint64_t STR_KEY_SPACE = 1 << 16;

// Built once, outside every timed region -- see the Rust counterpart. Keep the
// format identical there or the estimates diverge.
std::vector<std::string> string_keys() {
  std::vector<std::string> keys;
  keys.reserve(STR_KEY_SPACE);
  char buf[32];
  for (uint64_t i = 0; i < STR_KEY_SPACE; ++i) {
    snprintf(buf, sizeof buf, "key_%010llu", static_cast<unsigned long long>(i));
    keys.emplace_back(buf);
  }
  return keys;
}
constexpr double VALUES[NUM_VALUES] = {1.0, 2.0, 3.0};

datasketches::update_array_of_doubles_sketch build() {
  datasketches::update_array_of_doubles_sketch::builder builder{
      datasketches::default_array_of_doubles_update_policy(NUM_VALUES)};
  builder.set_lg_k(LG_K);
  return builder.build();
}

void report(const char* label, uint64_t items, double secs, double estimate) {
  printf("%-9s %12llu items  %8.3f s  %7.2f ns/op  %8.1f M/s  (estimate %.0f)\n",
         label, static_cast<unsigned long long>(items), secs,
         secs * 1e9 / static_cast<double>(items),
         static_cast<double>(items) / secs / 1e6, estimate);
}

double seconds_since(std::chrono::steady_clock::time_point start) {
  return std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
}

// Every key is new. Once theta drops below 1.0 most keys are rejected by
// hash_and_screen, which returns before the values are ever read.
void bench_distinct(uint64_t items) {
  auto sketch = build();
  const auto start = std::chrono::steady_clock::now();
  for (uint64_t key = 0; key < items; ++key) sketch.update(key, VALUES);
  const double secs = seconds_since(start);
  report("distinct", items, secs, sketch.get_estimate());
}

// Keys drawn from a space small enough to stay fully retained, so every call
// reaches the summary-combine path.
void bench_hot(uint64_t items) {
  auto sketch = build();
  const auto start = std::chrono::steady_clock::now();
  for (uint64_t i = 0; i < items; ++i) sketch.update(i % HOT_KEY_SPACE, VALUES);
  const double secs = seconds_since(start);
  report("hot", items, secs, sketch.get_estimate());
}

// Calls the same (data, length) overload the shim calls, so the difference
// against Rust is binding overhead rather than a choice of overload.
void bench_str(uint64_t items) {
  auto sketch = build();
  const auto keys = string_keys();
  const auto start = std::chrono::steady_clock::now();
  for (uint64_t i = 0; i < items; ++i) {
    const std::string& key = keys[i % STR_KEY_SPACE];
    sketch.update(key.data(), key.size(), VALUES);
  }
  const double secs = seconds_since(start);
  report("str", items, secs, sketch.get_estimate());
}

} // namespace

int main(int argc, char** argv) {
  const uint64_t items = argc > 1 ? strtoull(argv[1], nullptr, 10) : 10000000ULL;
  printf("lg_k=%u num_values=%u items=%llu\n", LG_K, NUM_VALUES,
         static_cast<unsigned long long>(items));
  bench_distinct(items);
  bench_hot(items);
  bench_str(items);
  return 0;
}
