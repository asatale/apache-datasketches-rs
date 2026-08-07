/// Panics if `path` does not exist on disk.
///
/// The bridge and shim file lists in `main` below are expected to be
/// exhaustive for each completed sketch family: every file named there
/// should actually exist, and every file that exists should be named
/// there. A missing file at this point means either a typo in one of
/// those lists, or a file that was renamed/moved/deleted without
/// updating the list to match -- both are bugs we want to catch as a
/// clear build failure here, not as a confusing link error or missing
/// symbol surfacing later.
fn require_exists(path: &str) {
    if !std::path::Path::new(path).exists() {
        panic!(
            "apache-datasketches-sys build.rs: expected file `{path}` does not exist.\n\n\
             This file is listed in build.rs as part of a completed sketch family's bridge/shim \
             file set, which is expected to be exhaustive. A missing file here means either a typo \
             in the file list in build.rs, or the file was renamed, moved, or deleted without \
             updating that list. Fix the list or restore the file."
        );
    }
}

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
            require_exists(path);
            bridges.push(path);
        }
    }
    if cfg!(feature = "cpc") {
        for path in ["src/cpc_sketch.rs", "src/cpc_union.rs"] {
            require_exists(path);
            bridges.push(path);
        }
    }
    if cfg!(feature = "tuple") {
        // Note: src/array_of_doubles_input.rs is deliberately absent — it
        // is a plain Rust module, not a cxx bridge.
        for path in [
            "src/array_of_doubles_sketch.rs",
            "src/array_of_doubles_compact.rs",
            "src/array_of_doubles_union.rs",
            "src/array_of_doubles_intersection.rs",
            "src/array_of_doubles_a_not_b.rs",
            "src/array_of_doubles_jaccard.rs",
            "src/tuple_generic.rs",
        ] {
            require_exists(path);
            bridges.push(path);
        }
    }

    check_bridge_name_uniqueness(&bridges);

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
        .include(vendor_dir.join("tuple/include"))
        .include("cpp")
        .include("cpp/hll")
        .include("cpp/theta")
        .include("cpp/cpc")
        .include("cpp/tuple")
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
            require_exists(path);
            build.file(path);
        }
    }
    if cfg!(feature = "cpc") {
        for path in ["cpp/cpc/cpc_sketch_shim.cc", "cpp/cpc/cpc_union_shim.cc"] {
            require_exists(path);
            build.file(path);
        }
    }
    if cfg!(feature = "tuple") {
        for path in [
            "cpp/tuple/array_of_doubles_sketch_shim.cc",
            "cpp/tuple/array_of_doubles_compact_shim.cc",
            "cpp/tuple/array_of_doubles_union_shim.cc",
            "cpp/tuple/array_of_doubles_intersection_shim.cc",
            "cpp/tuple/array_of_doubles_a_not_b_shim.cc",
            "cpp/tuple/array_of_doubles_jaccard_shim.cc",
            "cpp/tuple/dyn_summary.cc",
        ] {
            require_exists(path);
            build.file(path);
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
    println!("cargo:rerun-if-changed=src/array_of_doubles_sketch.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_compact.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_input.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_union.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_intersection.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_a_not_b.rs");
    println!("cargo:rerun-if-changed=src/array_of_doubles_jaccard.rs");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_sketch_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_sketch_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_compact_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_compact_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_union_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_union_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_intersection_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_intersection_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_a_not_b_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_a_not_b_shim.cc");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_jaccard_shim.h");
    println!("cargo:rerun-if-changed=cpp/tuple/array_of_doubles_jaccard_shim.cc");
    println!("cargo:rerun-if-changed=src/tuple_generic.rs");
    println!("cargo:rerun-if-changed=cpp/tuple/dyn_summary.h");
    println!("cargo:rerun-if-changed=cpp/tuple/dyn_summary.cc");
}

