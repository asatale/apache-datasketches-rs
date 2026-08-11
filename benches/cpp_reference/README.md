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
./benches/cpp_reference/run.sh                      # default item count
./benches/cpp_reference/run.sh 100000000            # explicit item count
```

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

`run.sh` builds and runs all of them, passing every family's include directory
to every program — that costs nothing and means adding a benchmark needs no
change to the include list.

## Keeping them in sync

Each `.cc` mirrors its Rust counterpart exactly: same `lg_k`, same target type
or `num_values`, same scenarios (`distinct` and `hot`), same hot-key space. If
you change the parameters on one side, change them on the other or the ratio
becomes meaningless. Both print the sketch estimate, which is the cheap check
that the two harnesses really are doing the same work — the numbers should match
exactly, since both feed identical keys to identical hashing.

## Reading the numbers

Prefer the **absolute** difference (Rust minus C++, in ns/op) to the ratio. The
binding costs a roughly constant 1.7–2.4 ns per update call; the ratio varies
from 1.3x to 2.2x purely because the families' own updates differ in cost, which
tells you about the algorithm rather than about the binding.

Expect 10–20% run-to-run variance on a normal laptop. Take a median of at least
three passes before recording anything.
