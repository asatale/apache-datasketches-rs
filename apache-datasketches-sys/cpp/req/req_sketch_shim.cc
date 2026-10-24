#include "req_sketch_shim.h"

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

// --- ReqSortedViewShim -------------------------------------------------------

template<typename T>
ReqSortedViewShim<T>::ReqSortedViewShim(req_sorted_view_t<T> view) : view_(std::move(view)) {}

template<typename T>
size_t ReqSortedViewShim<T>::size() const { return view_.size(); }

template<typename T>
T ReqSortedViewShim<T>::get_quantile(double rank, bool inclusive) const {
  check_rank(rank);
  return view_.get_quantile(rank, inclusive);
}

template<typename T>
double ReqSortedViewShim<T>::get_rank(T item, bool inclusive) const {
  return view_.get_rank(item, inclusive);
}

template<typename T>
std::unique_ptr<std::vector<double>> ReqSortedViewShim<T>::get_pmf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(view_.get_PMF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
std::unique_ptr<std::vector<double>> ReqSortedViewShim<T>::get_cdf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(view_.get_CDF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

// The three accessors below each walk the view once and hand back a whole
// std::vector, for the same reason serialize does: building a rust::Vec here
// would cost two calls back into Rust per element.
template<typename T>
std::unique_ptr<std::vector<T>> ReqSortedViewShim<T>::items() const {
  auto out = std::make_unique<std::vector<T>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) out->push_back((*it).first);
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> ReqSortedViewShim<T>::weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) out->push_back(it.get_weight());
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> ReqSortedViewShim<T>::cumulative_weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  out->reserve(view_.size());
  for (auto it = view_.begin(); it != view_.end(); ++it) {
    out->push_back(it.get_cumulative_weight(true));
  }
  return out;
}

// --- ReqSketchShim -----------------------------------------------------------

template<typename T>
ReqSketchShim<T>::ReqSketchShim(uint16_t k, bool hra) : sketch_(k, hra) {}

template<typename T>
ReqSketchShim<T>::ReqSketchShim(datasketches::req_sketch<T> sketch) : sketch_(std::move(sketch)) {}

template<typename T>
std::unique_ptr<ReqSketchShim<T>> ReqSketchShim<T>::clone_shim() const {
  return std::make_unique<ReqSketchShim<T>>(sketch_);
}

template<typename T>
void ReqSketchShim<T>::update(T item) { sketch_.update(item); }

template<typename T>
void ReqSketchShim<T>::merge(const ReqSketchShim& other) { sketch_.merge(other.sketch_); }

template<typename T>
bool ReqSketchShim<T>::is_empty() const { return sketch_.is_empty(); }

template<typename T>
uint64_t ReqSketchShim<T>::get_n() const { return sketch_.get_n(); }

template<typename T>
uint16_t ReqSketchShim<T>::get_k() const { return sketch_.get_k(); }

template<typename T>
uint32_t ReqSketchShim<T>::get_num_retained() const { return sketch_.get_num_retained(); }

template<typename T>
bool ReqSketchShim<T>::is_estimation_mode() const { return sketch_.is_estimation_mode(); }

template<typename T>
bool ReqSketchShim<T>::is_hra() const { return sketch_.is_HRA(); }

template<typename T>
T ReqSketchShim<T>::get_min_item() const { return sketch_.get_min_item(); }

template<typename T>
T ReqSketchShim<T>::get_max_item() const { return sketch_.get_max_item(); }

template<typename T>
T ReqSketchShim<T>::get_quantile(double rank, bool inclusive) const {
  check_rank(rank);
  return sketch_.get_quantile(rank, inclusive);
}

template<typename T>
double ReqSketchShim<T>::get_rank(T item, bool inclusive) const {
  return sketch_.get_rank(item, inclusive);
}