/// Guards against a collision class that has already caused a real crash on
/// this codebase: cxx derives each generated `extern "C"` trampoline symbol
/// (for free functions) and each generated C++ type definition (for shared
/// `struct`/`enum`/opaque types) from the `#[cxx::bridge(namespace = ..)]`
/// namespace plus the item's *name* alone — not from its parameter types and
/// not from which bridge module declared it. Two bridges that happen to
/// declare a free function or shared type with the same name therefore emit
/// the identical C++ symbol; the linker silently picks one definition for
/// both call sites, and callers of the "losing" declaration get the wrong
/// shim type reinterpreted at runtime instead of a link error. This is
/// exactly what happened when the theta and tuple jaccard shims both
/// declared `jaccard_sketch_sketch` (and three siblings) in the
/// `apache_datasketches_rs` namespace, producing a SIGBUS under
/// `--all-features` (fixed by renaming the tuple side to `tuple_jaccard_*`).
///
/// Methods are *not* affected — the receiver type is part of a method's
/// trampoline symbol — so this check only tracks free functions (a `fn`
/// whose first parameter is not `self: ...`) and type *definitions*: a bare
/// `type Name;` (opaque C++ type) or a `struct Name {`/`enum Name {` inside
/// the bridge module. A cross-bridge alias of the form
/// `type Name = crate::other_module::ffi::Name;` re-uses a type already
/// defined by another bridge and emits no second C++ definition, so it must
/// not be flagged as a duplicate — only the original bare declaration counts.
///
/// This is a deliberately simple line-oriented scan over the bridge source
/// files that are actually being compiled in this build (the `bridges` list
/// above, which already reflects the active feature set), not a full Rust
/// parser. It is documented in `AGENTS.md`'s "cxx::bridge names must be
/// globally unique" section.
fn check_bridge_name_uniqueness(bridges: &[&str]) {
    use std::collections::HashMap;

    // name -> the first bridge file that defined it.
    let mut type_defs: HashMap<String, String> = HashMap::new();
    let mut fn_defs: HashMap<String, String> = HashMap::new();

    for &path in bridges {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        // Tracks whether the scan is currently inside the `#[cxx::bridge]`
        // module body (the mod opened right after such an attribute).
        //
        // This matters for `extern "Rust"` bridges: the Rust type backing an
        // opaque `type Name;` declaration is a plain `struct Name { .. }`
        // defined elsewhere in the same file, outside the bridge module (see
        // `RustSummary` in src/tuple_generic.rs). That plain struct is not a
        // second C++-visible type definition -- it's the one and only
        // implementation behind the single opaque declaration -- so struct
        // and bare-`type` matches occurring outside the bridge module must
        // not be recorded.
        let mut brace_balance: i32 = 0;
        let mut saw_bridge_attr = false;
        let mut bridge_entry_depth: Option<i32> = None;

        while i < lines.len() {
            let trimmed = lines[i].trim();
            let in_bridge_mod = bridge_entry_depth.is_some();

            if trimmed.starts_with("//") {
                i += 1;
                continue;
            }

            if trimmed.starts_with("#[cxx::bridge") {
                saw_bridge_attr = true;
                i += 1;
                continue;
            }

            if in_bridge_mod || (saw_bridge_attr && trimmed.contains("mod ")) {
                if saw_bridge_attr && bridge_entry_depth.is_none() && trimmed.contains("mod ") {
                    bridge_entry_depth = Some(brace_balance);
                    saw_bridge_attr = false;
                }
                brace_balance +=
                    trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
                if bridge_entry_depth == Some(brace_balance) {
                    bridge_entry_depth = None;
                }
            }
            let in_bridge_mod = bridge_entry_depth.is_some();

            if in_bridge_mod {
                if let Some(name) = extract_struct_or_enum_name(trimmed) {
                    record_definition(&mut type_defs, name, path, "shared struct/enum");
                    i += 1;
                    continue;
                }

                if let Some(rest) = trimmed.strip_prefix("type ") {
                    let rest = rest.trim_end_matches(';').trim();
                    if !rest.contains('=') {
                        // Bare `type Name;` — an opaque C++ type definition.
                        // (A cross-bridge alias `type Name = crate::...;`
                        // reuses another bridge's definition and is not a
                        // new one.)
                        if !rest.is_empty() {
                            record_definition(
                                &mut type_defs,
                                rest.to_string(),
                                path,
                                "opaque type",
                            );
                        }
                    }
                    i += 1;
                    continue;
                }
            }

            if trimmed.starts_with("fn ") {
                // Free-function/method declarations can wrap across lines
                // (see e.g. `tuple_jaccard_sketch_sketch` in
                // src/array_of_doubles_jaccard.rs). Accumulate lines until
                // the parentheses balance, then look at how the statement
                // ends: `;` means a bridge declaration (inside an `extern`
                // block), `{` means a plain Rust fn definition with a body.
                //
                // The `{` case matters because of `extern "Rust"` bridges:
                // their trampoline is declared once (ending in `;`, inside
                // the `extern "Rust"` block) AND implemented once as a plain
                // fn of the same name elsewhere in the same file (see
                // src/tuple_generic.rs). Without distinguishing the two,
                // this scan would see two "fn rust_summary_clone" statements
                // in one file and report a false self-collision. A
                // definition's body is skipped by brace-counting so its
                // contents (which may themselves contain `fn `, `(`, `;`)
                // don't confuse the rest of the scan.
                let mut stmt = String::new();
                let mut paren_depth = 0i32;
                let mut seen_paren = false;
                let mut j = i;
                loop {
                    let line = lines[j];
                    stmt.push_str(line);
                    stmt.push(' ');
                    for c in line.chars() {
                        match c {
                            '(' => {
                                paren_depth += 1;
                                seen_paren = true;
                            }
                            ')' => paren_depth -= 1,
                            _ => {}
                        }
                    }
                    if seen_paren && paren_depth == 0 {
                        let end = stmt.trim_end();
                        if end.ends_with(';') || end.ends_with('{') {
                            break;
                        }
                    }
                    j += 1;
                    if j >= lines.len() {
                        break;
                    }
                }

                let trimmed_stmt = stmt.trim_end();
                let is_declaration = trimmed_stmt.ends_with(';');
                let is_definition = trimmed_stmt.ends_with('{');

                if is_definition {
                    // Skip the function body: keep a running brace balance
                    // (seeded from the `{` already consumed above) until it
                    // returns to zero.
                    let mut brace_depth = stmt.matches('{').count() as i32
                        - stmt.matches('}').count() as i32;
                    let mut k = j + 1;
                    while brace_depth > 0 && k < lines.len() {
                        let line = lines[k];
                        brace_depth += line.matches('{').count() as i32;
                        brace_depth -= line.matches('}').count() as i32;
                        k += 1;
                    }
                    i = k;
                    continue;
                }

                i = j + 1;

                if is_declaration {
                    if let (Some(fn_pos), Some(paren_pos)) = (stmt.find("fn "), stmt.find('('))
                    {
                        if paren_pos > fn_pos {
                            let name = stmt[fn_pos + 3..paren_pos].trim().to_string();
                            let params = stmt[paren_pos + 1..].trim_start();
                            let is_method = params.starts_with("self");
                            if !is_method && !name.is_empty() {
                                record_definition(&mut fn_defs, name, path, "free function");
                            }
                        }
                    }
                }
                continue;
            }

            i += 1;
        }
    }
}

