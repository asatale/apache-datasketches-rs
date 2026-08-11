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

## Keeping them in sync

`array_of_doubles_update.cc` mirrors `bench_tuple_update.rs`: same `lg_k`, same
`num_values`, same scenarios (`distinct` and `hot`), same hot-key space. If you
change the parameters on one side, change them on the other or the ratio
becomes meaningless. Both print the sketch estimate, which is the cheap check
that the two harnesses really are doing the same work — the numbers should
match exactly, since both feed identical keys to identical hashing.
