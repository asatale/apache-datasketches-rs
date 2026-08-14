#!/usr/bin/env bash
# Runs both sides of the comparison and prints the binding's overhead.
#
# Usage: ./benches/cpp_reference/compare.sh [item_count] [--reps N] [--ladder]
#
# Arguments are forwarded verbatim to every harness on both sides, which is the
# only way the two are guaranteed to be measuring the same shape.
#
# This is the script to use before quoting a number in CHANGELOG.md. Running
# run.sh and a `cargo run --example bench_*` by hand and subtracting works, but
# nothing checks that the two were given the same parameters -- and a mismatch
# there produces a plausible-looking ratio that means nothing. Here the
# per-scenario sketch estimates are compared and a disagreement is a hard
# failure, because both sides feed identical keys through identical hashing and
# so must agree exactly.
#
# See README.md in this directory.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

# Each C++ benchmark paired with the Rust example and feature that mirror it.
# bench_tuple_generic_update is deliberately absent: the generic Tuple sketch
# calls back into Rust per summary and has no C++ counterpart to compare to.
pairs=(
  "hll:bench_hll_update:hll"
  "hll_union:bench_hll_union_update:hll"
  "theta:bench_theta_update:theta"
  "cpc:bench_cpc_update:cpc"
  "array_of_doubles:bench_tuple_update:tuple"
)

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

echo "running the native C++ reference..." >&2
"$here/run.sh" "$@" >"$out/cpp.txt"

echo "running the Rust harnesses..." >&2
: >"$out/rust.txt"
for pair in "${pairs[@]}"; do
  IFS=: read -r bench example feature <<<"$pair"
  echo "--- $bench ---" >>"$out/rust.txt"
  (cd "$root" && cargo run -q --release -p apache-datasketches \
    --example "$example" --features "$feature" -- "$@") >>"$out/rust.txt"
done

python3 "$here/compare.py" "$out/cpp.txt" "$out/rust.txt"
