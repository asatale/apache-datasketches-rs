// Native C++ counterpart to apache-datasketches/examples/bench_req_update.rs.
//
// Establishes the floor: what the vendored datasketches-cpp costs for this
// workload with no Rust, no FFI, no shim.
//
// Unlike the other pairs, the cross-check is not an estimate: REQ compaction
// flips coins, so quantiles differ run to run. Both sides print n, min, max
// and retained (and bytes for ser/deser) instead, which depend only on the
// input and k (confirmed stable across repeated runs), and compare.sh
// requires those to match. Build and run via run.sh.

#include "req_sketch.hpp"

#include "bench_common.h"

namespace {

constexpr uint16_t K = 12;
constexpr uint64_t HOT_KEY_SPACE = 1 << 10;

// get_quantile cost tracks the retained-item count, not the item count, so
// like ser/deser this is a fixed call count. The first call builds the
// sketch's cached sorted view; it happens before the timed loop.
constexpr uint64_t QUERY_CALLS = 1000000;

using sketch_t = datasketches::req_sketch<double>;

sketch_t build() { return sketch_t(K, true); }

std::string fields(const sketch_t& s) {
  char buf[160];
  snprintf(buf, sizeof buf, " n=%llu min=%.0f max=%.0f retained=%u",
           static_cast<unsigned long long>(s.get_n()), s.get_min_item(), s.get_max_item(),
           s.get_num_retained());
  return buf;
}

std::string fields_bytes(const sketch_t& s, size_t bytes) {
  char buf[32];
  snprintf(buf, sizeof buf, " bytes=%zu", bytes);
  return fields(s) + buf;
}

void bench_distinct(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) sketch.update(static_cast<double>(i));
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    per_rep.push_back(fields(sketch));
  }
  bench::report_fields("distinct", items, ns_per_op, per_rep);
}

void bench_hot(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    auto sketch = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) sketch.update(static_cast<double>(i % HOT_KEY_SPACE));
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    per_rep.push_back(fields(sketch));
  }
  bench::report_fields("hot", items, ns_per_op, per_rep);
}

void bench_quantile(uint64_t items, uint64_t reps) {
  auto sketch = build();
  for (uint64_t i = 0; i < items; ++i) sketch.update(static_cast<double>(i));
  bench::keep(sketch.get_quantile(0.5));  // builds the cached sorted view
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < QUERY_CALLS; ++i) {
      total += sketch.get_quantile(static_cast<double>(i % 1000) / 1000.0);
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(QUERY_CALLS));
    bench::keep(total);
    per_rep.push_back(fields(sketch));
  }
  bench::report_fields("quantile", items, ns_per_op, per_rep);
}

void bench_serde(uint64_t items, uint64_t reps) {
  auto sketch = build();
  for (uint64_t i = 0; i < items; ++i) sketch.update(static_cast<double>(i));
  const auto reference = sketch.serialize();

  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    uint64_t total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::SER_CALLS; ++i) total += sketch.serialize().size();
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::SER_CALLS));
    bench::keep(static_cast<double>(total));
    per_rep.push_back(fields_bytes(sketch, reference.size()));
  }
  bench::report_fields("ser", items, ns_per_op, per_rep);

  ns_per_op.clear();
  per_rep.clear();
  const auto deserialize = [&reference] {
    return sketch_t::deserialize(reference.data(), reference.size());
  };
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::DESER_CALLS; ++i) {
      total += static_cast<double>(deserialize().get_num_retained());
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::DESER_CALLS));
    bench::keep(total);
    per_rep.push_back(fields_bytes(deserialize(), reference.size()));
  }
  bench::report_fields("deser", items, ns_per_op, per_rep);
}

} // namespace

int main(int argc, char** argv) {
  const bench::Config cfg = bench::parse_args(argc, argv);
  for (const uint64_t items : cfg.counts) {
    printf("k=%u items=%llu reps=%llu\n", K, static_cast<unsigned long long>(items),
           static_cast<unsigned long long>(cfg.reps));
    bench_distinct(items, cfg.reps);
    bench_hot(items, cfg.reps);
    bench_quantile(items, cfg.reps);
    bench_serde(items, cfg.reps);
  }
  return 0;
}
