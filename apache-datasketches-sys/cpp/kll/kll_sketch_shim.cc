#include "kll_sketch_shim.h"
#include "kolmogorov_smirnov.hpp"

#include <cmath>
#include <cstring>
#include <stdexcept>
#include <string>

namespace apache_datasketches_rs {

namespace {

// Written so that NaN fails; upstream casts rank * total_weight to an integer
// unchecked, which is undefined behaviour for NaN or out-of-range values.
void check_rank(double rank) {
  if (!(rank >= 0.0 && rank <= 1.0)) {
    throw std::invalid_argument("normalized rank must be in [0, 1]");
  }
}

}  // namespace

// --- KllSortedViewShim -------------------------------------------------------

template<typename T>
KllSortedViewShim<T>::KllSortedViewShim(kll_sorted_view_t<T> view) : view_(std::move(view)) {}

template<typename T>
size_t KllSortedViewShim<T>::size() const { return view_.size(); }

template<typename T>
T KllSortedViewShim<T>::get_quantile(double rank, bool inclusive) const {
  check_rank(rank);
  return view_.get_quantile(rank, inclusive);
}

template<typename T>
double KllSortedViewShim<T>::get_rank(T item, bool inclusive) const {
  return view_.get_rank(item, inclusive);
}

template<typename T>
std::unique_ptr<std::vector<double>> KllSortedViewShim<T>::get_pmf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(view_.get_PMF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
std::unique_ptr<std::vector<double>> KllSortedViewShim<T>::get_cdf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(view_.get_CDF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

// The three accessors below each walk the view once and hand back a whole
// std::vector, for the same reason serialize does: building a rust::Vec here
// would cost two calls back into Rust per element.
template<typename T>
std::unique_ptr<std::vector<T>> KllSortedViewShim<T>::items() const {
  auto out = std::make_unique<std::vector<T>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) out->push_back((*it).first);
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> KllSortedViewShim<T>::weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) out->push_back(it.get_weight());
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> KllSortedViewShim<T>::cumulative_weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) {
    out->push_back(it.get_cumulative_weight(true));
  }
  return out;
}

// --- KllSketchShim -----------------------------------------------------------

template<typename T>
KllSketchShim<T>::KllSketchShim(uint16_t k) : sketch_(k) {}

template<typename T>
KllSketchShim<T>::KllSketchShim(datasketches::kll_sketch<T> sketch) : sketch_(std::move(sketch)) {}

template<typename T>
std::unique_ptr<KllSketchShim<T>> KllSketchShim<T>::clone_shim() const {
  return std::make_unique<KllSketchShim<T>>(sketch_);
}

template<typename T>
void KllSketchShim<T>::update(T item) { sketch_.update(item); }

template<typename T>
void KllSketchShim<T>::merge(const KllSketchShim& other) { sketch_.merge(other.sketch_); }

template<typename T>
bool KllSketchShim<T>::is_empty() const { return sketch_.is_empty(); }

template<typename T>
uint64_t KllSketchShim<T>::get_n() const { return sketch_.get_n(); }

template<typename T>
uint16_t KllSketchShim<T>::get_k() const { return sketch_.get_k(); }

template<typename T>
uint32_t KllSketchShim<T>::get_num_retained() const { return sketch_.get_num_retained(); }

template<typename T>
bool KllSketchShim<T>::is_estimation_mode() const { return sketch_.is_estimation_mode(); }

template<typename T>
T KllSketchShim<T>::get_min_item() const { return sketch_.get_min_item(); }

template<typename T>
T KllSketchShim<T>::get_max_item() const { return sketch_.get_max_item(); }

template<typename T>
T KllSketchShim<T>::get_quantile(double rank, bool inclusive) const {
  check_rank(rank);
  return sketch_.get_quantile(rank, inclusive);
}

template<typename T>
double KllSketchShim<T>::get_rank(T item, bool inclusive) const {
  return sketch_.get_rank(item, inclusive);
}

