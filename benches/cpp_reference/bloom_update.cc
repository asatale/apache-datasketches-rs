// Native C++ counterpart to apache-datasketches/examples/bench_bloom_update.rs.
//
// Establishes the floor: what the vendored datasketches-cpp costs for this
// workload with no Rust, no FFI, no shim.
//
// A Bloom filter has no estimate, but it is fully deterministic: hashing is
// seeded and nothing is random. Both sides print exact fields instead:
// bits_used after each scenario, hits for query, and bytes for ser/deser.
// compare.sh requires those to match. Build and run via run.sh.
//
// The update scenarios call query_and_update, not update: the Rust binding
// routes every update through query_and_update (plain update leaves upstream's
// bits-set count stale, which causes false negatives), so both sides make the
// same upstream call and the overhead column isolates the FFI cost.

#include "bloom_filter.hpp"

#include "bench_common.h"

namespace {

constexpr uint64_t MAX_DISTINCT_ITEMS = 1000000;
constexpr double TARGET_FPP = 0.01;
constexpr uint64_t SEED = 9001;

using filter_t = datasketches::bloom_filter;

filter_t build() {
  return filter_t::builder::create_by_accuracy(MAX_DISTINCT_ITEMS, TARGET_FPP, SEED);
}

std::string fields(filter_t& f) {
  char buf[64];
  snprintf(buf, sizeof buf, " bits_used=%llu", static_cast<unsigned long long>(f.get_bits_used()));
  return buf;
}

std::string fields_extra(filter_t& f, const char* name, unsigned long long value) {
  char buf[48];
  snprintf(buf, sizeof buf, " %s=%llu", name, value);
  return fields(f) + buf;
}

filter_t filled(uint64_t begin, uint64_t end) {
  auto f = build();
  for (uint64_t i = begin; i < end; ++i) (void)f.query_and_update(i);
  return f;
}

void bench_distinct(uint64_t items, uint64_t reps) {
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    auto f = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) (void)f.query_and_update(i);
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    per_rep.push_back(fields(f));
  }
  bench::report_fields("distinct", items, ns_per_op, per_rep);
}

// Queries 0..items (all hits) then items..2*items (misses, bar false
// positives). ns/op is per query.
void bench_query(uint64_t items, uint64_t reps) {
  auto f = filled(0, items);
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    uint64_t hits = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < 2 * items; ++i) hits += f.query(i) ? 1 : 0;
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(2 * items));
    per_rep.push_back(fields_extra(f, "hits", hits));
  }
  bench::report_fields("query", items, ns_per_op, per_rep);
}

// Calls the same (data, length) overload the shim calls, so the difference
// against Rust is binding overhead rather than a choice of overload.
void bench_str(uint64_t items, uint64_t reps) {
  const auto keys = bench::string_keys();
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    auto f = build();
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < items; ++i) {
      const std::string& key = keys[i % bench::STR_KEY_SPACE];
      if (!key.empty()) (void)f.query_and_update(key.data(), key.size());
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(items));
    per_rep.push_back(fields(f));
  }
  bench::report_fields("str", items, ns_per_op, per_rep);
}

// Union cost tracks the bit-array size, not the item count, so this is a fixed
// call count. Each call unions into a fresh copy: reusing one accumulator
// would make every call after the first a no-op merge.
void bench_union(uint64_t items, uint64_t reps) {
  const uint64_t half = items / 2;
  auto a = filled(0, half);
  auto b = filled(half, items);
  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::OP_CALLS; ++i) {
      filter_t u(a);
      u.union_with(b);
      bench::keep(static_cast<double>(u.get_capacity()));
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::OP_CALLS));
    filter_t u(a);
    u.union_with(b);
    per_rep.push_back(fields(u));
  }
  bench::report_fields("union", items, ns_per_op, per_rep);
}

void bench_serde(uint64_t items, uint64_t reps) {
  auto f = filled(0, items);
  const auto reference = f.serialize();

  std::vector<double> ns_per_op;
  std::vector<std::string> per_rep;
  for (uint64_t r = 0; r < reps; ++r) {
    uint64_t total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::SER_CALLS; ++i) {
      // Read a byte as well as the size: with the size alone the compiler
      // elides the whole serialize call.
      const auto bytes = f.serialize();
      total += bytes.size() + bytes[i % bytes.size()];
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::SER_CALLS));
    bench::keep(static_cast<double>(total));
    per_rep.push_back(fields_extra(f, "bytes", reference.size()));
  }
  bench::report_fields("ser", items, ns_per_op, per_rep);

  ns_per_op.clear();
  per_rep.clear();
  const auto deserialize = [&reference] {
    return filter_t::deserialize(reference.data(), reference.size());
  };
  for (uint64_t r = 0; r < reps; ++r) {
    double total = 0;
    const auto start = std::chrono::steady_clock::now();
    for (uint64_t i = 0; i < bench::DESER_CALLS; ++i) {
      total += static_cast<double>(deserialize().get_capacity());
    }
    ns_per_op.push_back(bench::seconds_since(start) * 1e9 / static_cast<double>(bench::DESER_CALLS));
    bench::keep(total);
    auto d = deserialize();
    per_rep.push_back(fields_extra(d, "bytes", reference.size()));
  }
  bench::report_fields("deser", items, ns_per_op, per_rep);
}

} // namespace

int main(int argc, char** argv) {
  const bench::Config cfg = bench::parse_args(argc, argv);
  for (const uint64_t items : cfg.counts) {
    printf("max_distinct_items=%llu target_fpp=%.2f seed=%llu items=%llu reps=%llu\n",
           static_cast<unsigned long long>(MAX_DISTINCT_ITEMS), TARGET_FPP,
           static_cast<unsigned long long>(SEED), static_cast<unsigned long long>(items),
           static_cast<unsigned long long>(cfg.reps));
    bench_distinct(items, cfg.reps);
    bench_query(items, cfg.reps);
    bench_str(items, cfg.reps);
    bench_union(items, cfg.reps);
    bench_serde(items, cfg.reps);
  }
  return 0;
}
