// Shared mechanism for the native C++ reference benchmarks: argument parsing,
// repetition, median selection and the output format.
//
// Only mechanism lives here. Each benchmark keeps its own LG_K, HOT_KEY_SPACE
// and build() at the top of its .cc, because those are the parameters that have
// to stay in sync with the Rust counterpart, and they are easier to check
// against it when they are visible in the file you are already reading.
//
// The Rust harnesses in apache-datasketches/examples/ duplicate this logic
// rather than sharing it -- there is no way to share a module across examples
// without declaring every example in Cargo.toml. Keep the two in step, in
// particular the output format, the median definition and the key format.

#ifndef APACHE_DATASKETCHES_RS_BENCH_COMMON_H
#define APACHE_DATASKETCHES_RS_BENCH_COMMON_H

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

namespace bench {

// Size of the pre-built string-key pool. See string_keys.
constexpr uint64_t STR_KEY_SPACE = 1 << 16;

constexpr uint64_t DEFAULT_ITEMS = 10000000ULL;
constexpr uint64_t DEFAULT_REPS = 3;

// Item counts for --ladder, which exists because a single item count hides the
// shape: a family's per-update cost is not constant as the sketch fills.
//
// Starts at 1M rather than lower. Below that the cheap families are still in a
// warm-up regime -- HLL's coupon list, CPC's flavour transitions -- so the
// printed ns/op would be an average taken across a regime change rather than a
// steady-state cost, which is precisely the kind of number the ladder exists to
// stop people quoting.
inline std::vector<uint64_t> ladder() {
  return {1000000ULL, 10000000ULL, 100000000ULL};
}

struct Config {
  std::vector<uint64_t> counts;
  uint64_t reps;
};

// Parses [ITEMS] [--reps N] [--ladder]. Hand-rolled: three flags do not justify
// a dependency in a reference benchmark.
inline Config parse_args(int argc, char** argv) {
  Config cfg{{}, DEFAULT_REPS};
  bool want_ladder = false;
  bool have_items = false;
  uint64_t items = DEFAULT_ITEMS;
  for (int i = 1; i < argc; ++i) {
    if (strcmp(argv[i], "--ladder") == 0) {
      want_ladder = true;
    } else if (strcmp(argv[i], "--reps") == 0) {
      if (++i >= argc || (cfg.reps = strtoull(argv[i], nullptr, 10)) == 0) {
        fprintf(stderr, "error: --reps needs a positive integer\n");
        exit(2);
      }
    } else {
      items = strtoull(argv[i], nullptr, 10);
      if (items == 0) {
        fprintf(stderr, "error: item count must be a positive integer, got '%s'\n", argv[i]);
        exit(2);
      }
      have_items = true;
    }
  }
  // Rejected rather than resolved by precedence: silently ignoring an explicit
  // item count would make a mis-typed invocation look like it measured what was
  // asked for.
  if (want_ladder && have_items) {
    fprintf(stderr, "error: pass an item count or --ladder, not both\n");
    exit(2);
  }
  cfg.counts = want_ladder ? ladder() : std::vector<uint64_t>{items};
  return cfg;
}

// Built once, outside every timed region: formatting a key costs more than the
// update does, and it costs a different amount in each language, so including
// it would swamp the per-call delta these harnesses exist to show. Keep the
// format identical to the Rust counterpart or the estimates diverge.
inline std::vector<std::string> string_keys() {
  std::vector<std::string> keys;
  keys.reserve(STR_KEY_SPACE);
  char buf[32];
  for (uint64_t i = 0; i < STR_KEY_SPACE; ++i) {
    snprintf(buf, sizeof buf, "key_%010llu", static_cast<unsigned long long>(i));
    keys.emplace_back(buf);
  }
  return keys;
}

inline double seconds_since(std::chrono::steady_clock::time_point start) {
  return std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
}

// Prints the lower median of the passes plus the spread, so a published figure
// is never a single noisy point -- the AGENTS.md rule that a performance claim
// rest on a median of at least three runs is enforced here rather than left to
// whoever happens to be running it.
//
// Lower median (sorted[(n - 1) / 2]), not the average of the two middle values:
// every number printed is then one that an actual pass produced. At the default
// reps=3 the two definitions agree; this only matters for an even --reps.
//
// The estimates are asserted equal across reps rather than merely reported.
// These workloads are deterministic, so a disagreement means the reps are not
// running the same thing -- most likely a sketch reused across reps instead of
// rebuilt, which would quietly lower the ns/op of every rep after the first.
//
// ns/op, reps and estimate are printed as labelled values rather than as bare
// numbers in fixed columns, so that reading them back does not mean counting
// awk fields that shift whenever a column is added.
inline void report(const char* label, uint64_t items, std::vector<double> ns_per_op,
                   const std::vector<double>& estimates) {
  for (size_t i = 1; i < estimates.size(); ++i) {
    if (estimates[i] != estimates[0]) {
      fprintf(stderr,
              "error: %s rep %zu estimated %.0f but rep 0 estimated %.0f; the reps are not "
              "running the same workload\n",
              label, i, estimates[i], estimates[0]);
      exit(1);
    }
  }
  std::sort(ns_per_op.begin(), ns_per_op.end());
  const double median = ns_per_op[(ns_per_op.size() - 1) / 2];
  printf("%-9s %12llu items  %7.2f ns/op  min %7.2f  max %7.2f  %8.1f M/s  reps=%zu estimate=%.0f\n",
         label, static_cast<unsigned long long>(items), median, ns_per_op.front(),
         ns_per_op.back(), 1000.0 / median, ns_per_op.size(), estimates[0]);
}

} // namespace bench

#endif // APACHE_DATASKETCHES_RS_BENCH_COMMON_H