template<typename T>
std::unique_ptr<std::vector<double>> KllSketchShim<T>::get_pmf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(sketch_.get_PMF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
std::unique_ptr<std::vector<double>> KllSketchShim<T>::get_cdf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(sketch_.get_CDF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
double KllSketchShim<T>::get_normalized_rank_error(bool pmf) const {
  return sketch_.get_normalized_rank_error(pmf);
}

// Returned whole as a heap std::vector; see cpc_sketch_shim.cc for why not a
// rust::Vec built element by element.
template<typename T>
std::unique_ptr<std::vector<uint8_t>> KllSketchShim<T>::serialize() const {
  return std::make_unique<std::vector<uint8_t>>(sketch_.serialize());
}

template<typename T>
rust::String KllSketchShim<T>::to_string_summary() const {
  return rust::String(std::string(sketch_.to_string().c_str()));
}

template<typename T>
std::unique_ptr<std::vector<T>> KllSketchShim<T>::retained_items() const {
  auto out = std::make_unique<std::vector<T>>();
  out->reserve(sketch_.get_num_retained());
  for (auto pair : sketch_) out->push_back(pair.first);
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> KllSketchShim<T>::retained_weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(sketch_.get_num_retained());
  for (auto pair : sketch_) out->push_back(pair.second);
  return out;
}

template<typename T>
std::unique_ptr<KllSortedViewShim<T>> KllSketchShim<T>::sorted_view() const {
  return std::make_unique<KllSortedViewShim<T>>(sketch_.get_sorted_view());
}

template<typename T>
double KllSketchShim<T>::ks_delta(const KllSketchShim& other) const {
  return datasketches::kolmogorov_smirnov::delta(sketch_, other.sketch_);
}

template<typename T>
double KllSketchShim<T>::ks_threshold(const KllSketchShim& other, double p) const {
  return datasketches::kolmogorov_smirnov::threshold(sketch_, other.sketch_, p);
}

template<typename T>
bool KllSketchShim<T>::ks_test(const KllSketchShim& other, double p) const {
  return datasketches::kolmogorov_smirnov::test(sketch_, other.sketch_, p);
}

template class KllSortedViewShim<double>;
template class KllSortedViewShim<float>;
template class KllSketchShim<double>;
template class KllSketchShim<float>;

// --- free functions ----------------------------------------------------------

namespace {

// Reads a native-endian value with memcpy (the bytes need not be aligned),
// after checking it lies within the buffer. This matches upstream, whose
// copy_to_mem/copy_from_mem also memcpy native integers (little-endian on
// every platform this crate builds for).
template<typename V>
V read_at(const uint8_t* data, size_t size, size_t offset, const char* what) {
  if (offset > size || size - offset < sizeof(V)) {
    throw std::invalid_argument(std::string("KLL sketch bytes too short to hold ") + what);
  }
  V value;
  std::memcpy(&value, data + offset, sizeof(V));
  return value;
}

// Upstream kll_sketch::deserialize trusts the serialized level layout: it
// never checks that the levels are non-decreasing, fit inside the derived
// capacity, or account for exactly n items. A sketch that passes those
// unchecked reaches assert_correct_total_weight() (std::logic_error) inside
// merge, or worse, so validate the layout here before upstream sees it.
// Empty and single-item sketches carry no level layout; their remaining
// checks (preamble, family, length) are left to upstream.
void validate_kll_layout(const uint8_t* data, size_t size) {
  // Byte layout, from kll_sketch::serialize: 0 preamble_ints, 1 serial_version,
  // 2 family, 3 flags, 4-5 k, 6 m, 7 unused; then for the full preamble
  // 8-15 n, 16-17 min_k, 18 num_levels, 19 unused, 20.. levels (u32 each,
  // the final boundary omitted because it equals the capacity).
  if (size < 8) return;  // upstream rejects short headers itself
  const auto flags = read_at<uint8_t>(data, size, 3, "the flags");
  // Bit positions mirror kll_sketch's private `enum flags { IS_EMPTY,
  // IS_LEVEL_ZERO_SORTED, IS_SINGLE_ITEM }` (kll_sketch.hpp).
  constexpr uint8_t IS_EMPTY_BIT = 0;
  constexpr uint8_t IS_SINGLE_ITEM_BIT = 2;
  const bool is_empty = flags & (1 << IS_EMPTY_BIT);
  const bool is_single_item = flags & (1 << IS_SINGLE_ITEM_BIT);
  if (is_empty) return;

  // Upstream checks k only in the constructor, not on this path.
  const auto k = read_at<uint16_t>(data, size, 4, "k");
  if (k < datasketches::kll_constants::MIN_K) {
    throw std::invalid_argument("KLL sketch k must be at least "
        + std::to_string(datasketches::kll_constants::MIN_K) + ": " + std::to_string(k));
  }
  if (is_single_item) return;

  const auto m = read_at<uint8_t>(data, size, 6, "m");
  const auto n = read_at<uint64_t>(data, size, 8, "n");
  const auto num_levels = read_at<uint8_t>(data, size, 18, "num_levels");

  // Upstream's capacity helper (kll_helper::int_cap_aux) rejects a depth
  // above 60, i.e. more than 61 levels; with at most 61 levels the largest
  // level weight is 1 << 60, which fits in a uint64_t.
  constexpr uint8_t MAX_NUM_LEVELS = 61;
  if (num_levels < 1 || num_levels > MAX_NUM_LEVELS) {
    throw std::invalid_argument("KLL sketch has an invalid number of levels: "
        + std::to_string(num_levels));
  }
  if (n == 0) throw std::invalid_argument("KLL sketch is flagged non-empty but has n = 0");
  // kll_helper::ub_on_num_levels(n) never returns for n >= 2^63 (the
  // doubling in floor_of_log2_of_fraction wraps to 0), and upstream merge
  // calls it on the merged n (kll_sketch_impl.hpp:788). No real stream gets
  // there, so refuse such sketches outright; the safe merge guards the sum.
  if (n >= (uint64_t(1) << 63)) {
    throw std::invalid_argument("KLL sketch n = " + std::to_string(n) + " is not below 2^63");
  }

  // Upstream derives the capacity from k (not min_k), m and num_levels.
  const uint32_t capacity = datasketches::kll_helper::compute_total_capacity(k, m, num_levels);

  constexpr size_t LEVELS_OFFSET = 20;
  uint32_t prev = 0;
  uint64_t total = 0;
  for (uint8_t i = 0; i <= num_levels; ++i) {
    const uint32_t boundary = i < num_levels
        ? read_at<uint32_t>(data, size, LEVELS_OFFSET + 4 * size_t(i), "the levels array")
        : capacity;
    if (boundary > capacity) {
      throw std::invalid_argument("KLL sketch level boundary exceeds its capacity");
    }
    if (i > 0) {
      if (boundary < prev) throw std::invalid_argument("KLL sketch levels are not non-decreasing");
      const uint64_t count = boundary - prev;
      const unsigned shift = i - 1;  // weight of level i-1 is 2^(i-1)
      if (count != 0 && (count > (UINT64_MAX >> shift)
          || (count << shift) > UINT64_MAX - total)) {
        throw std::invalid_argument("KLL sketch level weights overflow");
      }
      total += count << shift;
    }
    prev = boundary;
  }
  if (total != n) {
    throw std::invalid_argument("KLL sketch levels hold total weight " + std::to_string(total)
        + " but n is " + std::to_string(n));
  }

  // A consistent layout can still carry more levels than n items could ever
  // produce (e.g. n = 3 in level 0 with nine empty levels above). Upstream
  // merge sizes its work arrays by ub_on_num_levels(n) and writes one entry
  // per source level, so such a sketch makes merge throw or overflow the
  // heap (populate_work_arrays). Genuine sketches never exceed this bound.
  // (n < 2^63 is checked above, so this call terminates.)
  if (num_levels > datasketches::kll_helper::ub_on_num_levels(n)) {
    throw std::invalid_argument("KLL sketch has " + std::to_string(num_levels)
        + " levels, more than n = " + std::to_string(n) + " allows");
  }
}

// NaN items break the ordering every query relies on; upstream's update()
// drops them, but deserialize accepts them. Checked on the decoded sketch
// (retained items plus min/max) rather than by re-parsing items from bytes,
// so item width and the min/max position need no second encoding of the
// format.
template<typename T>
void reject_nan_items(const datasketches::kll_sketch<T>& sketch) {
  if (sketch.is_empty()) return;
  bool has_nan = std::isnan(sketch.get_min_item()) || std::isnan(sketch.get_max_item());
  for (auto pair : sketch) has_nan = has_nan || std::isnan(pair.first);
  if (has_nan) throw std::invalid_argument("KLL sketch bytes contain a NaN item");
}

template<typename T>
std::unique_ptr<KllSketchShim<T>> kll_deserialize(rust::Slice<const uint8_t> bytes) {
  validate_kll_layout(bytes.data(), bytes.size());
  auto sketch = datasketches::kll_sketch<T>::deserialize(bytes.data(), bytes.size());
  reject_nan_items(sketch);
  return std::make_unique<KllSketchShim<T>>(std::move(sketch));
}

} // namespace

std::unique_ptr<KllDoublesSketchShim> kll_doubles_new(uint16_t k) {
  return std::make_unique<KllDoublesSketchShim>(k);
}

std::unique_ptr<KllDoublesSketchShim> kll_doubles_deserialize(rust::Slice<const uint8_t> bytes) {
  return kll_deserialize<double>(bytes);
}

double kll_doubles_normalized_rank_error(uint16_t k, bool pmf) {
  return datasketches::kll_sketch<double>::get_normalized_rank_error(k, pmf);
}

std::unique_ptr<KllFloatsSketchShim> kll_floats_new(uint16_t k) {
  return std::make_unique<KllFloatsSketchShim>(k);
}

std::unique_ptr<KllFloatsSketchShim> kll_floats_deserialize(rust::Slice<const uint8_t> bytes) {
  return kll_deserialize<float>(bytes);
}

double kll_floats_normalized_rank_error(uint16_t k, bool pmf) {
  return datasketches::kll_sketch<float>::get_normalized_rank_error(k, pmf);
}

} // namespace apache_datasketches_rs
