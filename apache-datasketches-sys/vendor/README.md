# Vendored datasketches-cpp headers

This directory is a manual copy of the headers this crate builds against,
taken from the `vendor/datasketches-cpp` git submodule at the repo root.
It exists so `cargo package`/`cargo publish` — which only includes files
inside this crate's own directory — can produce a self-contained tarball.

Only the headers actually compiled (`common/include`, `hll/include`,
`theta/include`, `cpc/include`, `tuple/include`, `LICENSE`, `NOTICE`) are
copied; `version.hpp.in` is skipped since none of the compiled headers
include it.

## Updating after bumping the submodule's pinned tag

```bash
rm -rf apache-datasketches-sys/vendor/datasketches-cpp
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/common
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/hll
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/theta
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/cpc
mkdir -p apache-datasketches-sys/vendor/datasketches-cpp/tuple
cp -R vendor/datasketches-cpp/common/include apache-datasketches-sys/vendor/datasketches-cpp/common/include
cp -R vendor/datasketches-cpp/hll/include apache-datasketches-sys/vendor/datasketches-cpp/hll/include
cp -R vendor/datasketches-cpp/theta/include apache-datasketches-sys/vendor/datasketches-cpp/theta/include
cp -R vendor/datasketches-cpp/cpc/include apache-datasketches-sys/vendor/datasketches-cpp/cpc/include
cp -R vendor/datasketches-cpp/tuple/include apache-datasketches-sys/vendor/datasketches-cpp/tuple/include
rm apache-datasketches-sys/vendor/datasketches-cpp/common/include/version.hpp.in
cp vendor/datasketches-cpp/LICENSE apache-datasketches-sys/vendor/datasketches-cpp/LICENSE
cp vendor/datasketches-cpp/NOTICE apache-datasketches-sys/vendor/datasketches-cpp/NOTICE
```

When a future sketch family needs headers outside `common/`+`hll/`+
`theta/`+`cpc/`+`tuple/`, add its `include/` directory to both this script
and `build.rs`.
