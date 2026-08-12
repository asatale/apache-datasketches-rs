# Native C++ reference benchmarks

The Rust benches (`apache-datasketches/examples/bench_*.rs`) tell you how fast
this crate is. They cannot tell you how much of the cost is *ours* — the FFI
crossing, shim copies, wrapper validation — versus inherent to the underlying
algorithm. For that you need the same workload run directly against the
vendored C++, and that is what lives here.

Every performance claim in `CHANGELOG.md` phrased as "Nx native C++" is
produced by pairing a Rust bench with its counterpart here. Without these
files, those ratios are unreproducible from a clean checkout.

## Why this is not a Cargo target

These are standalone C++ programs, not Rust. They also deliberately sit outside
both crate directories: `cargo package` includes everything inside a crate's
own directory, so a `.cc` file under `apache-datasketches-sys/` would ship in
the published tarball as dead weight.

## Running

```bash
./benches/cpp_reference/run.sh                      # 10M items, 3 passes
./benches/cpp_reference/run.sh 100000000            # explicit item count
./benches/cpp_reference/run.sh --ladder             # sweep 1M / 10M / 100M
./benches/cpp_reference/run.sh 10000000 --reps 5    # more passes, tighter median
```

Arguments are forwarded verbatim to every program, and the Rust harnesses take
the same ones, so the two sides always measure the same shape.

The script compiles against `apache-datasketches-sys/vendor/datasketches-cpp`
— the copy `build.rs` actually compiles, *not* the root `vendor/` submodule —
so the comparison is against the same headers the Rust side uses. It builds
with `-O2`, matching a release Cargo profile closely enough for a ratio.

Run the Rust side with the same item count to compare:

```bash
cargo run --release -p apache-datasketches --example bench_tuple_update --features tuple -- 100000000
```

## What is here

| this directory | its Rust counterpart |
|---|---|
| `hll_update.cc` | `examples/bench_hll_update.rs` |
| `theta_update.cc` | `examples/bench_theta_update.rs` |
| `cpc_update.cc` | `examples/bench_cpc_update.rs` |
| `array_of_doubles_update.cc` | `examples/bench_tuple_update.rs` |

`examples/bench_tuple_generic_update.rs` deliberately has no counterpart: the
generic Tuple sketch calls back into Rust per summary, which has no C++
equivalent to compare against. Quote its cost against the concrete
ArrayOfDoubles path instead.

`bench_common.h` holds the shared mechanism: argument parsing, the repetition
loop's median selection, the key pool and the output format. Only mechanism —
each `.cc` keeps its own `LG_K`, `HOT_KEY_SPACE` and `build()` at the top,
since those are the parameters that have to match the Rust side and they are
easier to check when visible in the file you are reading.

The Rust harnesses duplicate that logic rather than sharing it: there is no way
to share a module across Cargo examples without declaring every example in
`Cargo.toml`. Keep the two in step — especially the output format, the median
definition and the key format.

`run.sh` builds and runs all of them, passing every family's include directory
to every program — that costs nothing and means adding a benchmark needs no
change to the include list.

## Keeping them in sync

Each `.cc` mirrors its Rust counterpart exactly: same `lg_k`, same target type
or `num_values`, same scenarios (`distinct`, `hot` and `str`), same key spaces. If
you change the parameters on one side, change them on the other or the ratio
becomes meaningless. Both print the sketch estimate, which is the cheap check
that the two harnesses really are doing the same work — the numbers should match
exactly, since both feed identical keys to identical hashing.

## The `str` scenario

`str` measures the string update path, which crosses the boundary as a
borrowed `(pointer, length)` pair rather than by value. Two deliberate choices
make the comparison mean something:

- **The C++ side calls `update(data, length)`, not `update(const
  std::string&)`.** That is the overload the shim itself calls, so the
  difference between the two harnesses is binding overhead rather than a
  choice of overload. Comparing against the `std::string` overload would
  flatter us, since that overload allocates on the C++ side too.
- **Keys are pre-built into a fixed pool, outside the timed region.**
  Formatting a key costs more than the update does, and it costs a different
  amount in Rust than in C++, so timing it would swamp the per-call delta.

The pool is 64Ki distinct keys, cycled. For the families with a theta screen
that puts most updates past it — which is exactly where a per-call allocation
in the shim is pure waste, since upstream discards the key without ever
looking at it.

## Reading the numbers

Prefer the **absolute** difference (Rust minus C++, in ns/op) to the ratio. The
binding costs a roughly constant 1.7–2.4 ns per update call; the ratio varies
from 1.3x to 2.2x purely because the families' own updates differ in cost, which
tells you about the algorithm rather than about the binding.

Expect 10–20% run-to-run variance on a normal laptop. Each harness handles that
itself: the printed `ns/op` is the lower median of `--reps` passes (default 3),
with `min` and `max` beside it. The lower median rather than an average of the
two middle values, so every number printed is one an actual pass produced. If
the spread is wide, raise `--reps` rather than averaging passes by hand.

Run `--ladder` before quoting anything. Per-update cost is not constant as a
sketch fills — CPC's `distinct` is 13.80 ns/op at 1M against 5.54 at 100M —
so a single point can be a warm-up average dressed up as a steady-state cost.
The ladder starts at 1M for that reason: below it HLL is still in its coupon
list and CPC is still changing flavour. `str`, by contrast, is flat across all
three rungs.

Each rep rebuilds its sketch, and the harness aborts if the per-rep estimates
disagree — these workloads are deterministic, so a disagreement means the reps
are not running the same thing.
