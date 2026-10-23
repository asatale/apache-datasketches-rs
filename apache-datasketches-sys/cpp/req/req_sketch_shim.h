#pragma once
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "req_sketch.hpp"

namespace apache_datasketches_rs {

// One template serves both item types, mirroring kll_sketch_shim.h. The
// sorted-view shim is a deliberate copy of KLL's rather than a shared one,
// so the `req` and `kll` features stay independent (see the REQ design doc).

template<typename T>
using req_sorted_view_t = datasketches::quantiles_sorted_view<T, std::less<T>, std::allocator<T>>;

template<typename T>
class ReqSortedViewShim {
public:
  explicit ReqSortedViewShim(req_sorted_view_t<T> view);

  size_t size() const;
  T get_quantile(double rank, bool inclusive) const;
  double get_rank(T item, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_pmf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_cdf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<T>> items() const;
  std::unique_ptr<std::vector<uint64_t>> weights() const;
  std::unique_ptr<std::vector<uint64_t>> cumulative_weights() const;

private:
  req_sorted_view_t<T> view_;
};

template<typename T>
class ReqSketchShim {
public:
  ReqSketchShim(uint16_t k, bool hra);
  explicit ReqSketchShim(datasketches::req_sketch<T> sketch);

  std::unique_ptr<ReqSketchShim> clone_shim() const;
  void update(T item);
  void merge(const ReqSketchShim& other);

  bool is_empty() const;
  uint64_t get_n() const;
  uint16_t get_k() const;
  uint32_t get_num_retained() const;
  bool is_estimation_mode() const;
  bool is_hra() const;
  T get_min_item() const;
  T get_max_item() const;

  T get_quantile(double rank, bool inclusive) const;
  double get_rank(T item, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_pmf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_cdf(rust::Slice<const T> split_points, bool inclusive) const;
  double get_rank_lower_bound(double rank, uint8_t num_std_dev) const;
  double get_rank_upper_bound(double rank, uint8_t num_std_dev) const;

  std::unique_ptr<std::vector<uint8_t>> serialize() const;
  rust::String to_string_summary() const;

  std::unique_ptr<std::vector<T>> retained_items() const;
  std::unique_ptr<std::vector<uint64_t>> retained_weights() const;
  std::unique_ptr<ReqSortedViewShim<T>> sorted_view() const;

private:
  datasketches::req_sketch<T> sketch_;
};

using ReqDoublesSketchShim = ReqSketchShim<double>;
using ReqFloatsSketchShim = ReqSketchShim<float>;
using ReqDoublesSortedViewShim = ReqSortedViewShim<double>;
using ReqFloatsSortedViewShim = ReqSortedViewShim<float>;

std::unique_ptr<ReqDoublesSketchShim> req_doubles_new(uint16_t k, bool hra);
std::unique_ptr<ReqDoublesSketchShim> req_doubles_deserialize(rust::Slice<const uint8_t> bytes);
double req_doubles_a_priori_rank_error(uint16_t k, double rank, bool hra, uint64_t n);

std::unique_ptr<ReqFloatsSketchShim> req_floats_new(uint16_t k, bool hra);
std::unique_ptr<ReqFloatsSketchShim> req_floats_deserialize(rust::Slice<const uint8_t> bytes);
double req_floats_a_priori_rank_error(uint16_t k, double rank, bool hra, uint64_t n);

} // namespace apache_datasketches_rs
