#!/usr/bin/env bash
# Compiles and runs the native C++ reference benchmarks.
#
# Usage: ./benches/cpp_reference/run.sh [item_count]
#
# See README.md in this directory for what these are for.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

# The copy build.rs compiles, NOT the root vendor/ submodule -- so the
# comparison is against the same headers the Rust side is built from.
vendor="$root/apache-datasketches-sys/vendor/datasketches-cpp"

if [[ ! -d "$vendor/tuple/include" ]]; then
  echo "error: vendored headers not found at $vendor" >&2
  echo "This copy is checked into the repo; if it is missing, see" >&2
  echo "apache-datasketches-sys/vendor/README.md." >&2
  exit 1
fi

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

# -O2 to match a release Cargo profile closely enough for a ratio. The include
# set mirrors build.rs's .include(...) calls. Every family's headers are passed
# to every program rather than tailoring the set per benchmark: it costs nothing
# at compile time, and it means adding a benchmark needs no change here.
#
# CPC is the exception that needs its own source file: cpc_sketch.hpp pulls in
# a .cpp-style implementation for the compression tables, so it links from the
# header set alone -- but the include dir has to be present.
CXX="${CXX:-c++}"
includes=(
  -I"$vendor/common/include"
  -I"$vendor/theta/include"
  -I"$vendor/tuple/include"
  -I"$vendor/hll/include"
  -I"$vendor/cpc/include"
)

# Order matters only for readability of the output: cheapest family first.
benchmarks=(hll theta cpc array_of_doubles)

echo "native C++ reference ($($CXX --version | head -1))"
for bench in "${benchmarks[@]}"; do
  src="$here/${bench}_update.cc"
  if [[ ! -f "$src" ]]; then
    echo "error: $src not found" >&2
    exit 1
  fi
  "$CXX" -std=c++17 -O2 -DNDEBUG "${includes[@]}" -o "$out/$bench" "$src"
done

for bench in "${benchmarks[@]}"; do
  echo
  echo "--- $bench ---"
  "$out/$bench" "$@"
done
