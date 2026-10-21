#include "kll_sketch_shim.h"
#include "kolmogorov_smirnov.hpp"

namespace apache_datasketches_rs {

// --- KllSortedViewShim -------------------------------------------------------

template<typename T>
KllSortedViewShim<T>::KllSortedViewShim(kll_sorted_view_t<T> view) : view_(std::move(view)) {}

template<typename T>
size_t KllSortedViewShim<T>::size() const { return view_.size(); }

template<typename T>
T KllSortedViewShim<T>::get_quantile(double rank, bool inclusive) const {
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

std::unique_ptr<KllDoublesSketchShim> kll_doubles_new(uint16_t k) {
  return std::make_unique<KllDoublesSketchShim>(k);
}

std::unique_ptr<KllDoublesSketchShim> kll_doubles_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<KllDoublesSketchShim>(
      datasketches::kll_sketch<double>::deserialize(bytes.data(), bytes.size()));
}

double kll_doubles_normalized_rank_error(uint16_t k, bool pmf) {
  return datasketches::kll_sketch<double>::get_normalized_rank_error(k, pmf);
}

std::unique_ptr<KllFloatsSketchShim> kll_floats_new(uint16_t k) {
  return std::make_unique<KllFloatsSketchShim>(k);
}

std::unique_ptr<KllFloatsSketchShim> kll_floats_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<KllFloatsSketchShim>(
      datasketches::kll_sketch<float>::deserialize(bytes.data(), bytes.size()));
}

double kll_floats_normalized_rank_error(uint16_t k, bool pmf) {
  return datasketches::kll_sketch<float>::get_normalized_rank_error(k, pmf);
}

} // namespace apache_datasketches_rs