fn extract_struct_or_enum_name(trimmed: &str) -> Option<String> {
    let after_kw = trimmed
        .strip_prefix("pub struct ")
        .or_else(|| trimmed.strip_prefix("struct "))
        .or_else(|| trimmed.strip_prefix("pub enum "))
        .or_else(|| trimmed.strip_prefix("enum "))?;
    let name: String = after_kw
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn record_definition(
    map: &mut std::collections::HashMap<String, String>,
    name: String,
    path: &str,
    kind: &str,
) {
    if let Some(existing) = map.get(&name) {
        panic!(
            "apache-datasketches-sys build.rs: duplicate {kind} name `{name}` is defined in both \
             `{existing}` and `{path}`.\n\n\
             cxx derives each generated extern \"C\" trampoline symbol (for free functions) and \
             each generated C++ type definition (for shared struct/enum/opaque types) from the \
             bridge namespace plus the item's name alone -- not from parameter types and not from \
             which bridge module declared it. Two bridges declaring the same name therefore emit \
             the identical C++ symbol, and the linker silently picks one definition for both call \
             sites: callers of the \"losing\" declaration get the wrong shim type reinterpreted at \
             runtime, which shows up as a crash or a wrong result, not a link error. This is \
             exactly the bug class that previously caused a SIGBUS when the theta and tuple \
             jaccard shims both declared `jaccard_sketch_sketch` under --all-features. Rename one \
             of the two, typically by prefixing with its family name (e.g. `tuple_jaccard_*`, \
             `TupleResizeFactor`)."
        );
    }
    map.insert(name, path.to_string());
}
