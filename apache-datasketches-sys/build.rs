fn main() {
    let mut bridges: Vec<&str> = Vec::new();

    if cfg!(feature = "hll") {
        bridges.push("src/hll.rs");
    }

    if bridges.is_empty() {
        return;
    }

    // We build against a copy of the needed datasketches-cpp headers
    // vendored into this crate (`vendor/datasketches-cpp`), not the
    // workspace-root git submodule: crates.io packaging only includes files
    // inside the crate directory, so a path escaping it via `../` would be
    // missing from the published tarball. The workspace-root submodule
    // remains the source of truth for updating the pinned version (see
    // vendor/README.md); this copy is refreshed from it manually.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let vendor_dir = manifest_dir.join("vendor/datasketches-cpp");

    // cxx-build generates the bridge header at
    // OUT_DIR/cxxbridge/include/<pkg-name>/src/hll.rs.h, but our shim headers
    // include it as a bare "hll.rs.h" (matching how cxx examples reference
    // their own generated headers), so we need that directory directly on
    // the include path in addition to cxx_build::bridges' default dirs.
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let generated_header_dir = out_dir
        .join("cxxbridge/include")
        .join(env!("CARGO_PKG_NAME"))
        .join("src");

    let mut build = cxx_build::bridges(&bridges);
    build
        .include(vendor_dir.join("common/include"))
        .include(vendor_dir.join("hll/include"))
        .include("cpp")
        .include("cpp/hll")
        .include(generated_header_dir)
        .flag_if_supported("-std=c++17");

    if cfg!(feature = "hll") {
        build
            .file("cpp/hll/hll_sketch_shim.cc")
            .file("cpp/hll/hll_union_shim.cc");
    }

    build.compile("apache_datasketches_sys");

    println!("cargo:rerun-if-changed=src/hll.rs");
    println!("cargo:rerun-if-changed=cpp/hll/hll_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/hll/hll_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/hll/hll_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/hll/hll_union_shim.cc");
}
