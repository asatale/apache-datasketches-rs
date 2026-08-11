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
# set mirrors build.rs's .include(...) calls; tuple headers pull in theta and
# common.
CXX="${CXX:-c++}"
"$CXX" -std=c++17 -O2 -DNDEBUG \
  -I"$vendor/common/include" \
  -I"$vendor/theta/include" \
  -I"$vendor/tuple/include" \
  -o "$out/array_of_doubles_update" \
  "$here/array_of_doubles_update.cc"

echo "native C++ reference ($($CXX --version | head -1))"
"$out/array_of_doubles_update" "$@"
