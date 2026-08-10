//! Guards against drift between the two copies of the DataSketches C++ headers.
//!
//! `vendor/datasketches-cpp` at the workspace root is a git submodule pinned to
//! an upstream release; it records WHICH version this project is built against.
//! `apache-datasketches-sys/vendor/datasketches-cpp` is a manual copy of it, and
//! it is the one `build.rs` actually compiles (crates.io packaging only includes
//! files under the package directory, so a submodule at the workspace root would
//! never reach a published crate).
//!
//! Because the copy is refreshed by hand, the two can diverge silently: bump the
//! submodule to a new upstream release, forget the refresh, and everything still
//! compiles, every test still passes, and `cargo publish` still succeeds -- while
//! the repo claims one upstream version and users get another. Nothing else in
//! the build would notice. This test does.
//!
//! It catches drift in both directions: a submodule bump without a refresh, and a
//! local edit to the vendored copy that never went upstream (which would be
//! invisible in review and silently discarded by the next refresh).
//!
//! What it does NOT prove: that either tree is genuinely the upstream release it
//! claims to be. If a refresh dropped a file from both sides this stays green.
//! The submodule's pinned release commit is what gives you that guarantee.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The header trees `build.rs` puts on the include path.
const MODULES: [&str; 5] = ["common", "hll", "cpc", "theta", "tuple"];

/// Present upstream, deliberately absent from the vendored copy: a CMake input
/// template, not a header the Rust build has any use for.
const IGNORED: [&str; 1] = ["version.hpp.in"];

/// Every file under `dir`, keyed by path relative to `dir`, with its contents.
fn collect(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", current.display()));
        for entry in entries {
            let path = entry.expect("cannot read directory entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if IGNORED.contains(&name) {
                continue;
            }
            let relative = path
                .strip_prefix(dir)
                .expect("walked path is under dir")
                .to_path_buf();
            let bytes = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            out.insert(relative, bytes);
        }
    }
    out
}

#[test]
fn vendored_headers_match_the_pinned_submodule() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let vendored_root = manifest.join("vendor/datasketches-cpp");
    let submodule_root = manifest
        .parent()
        .expect("the sys crate has a parent directory")
        .join("vendor/datasketches-cpp");

    // The submodule is absent in the published crate and in any clone made
    // without `--recursive`. Neither is a failure -- there is simply nothing to
    // compare against. An uninitialised submodule leaves the directory empty,
    // so check for real content rather than mere existence.
    if !submodule_root.join("common/include").is_dir() {
        eprintln!(
            "skipping: no submodule checkout at {} \
             (run `git submodule update --init --recursive` to enable this check)",
            submodule_root.display()
        );
        return;
    }

    let mut problems = Vec::new();

    for module in MODULES {
        let vendored = vendored_root.join(module).join("include");
        let upstream = submodule_root.join(module).join("include");

        if !vendored.is_dir() {
            problems.push(format!("{module}: missing from the vendored copy entirely"));
            continue;
        }
        if !upstream.is_dir() {
            problems.push(format!("{module}: missing from the submodule entirely"));
            continue;
        }

        let vendored_files = collect(&vendored);
        let upstream_files = collect(&upstream);

        for path in upstream_files.keys() {
            if !vendored_files.contains_key(path) {
                problems.push(format!(
                    "{module}: {} is missing from the vendored copy (upstream added it?)",
                    path.display()
                ));
            }
        }
        for path in vendored_files.keys() {
            if !upstream_files.contains_key(path) {
                problems.push(format!(
                    "{module}: {} exists only in the vendored copy (upstream removed it?)",
                    path.display()
                ));
            }
        }
        for (path, upstream_bytes) in &upstream_files {
            if let Some(vendored_bytes) = vendored_files.get(path) {
                if vendored_bytes != upstream_bytes {
                    problems.push(format!("{module}: {} differs in content", path.display()));
                }
            }
        }
    }

    assert!(
        problems.is_empty(),
        "the vendored C++ headers have drifted from the pinned submodule \
         ({} difference(s)).\n\n{}\n\n\
         build.rs compiles the VENDORED copy ({}), so this is what users get; \
         the submodule ({}) only records the upstream release. Refresh the \
         vendored copy from the submodule, or revert the local edit.",
        problems.len(),
        problems.join("\n"),
        vendored_root.display(),
        submodule_root.display(),
    );
}