template<typename T>
std::unique_ptr<std::vector<double>> ReqSketchShim<T>::get_pmf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(sketch_.get_PMF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
std::unique_ptr<std::vector<double>> ReqSketchShim<T>::get_cdf(
    rust::Slice<const T> split_points, bool inclusive) const {
  return std::make_unique<std::vector<double>>(sketch_.get_CDF(
      split_points.data(), static_cast<uint32_t>(split_points.size()), inclusive));
}

template<typename T>
double ReqSketchShim<T>::get_rank_lower_bound(double rank, uint8_t num_std_dev) const {
  return sketch_.get_rank_lower_bound(rank, num_std_dev);
}

template<typename T>
double ReqSketchShim<T>::get_rank_upper_bound(double rank, uint8_t num_std_dev) const {
  return sketch_.get_rank_upper_bound(rank, num_std_dev);
}

// Returned whole as a heap std::vector; see cpc_sketch_shim.cc for why not a
// rust::Vec built element by element.
template<typename T>
std::unique_ptr<std::vector<uint8_t>> ReqSketchShim<T>::serialize() const {
  return std::make_unique<std::vector<uint8_t>>(sketch_.serialize());
}

template<typename T>
rust::String ReqSketchShim<T>::to_string_summary() const {
  return rust::String(std::string(sketch_.to_string().c_str()));
}

template<typename T>
std::unique_ptr<std::vector<T>> ReqSketchShim<T>::retained_items() const {
  auto out = std::make_unique<std::vector<T>>();
  // Upstream's iterator reads past the end on an empty sketch.
  if (sketch_.is_empty()) return out;
  out->reserve(sketch_.get_num_retained());
  for (auto pair : sketch_) out->push_back(pair.first);
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> ReqSketchShim<T>::retained_weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
  // Upstream's iterator reads past the end on an empty sketch.
  if (sketch_.is_empty()) return out;
  out->reserve(sketch_.get_num_retained());
  for (auto pair : sketch_) out->push_back(pair.second);
  return out;
}

template<typename T>
std::unique_ptr<ReqSortedViewShim<T>> ReqSketchShim<T>::sorted_view() const {
  return std::make_unique<ReqSortedViewShim<T>>(sketch_.get_sorted_view());
}

template class ReqSortedViewShim<double>;
template class ReqSortedViewShim<float>;
template class ReqSketchShim<double>;
template class ReqSketchShim<float>;

// --- free functions ----------------------------------------------------------

namespace {

// Reads a native-endian value with memcpy (the bytes need not be aligned),
// after checking it lies within the buffer. This matches upstream, whose
// copy_from_mem also memcpys native values (little-endian on every platform
// this crate builds for).
template<typename V>
V read_at(const uint8_t* data, size_t size, size_t offset, const char* what) {
  if (offset > size || size - offset < sizeof(V)) {
    throw std::invalid_argument(std::string("REQ sketch bytes too short to hold ") + what);
  }
  V value;
  std::memcpy(&value, data + offset, sizeof(V));
  return value;
}

[[noreturn]] void reject(const std::string& why) {
  throw std::invalid_argument("REQ sketch bytes rejected: " + why);
}

// Same formula as upstream's private req_compactor::nearest_even
// (req_compactor_impl.hpp:325). Only called on values already checked to lie
// in (0, 1024], so the cast is defined.
uint32_t nearest_even(float value) {
  return static_cast<uint32_t>(std::round(value / 2)) << 1;
}

// Upstream req_sketch::deserialize trusts its input: it reads each 20-byte
// compactor header after checking for only 8 bytes, allocates num_items
// items before checking they fit, derives a single-level sketch's min/max by
// dereferencing begin() even when the level is empty, and never relates n to
// the levels. Everything a later update/merge/query relies on is therefore
// checked here, against the invariants upstream's own code maintains.
//
// Byte layout, from req_sketch::serialize and req_compactor::serialize (all
// little-endian): 0 preamble_ints, 1 serial_version, 2 family, 3 flags, 4-5 k,
// 6 num_levels, 7 num_raw_items; if num_levels > 1, n u64 then min and max;
// then either num_raw_items raw items or, per level, state u64,
// section_size_raw f32, lg_weight u8, num_sections u8, 2 padding bytes,
// num_items u32 and num_items items.
template<typename T>
void validate_req_layout(rust::Slice<const uint8_t> bytes) {
  const uint8_t* data = bytes.data();
  const size_t size = bytes.size();
  constexpr size_t HEADER_SIZE = 8;
  if (size < HEADER_SIZE) reject("shorter than the 8-byte header");

  const auto preamble_ints = read_at<uint8_t>(data, size, 0, "preamble_ints");
  const auto serial_version = read_at<uint8_t>(data, size, 1, "the serial version");
  const auto family = read_at<uint8_t>(data, size, 2, "the family");
  const auto flags = read_at<uint8_t>(data, size, 3, "the flags");
  const auto k = read_at<uint16_t>(data, size, 4, "k");
  const auto num_levels = read_at<uint8_t>(data, size, 6, "num_levels");
  const auto num_raw_items = read_at<uint8_t>(data, size, 7, "num_raw_items");

  // Upstream checks these three too (req_sketch_impl.hpp:532-534); checking
  // them first keeps the message right for bytes of another family.
  if (family != 17) reject("family is " + std::to_string(family) + ", not 17 (REQ)");
  if (serial_version != 1) reject("serial version " + std::to_string(serial_version));
  if (preamble_ints != (num_levels > 1 ? 4 : 2)) {
    reject("preamble_ints " + std::to_string(preamble_ints) + " does not match "
        + std::to_string(num_levels) + " levels");
  }

  // req_sketch's private `enum flags { RESERVED1, RESERVED2, IS_EMPTY,
  // IS_HIGH_RANK, RAW_ITEMS, IS_LEVEL_ZERO_SORTED }` (req_sketch.hpp:437).
  constexpr uint8_t IS_EMPTY = 1 << 2;
  constexpr uint8_t IS_HIGH_RANK = 1 << 3;
  constexpr uint8_t RAW_ITEMS = 1 << 4;
  constexpr uint8_t IS_LEVEL_ZERO_SORTED = 1 << 5;
  constexpr uint8_t KNOWN_FLAGS = IS_EMPTY | IS_HIGH_RANK | RAW_ITEMS | IS_LEVEL_ZERO_SORTED;
  if (flags & ~KNOWN_FLAGS) reject("unknown flag bits set: " + std::to_string(flags));

  // Upstream documents k as even and in [4, 1024] but checks neither, on any
  // path. Empty bytes are rebuilt through the constructor, which truncates k
  // to 8 bits (req_sketch_impl.hpp:32, 538), so there k must survive that.
  constexpr uint16_t MIN_K = datasketches::req_constants::MIN_K;
  constexpr uint16_t MAX_K = 1024;
  constexpr uint16_t MAX_CONSTRUCTOR_K = 254;
  if (k % 2 != 0 || k < MIN_K || k > MAX_K) {
    reject("k = " + std::to_string(k) + " is not even and in [4, 1024]");
  }
  if (flags & IS_EMPTY) {
    if (k > MAX_CONSTRUCTOR_K) {
      reject("empty sketch has k = " + std::to_string(k)
          + "; upstream would rebuild it with a truncated k");
    }
    // Genuine empty bytes are exactly the header (they also set RAW_ITEMS,
    // since upstream writes it whenever n <= 4, so no raw-item rule applies).
    if (size != HEADER_SIZE) reject("trailing bytes after an empty sketch's header");
    return;
  }

  // A non-empty sketch with no levels has no compactor 0 for upstream to
  // index, and no min/max (deserialize derives those only for one level).
  if (num_levels == 0) reject("non-empty sketch has no levels");

  size_t offset = HEADER_SIZE;
  uint64_t n = 0;
  if (num_levels > 1) {
    n = read_at<uint64_t>(data, size, offset, "n");
    offset += sizeof(uint64_t);
    read_at<T>(data, size, offset, "the min item");
    read_at<T>(data, size, offset + sizeof(T), "the max item");
    offset += 2 * sizeof(T);
  }

  if (flags & RAW_ITEMS) {
    // Upstream writes raw items only when n <= MIN_K (req_sketch_impl.hpp:416),
    // which also means a single level; with more levels it would take n,
    // min and max from the header but items from the raw block only.
    if (num_levels != 1) reject("raw items with " + std::to_string(num_levels) + " levels");
    if (num_raw_items == 0 || num_raw_items > MIN_K) {
      reject(std::to_string(num_raw_items) + " raw items, not 1 to 4");
    }
    const size_t items_size = size_t(num_raw_items) * sizeof(T);
    if (size - offset < items_size) {
      reject("buffer does not hold " + std::to_string(num_raw_items) + " raw items");
    }
    // For 2 to 4 raw items, upstream's byte serialize sizes the buffer as if
    // it held a compactor (get_serialized_size_bytes, req_sketch_impl.hpp:340
    // -344) but writes only the raw items (line 436), leaving the 20-byte
    // compactor header's worth of zeros at the end. Accept exactly that.
    const size_t trailing = size - offset - items_size;
    constexpr size_t PHANTOM_COMPACTOR_HEADER = 20;
    bool trailing_ok = trailing == 0;
    if (trailing == PHANTOM_COMPACTOR_HEADER) {
      trailing_ok = true;
      for (size_t i = size - trailing; i < size; ++i) trailing_ok = trailing_ok && data[i] == 0;
    }
    if (!trailing_ok) reject("unexpected trailing bytes after the raw items");
    return;
  }

  // Upstream keeps 1ULL << (num_sections - 1) defined only while
  // num_sections <= 64, and num_sections only ever starts at
  // INIT_NUM_SECTIONS = 3 and doubles (req_compactor_impl.hpp:44, 304-307),
  // so the valid counts are 3, 6, 12, 24 and 48; the next, 96, already makes
  // that shift undefined.
  constexpr uint64_t COMPACTOR_HEADER_SIZE = 20;
  // num_sections reaches 96 only when a level with 48 sections has state
  // >= 2^47 (req_compactor_impl.hpp:304); merge ORs states (line 249) and
  // then doubles num_sections in a loop, so a huge state merged into a large-k
  // sketch drives the count past 48 until the uint8_t wraps to 0 and the
  // nominal capacity with it (then compact() throws "compaction range error"
  // out of update, an abort across the FFI). state counts compactions of one
  // level and grows by one per compaction (line 292), so a cap of 2^46 sits
  // far above any real sketch and leaves about 2^46 compactions of margin.
  constexpr uint64_t MAX_STATE = uint64_t(1) << 46;
  uint64_t total_weight = 0;
  uint64_t total_items = 0;
  uint64_t total_nom_capacity = 0;
  for (unsigned level = 0; level < num_levels; ++level) {
    const auto state = read_at<uint64_t>(data, size, offset, "a compactor's state");
    const auto section_size_raw = read_at<float>(data, size, offset + 8, "a compactor's section size");
    const auto lg_weight = read_at<uint8_t>(data, size, offset + 12, "a compactor's lg_weight");
    const auto num_sections = read_at<uint8_t>(data, size, offset + 13, "a compactor's num_sections");
    const auto num_items = read_at<uint32_t>(data, size, offset + 16, "a compactor's num_items");
    offset += COMPACTOR_HEADER_SIZE;
    const std::string where = "level " + std::to_string(level) + ": ";

    // Before upstream allocates num_items items (req_compactor_impl.hpp:478).
    if (uint64_t(num_items) > (size - offset) / sizeof(T)) {
      reject(where + std::to_string(num_items) + " items do not fit in the buffer");
    }
    offset += size_t(num_items) * sizeof(T);

    // grow() gives each new level lg_weight = its index
    // (req_sketch_impl.hpp:592), and merge requires the weights to match.
    if (lg_weight != level) reject(where + "lg_weight is " + std::to_string(lg_weight));
    // Every compaction leaves at least nom_capacity / 2 items behind
    // (req_compactor_impl.hpp:316) and promotes at least one, so no level of
    // a non-empty sketch is ever empty. Upstream's iterator
    // (req_sketch_impl.hpp:754-768) dereferences an empty level and then
    // never reaches end(); a single empty level also makes deserialize
    // dereference begin() of an empty range (line 583).
    if (num_items == 0) reject(where + "no items");
    if (num_sections != 3 && num_sections != 6 && num_sections != 12
        && num_sections != 24 && num_sections != 48) {
      reject(where + std::to_string(num_sections) + " sections");
    }
    // The section size starts at k and only shrinks while nearest_even of
    // the result stays >= MIN_K (req_compactor_impl.hpp:42, 302-305). A
    // section size of at least MIN_K is what keeps compact()'s range at
    // least 2 (line 277) and so keeps update, which cannot report an error
    // across the FFI, from throwing. Written so that NaN fails.
    if (!(section_size_raw > 0.0f && section_size_raw <= static_cast<float>(k))
        || nearest_even(section_size_raw) < MIN_K) {
      reject(where + "section size " + std::to_string(section_size_raw)
          + " is not in [3, k] (k = " + std::to_string(k) + ")");
    }
    if (state >= MAX_STATE) reject(where + "state " + std::to_string(state) + " is not below 2^46");

    // Overflow-checked before shifting (lg_weight == level may exceed 63).
    if (lg_weight >= 64 || uint64_t(num_items) > (UINT64_MAX >> lg_weight)
        || (uint64_t(num_items) << lg_weight) > UINT64_MAX - total_weight) {
      reject("level weights overflow u64");
    }
    total_weight += uint64_t(num_items) << lg_weight;
    total_items += num_items;
    total_nom_capacity += uint64_t(2) * num_sections * nearest_even(section_size_raw);
  }
  if (offset != size) reject("trailing bytes after the last compactor");

  // Upstream compresses on update only when the retained count *equals* the
  // total nominal capacity (req_sketch_impl.hpp:182), and every compress
  // leaves each level below its own capacity (lines 615-627), so a genuine
  // sketch always retains fewer. One at or past it would never compact again
  // and grow without bound.
  if (total_items >= total_nom_capacity) {
    reject("retains " + std::to_string(total_items) + " items, not fewer than its capacity "
        + std::to_string(total_nom_capacity));
  }

  if (num_levels > 1) {
    // Compaction and merge conserve total weight, so it always equals n.
    if (total_weight != n) {
      reject("levels hold total weight " + std::to_string(total_weight) + " but n is "
          + std::to_string(n));
    }
    // serialize writes n raw items from level 0 whenever n <= MIN_K
    // (req_sketch_impl.hpp:416, 436), reading past level 0 if other levels
    // hold some of them. A genuine sketch reaches a second level only by
    // compacting a full level 0 (at least 2 * 3 * MIN_K = 24 items), so it
    // never has more than one level at n <= MIN_K.
    if (n <= MIN_K) reject("n = " + std::to_string(n) + " with " + std::to_string(num_levels) + " levels");
  }
  // The sorted view's get_quantile casts rank * n to uint64_t
  // (quantiles_sorted_view_impl.hpp:78), undefined once the double rounds to
  // 2^64. 2^63 is a round bound well below that; the safe merge keeps the
  // sum below the same limit.
  if (total_weight >= (uint64_t(1) << 63)) {
    reject("n = " + std::to_string(total_weight) + " is not below 2^63");
  }
}

// NaN items break the ordering every query relies on (and std::sort's
// comparator contract); upstream's update() drops them, but deserialize
// accepts them. Checked on the decoded sketch (retained items plus min/max)
// rather than by re-parsing items from bytes. validate_req_layout has made
// every level non-empty, so upstream's iterator is safe here.
template<typename T>
void reject_nan_items(const datasketches::req_sketch<T>& sketch) {
  if (sketch.is_empty()) return;
  bool has_nan = std::isnan(sketch.get_min_item()) || std::isnan(sketch.get_max_item());
  for (auto pair : sketch) has_nan = has_nan || std::isnan(pair.first);
  if (has_nan) reject("a NaN item");
}

template<typename T>
std::unique_ptr<ReqSketchShim<T>> req_deserialize(rust::Slice<const uint8_t> bytes) {
  validate_req_layout<T>(bytes);
  auto sketch = datasketches::req_sketch<T>::deserialize(bytes.data(), bytes.size());
  reject_nan_items(sketch);
  return std::make_unique<ReqSketchShim<T>>(std::move(sketch));
}

}  // namespace

std::unique_ptr<ReqDoublesSketchShim> req_doubles_new(uint16_t k, bool hra) {
  return std::make_unique<ReqDoublesSketchShim>(k, hra);
}

std::unique_ptr<ReqDoublesSketchShim> req_doubles_deserialize(rust::Slice<const uint8_t> bytes) {
  return req_deserialize<double>(bytes);
}

double req_doubles_a_priori_rank_error(uint16_t k, double rank, bool hra, uint64_t n) {
  // From datasketches-cpp 5.3.0, get_RSE returns the one-standard-deviation
  // rank error directly (get_rank_ub(k, 2, rank, 1, n, hra) - rank). 5.2.0
  // returned the lower bound get_rank_lb(...) instead, which this function
  // used to convert; the value it returns is unchanged.
  return datasketches::req_sketch<double>::get_RSE(k, rank, hra, n);
}

std::unique_ptr<ReqFloatsSketchShim> req_floats_new(uint16_t k, bool hra) {
  return std::make_unique<ReqFloatsSketchShim>(k, hra);
}

std::unique_ptr<ReqFloatsSketchShim> req_floats_deserialize(rust::Slice<const uint8_t> bytes) {
  return req_deserialize<float>(bytes);
}

double req_floats_a_priori_rank_error(uint16_t k, double rank, bool hra, uint64_t n) {
  // From datasketches-cpp 5.3.0, get_RSE returns the one-standard-deviation
  // rank error directly (get_rank_ub(k, 2, rank, 1, n, hra) - rank). 5.2.0
  // returned the lower bound get_rank_lb(...) instead, which this function
  // used to convert; the value it returns is unchanged.
  return datasketches::req_sketch<float>::get_RSE(k, rank, hra, n);
}

} // namespace apache_datasketches_rs
