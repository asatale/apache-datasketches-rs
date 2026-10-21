#pragma once
#include <cstddef>
#include <cstdint>
#include <functional>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "kll_sketch.hpp"

namespace apache_datasketches_rs {

// One template serves both item types. cxx cannot name a template, so the
// concrete aliases at the bottom of this file are what the bridges declare,
// and kll_sketch_shim.cc explicitly instantiates both.

template<typename T>
using kll_sorted_view_t = datasketches::quantiles_sorted_view<T, std::less<T>, std::allocator<T>>;

template<typename T>
class KllSortedViewShim {
public:
  explicit KllSortedViewShim(kll_sorted_view_t<T> view);

  size_t size() const;
  T get_quantile(double rank, bool inclusive) const;
  double get_rank(T item, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_pmf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_cdf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<T>> items() const;
  std::unique_ptr<std::vector<uint64_t>> weights() const;
  std::unique_ptr<std::vector<uint64_t>> cumulative_weights() const;

private:
  kll_sorted_view_t<T> view_;
};

template<typename T>
class KllSketchShim {
public:
  explicit KllSketchShim(uint16_t k);
  explicit KllSketchShim(datasketches::kll_sketch<T> sketch);

  std::unique_ptr<KllSketchShim> clone_shim() const;
  void update(T item);
  void merge(const KllSketchShim& other);

  bool is_empty() const;
  uint64_t get_n() const;
  uint16_t get_k() const;
  uint32_t get_num_retained() const;
  bool is_estimation_mode() const;
  T get_min_item() const;
  T get_max_item() const;

  T get_quantile(double rank, bool inclusive) const;
  double get_rank(T item, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_pmf(rust::Slice<const T> split_points, bool inclusive) const;
  std::unique_ptr<std::vector<double>> get_cdf(rust::Slice<const T> split_points, bool inclusive) const;
  double get_normalized_rank_error(bool pmf) const;

  std::unique_ptr<std::vector<uint8_t>> serialize() const;
  rust::String to_string_summary() const;

  std::unique_ptr<std::vector<T>> retained_items() const;
  std::unique_ptr<std::vector<uint64_t>> retained_weights() const;
  std::unique_ptr<KllSortedViewShim<T>> sorted_view() const;

  double ks_delta(const KllSketchShim& other) const;
  double ks_threshold(const KllSketchShim& other, double p) const;
  bool ks_test(const KllSketchShim& other, double p) const;

private:
  datasketches::kll_sketch<T> sketch_;
};

using KllDoublesSketchShim = KllSketchShim<double>;
using KllFloatsSketchShim = KllSketchShim<float>;
using KllDoublesSortedViewShim = KllSortedViewShim<double>;
using KllFloatsSortedViewShim = KllSortedViewShim<float>;

std::unique_ptr<KllDoublesSketchShim> kll_doubles_new(uint16_t k);
std::unique_ptr<KllDoublesSketchShim> kll_doubles_deserialize(rust::Slice<const uint8_t> bytes);
double kll_doubles_normalized_rank_error(uint16_t k, bool pmf);

std::unique_ptr<KllFloatsSketchShim> kll_floats_new(uint16_t k);
std::unique_ptr<KllFloatsSketchShim> kll_floats_deserialize(rust::Slice<const uint8_t> bytes);
double kll_floats_normalized_rank_error(uint16_t k, bool pmf);

} // namespace apache_datasketches_rs
