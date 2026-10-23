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
  out->reserve(sketch_.get_num_retained());
  for (auto pair : sketch_) out->push_back(pair.first);
  return out;
}

template<typename T>
std::unique_ptr<std::vector<uint64_t>> ReqSketchShim<T>::retained_weights() const {
  auto out = std::make_unique<std::vector<uint64_t>>();
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

std::unique_ptr<ReqDoublesSketchShim> req_doubles_new(uint16_t k, bool hra) {
  return std::make_unique<ReqDoublesSketchShim>(k, hra);
}

std::unique_ptr<ReqDoublesSketchShim> req_doubles_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<ReqDoublesSketchShim>(
      datasketches::req_sketch<double>::deserialize(bytes.data(), bytes.size()));
}

double req_doubles_rse(uint16_t k, double rank, bool hra, uint64_t n) {
  return datasketches::req_sketch<double>::get_RSE(k, rank, hra, n);
}

std::unique_ptr<ReqFloatsSketchShim> req_floats_new(uint16_t k, bool hra) {
  return std::make_unique<ReqFloatsSketchShim>(k, hra);
}

std::unique_ptr<ReqFloatsSketchShim> req_floats_deserialize(rust::Slice<const uint8_t> bytes) {
  return std::make_unique<ReqFloatsSketchShim>(
      datasketches::req_sketch<float>::deserialize(bytes.data(), bytes.size()));
}

double req_floats_rse(uint16_t k, double rank, bool hra, uint64_t n) {
  return datasketches::req_sketch<float>::get_RSE(k, rank, hra, n);
}

} // namespace apache_datasketches_rs
