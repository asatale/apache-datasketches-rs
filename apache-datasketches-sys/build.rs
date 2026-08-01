fn main() {
    let mut bridges: Vec<&str> = Vec::new();

    if cfg!(feature = "hll") {
        bridges.push("src/hll.rs");
    }
    if cfg!(feature = "theta") {
        for path in [
            "src/theta_sketch.rs",
            "src/theta_compact.rs",
            "src/theta_wrapped.rs",
            "src/theta_union.rs",
            "src/theta_intersection.rs",
            "src/theta_a_not_b.rs",
            "src/theta_jaccard.rs",
        ] {
            if std::path::Path::new(path).exists() {
                bridges.push(path);
            }
        }
    }
    if cfg!(feature = "cpc") {
        // Same incremental-availability rationale as theta above: these
        // bridge modules are added incrementally by the CPC sketch
        // family plan's tasks; only reference the ones that exist so far
        // so that `--features cpc` keeps building at every intermediate
        // task.
        for path in ["src/cpc_sketch.rs", "src/cpc_union.rs"] {
            if std::path::Path::new(path).exists() {
                bridges.push(path);
            }
        }
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

    // cxx-build generates each bridge's header at
    // OUT_DIR/cxxbridge/include/<pkg-name>/src/<name>.rs.h, but our shim
    // headers include it as a bare "<name>.rs.h", so we need that directory
    // directly on the include path in addition to cxx_build::bridges'
    // default dirs.
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let generated_header_dir = out_dir
        .join("cxxbridge/include")
        .join(env!("CARGO_PKG_NAME"))
        .join("src");

    let mut build = cxx_build::bridges(&bridges);
    build
        .include(vendor_dir.join("common/include"))
        .include(vendor_dir.join("hll/include"))
        .include(vendor_dir.join("theta/include"))
        .include(vendor_dir.join("cpc/include"))
        .include("cpp")
        .include("cpp/hll")
        .include("cpp/theta")
        .include("cpp/cpc")
        .include(generated_header_dir)
        .flag_if_supported("-std=c++17")
        // Upstream datasketches-cpp declares virtual destructors on a couple
        // of `final` classes (e.g. hll_sketch_alloc, AuxHashMap) — harmless,
        // but noisy under clang. Silenced here rather than patched in the
        // vendored headers so we don't diverge from upstream.
        .flag_if_supported("-Wno-unnecessary-virtual-specifier");

    if cfg!(feature = "hll") {
        build
            .file("cpp/hll/hll_sketch_shim.cc")
            .file("cpp/hll/hll_union_shim.cc");
    }
    if cfg!(feature = "theta") {
        for path in [
            "cpp/theta/theta_sketch_shim.cc",
            "cpp/theta/theta_compact_shim.cc",
            "cpp/theta/theta_wrapped_shim.cc",
            "cpp/theta/theta_union_shim.cc",
            "cpp/theta/theta_intersection_shim.cc",
            "cpp/theta/theta_a_not_b_shim.cc",
            "cpp/theta/theta_jaccard_shim.cc",
        ] {
            if std::path::Path::new(path).exists() {
                build.file(path);
            }
        }
    }
    if cfg!(feature = "cpc") {
        for path in ["cpp/cpc/cpc_sketch_shim.cc", "cpp/cpc/cpc_union_shim.cc"] {
            if std::path::Path::new(path).exists() {
                build.file(path);
            }
        }
    }

    build.compile("apache_datasketches_sys");

    println!("cargo:rerun-if-changed=src/hll.rs");
    println!("cargo:rerun-if-changed=cpp/hll/hll_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/hll/hll_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/hll/hll_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/hll/hll_union_shim.cc");
    println!("cargo:rerun-if-changed=src/theta_sketch.rs");
    println!("cargo:rerun-if-changed=src/theta_compact.rs");
    println!("cargo:rerun-if-changed=src/theta_wrapped.rs");
    println!("cargo:rerun-if-changed=src/theta_union.rs");
    println!("cargo:rerun-if-changed=src/theta_intersection.rs");
    println!("cargo:rerun-if-changed=src/theta_a_not_b.rs");
    println!("cargo:rerun-if-changed=src/theta_jaccard.rs");
    println!("cargo:rerun-if-changed=cpp/theta/theta_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_compact_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_compact_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_wrapped_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_wrapped_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_union_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_intersection_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_intersection_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_a_not_b_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_a_not_b_shim.cc");
    println!("cargo:rerun-if-changed=cpp/theta/theta_jaccard_shim.h");
    println!("cargo:rerun-if-changed=cpp/theta/theta_jaccard_shim.cc");
    println!("cargo:rerun-if-changed=src/cpc_sketch.rs");
    println!("cargo:rerun-if-changed=src/cpc_union.rs");
    println!("cargo:rerun-if-changed=cpp/cpc/cpc_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/cpc/cpc_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/cpc/cpc_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/cpc/cpc_union_shim.cc");
}
