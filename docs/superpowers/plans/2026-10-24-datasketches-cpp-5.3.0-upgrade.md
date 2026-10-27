# datasketches-cpp 5.3.0 Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the vendored Apache DataSketches C++ from 5.2.0 to 5.3.0 for every bound family, adapting the two places that depend on 5.2.0 behavior, pinning the security fixes with regression tests, and updating docs and CHANGELOG.

**Architecture:** A single version bump that refreshes the submodule and the vendored header copy together with the two required code/test changes, so every commit leaves the suite green; then separate commits for ported upstream regression tests and for documentation.

**Tech Stack:** Rust 2021, `cxx` 1.x, Apache DataSketches C++ 5.3.0, C++17.

**Spec:** `docs/superpowers/specs/2026-10-24-datasketches-cpp-5.3.0-upgrade-design.md` — read it first. It records the impact analysis (what breaks, what changes, which workarounds stay).

## Global Constraints

- Work on branch `chore/datasketches-cpp-5.3.0` (already created; the spec is committed there).
- Commit messages: conventional prefix (`chore(deps):`, `test:`, `docs:`, `fix:`). **No `Co-Authored-By` trailer and no AI attribution of any kind.** Do not push.
- Every commit must leave `cargo test --workspace --all-features` green.
- Upstream tag `5.3.0` (tag object `7bfa753`). The vendored copy under `apache-datasketches-sys/vendor/datasketches-cpp` must stay byte-identical to the submodule (`vendor_drift_test`).
- REQ `ReqSketch::a_priori_rank_error` public semantics must not change: the existing pinned tests must pass **unmodified**.
- Workarounds that stay (5.3.0 does not fix them): REQ `k` cap at 254, REQ empty-iterator guard, REQ 20-byte padding acceptance, KLL `n < 2^63` bounds, shim NaN/out-of-range rank checks; `validate_kll_layout`/`validate_req_layout` logic unchanged.
- Format Rust files with `rustfmt --edition 2021 <file>`.
- No performance figures quoted anywhere.

---

### Task 1: Bump to 5.3.0 with the required REQ and `lg_k` changes

**Files:**
- Modify: `vendor/datasketches-cpp` (submodule pointer)
- Modify: `apache-datasketches-sys/vendor/datasketches-cpp/**` (refreshed copy, incl. `LICENSE`, `NOTICE`)
- Modify: `apache-datasketches-sys/cpp/req/req_sketch_shim.cc`
- Modify: `apache-datasketches/src/req/sketch.rs` (rustdoc of `a_priori_rank_error`)
- Modify (assertion flips): `apache-datasketches/tests/{theta_sketch_smoke_test,theta_sketch_test,theta_union_test,tuple_union_test,tuple_union_smoke_test,tuple_sketch_smoke_test,tuple_generic_sketch_smoke_test,tuple_generic_union_smoke_test}.rs`, `apache-datasketches-sys/tests/{theta_sketch_link_test,array_of_doubles_sketch_link_test,array_of_doubles_union_link_test,tuple_generic_sketch_link_test,tuple_generic_union_link_test}.rs`
- Modify (docs stating the lg_k range): Theta/Tuple builder rustdoc under `apache-datasketches/src/{theta,tuple}/` and comments in the test files above

**Interfaces:**
- Produces: the repo builds against 5.3.0; Theta/Tuple builders accept `lg_k` in `4..=26`; REQ shim functions `req_{doubles,floats}_a_priori_rank_error` return upstream `get_RSE` unchanged.

- [ ] **Step 1: Move the submodule**

```bash
git -C vendor/datasketches-cpp fetch --tags origin
git -C vendor/datasketches-cpp checkout 5.3.0
git -C vendor/datasketches-cpp describe --tags   # expect: 5.3.0
```

- [ ] **Step 2: See what breaks before changing anything (RED)**

Run: `cargo test -p apache-datasketches-sys --test vendor_drift_test`
Expected: FAIL — the vendored copy differs from the submodule (lists changed files).

- [ ] **Step 3: Refresh the vendored copy**

Run the copy recipe from `apache-datasketches-sys/vendor/README.md` verbatim (it removes and recreates `apache-datasketches-sys/vendor/datasketches-cpp` from the seven module `include/` dirs plus `LICENSE` and `NOTICE`, and removes `common/include/version.hpp.in`). Then:

Run: `cargo test -p apache-datasketches-sys --test vendor_drift_test` — Expected: PASS.
Run: `git status --short apache-datasketches-sys/vendor | head` — expect modified headers, new `common/include/fdlibm_log.hpp`, modified `LICENSE`.

- [ ] **Step 4: Run the full suite to see the expected failures**

Run: `cargo test --workspace --all-features --no-fail-fast 2>&1 | grep -E '^(test |---- )|FAILED|panicked' | head -80`
Expected: failures only in (a) `req_sketch_smoke_test` `a_priori_rank_error*` and (b) the `lg_k = 4` rejection assertions in the 13 files listed under **Files**. If anything else fails, stop and report it (the impact analysis did not predict it).

- [ ] **Step 5: REQ — return upstream `get_RSE` directly**

In `apache-datasketches-sys/cpp/req/req_sketch_shim.cc`, in both `req_doubles_a_priori_rank_error` and `req_floats_a_priori_rank_error`, replace the `rank - …get_RSE(k, rank, hra, n)` computation with `return datasketches::req_sketch<double>::get_RSE(k, rank, hra, n);` (and `<float>`), and replace the upgrade-note comment above each with:

```cpp
// From datasketches-cpp 5.3.0, get_RSE returns the one-standard-deviation
// rank error directly (get_rank_ub(k, 2, rank, 1, n, hra) - rank). 5.2.0
// returned the lower bound get_rank_lb(...) instead, which this function
// used to convert; the value it returns is unchanged.
```

In `apache-datasketches/src/req/sketch.rs`, edit the `a_priori_rank_error` rustdoc: keep what it returns and its bounds (`0` in the exact region, at most `0.084 / k`), and replace the sentence(s) saying upstream's `get_RSE` is misnamed / returns a lower bound with: "This is upstream's `get_RSE`. (In datasketches-cpp 5.2.0, `get_RSE` returned a rank lower bound instead; that was fixed in 5.3.0.)"

Run: `cargo test -p apache-datasketches --features req --test req_sketch_smoke_test`
Expected: PASS with the test file **unchanged** (`git diff --stat apache-datasketches/tests/req_sketch_smoke_test.rs` shows nothing).

- [ ] **Step 6: Theta/Tuple — accept `lg_k = 4`, reject `lg_k = 3`**

In each of the 13 test files, find the assertion that `lg_k(4)` / `lg_k = 4` fails to build (grep `4` near `lg_k` and `is_err`), change it to assert success, and add next to it the same assertion for `3` expecting `Err` (`SketchError::InvalidConfig(_)` in the safe crate; `is_err()` in sys link tests). Locations from the impact analysis (line numbers approximate):

- `apache-datasketches/tests/theta_sketch_smoke_test.rs:15`, `theta_sketch_test.rs:197`, `theta_union_test.rs:145`, `tuple_union_test.rs:143`, `tuple_union_smoke_test.rs:91`, `tuple_sketch_smoke_test.rs:16`, `tuple_generic_sketch_smoke_test.rs:135`, `tuple_generic_union_smoke_test.rs:93`
- `apache-datasketches-sys/tests/theta_sketch_link_test.rs:17-18`, `array_of_doubles_sketch_link_test.rs:22-23`, `array_of_doubles_union_link_test.rs:68`, `tuple_generic_sketch_link_test.rs:55`, `tuple_generic_union_link_test.rs:110`

Update comments in those files that state the range as `[5, 26]` or say `MIN_LG_K == 5` (e.g. `theta_sketch_test.rs:32`, `:77-80`, `:196`) to `[4, 26]` / `MIN_LG_K == 4 (5.3.0)`. Grep `apache-datasketches/src/theta` and `apache-datasketches/src/tuple` for `5..=26`, `[5, 26]`, `5 and 26` and similar, and update builder rustdoc to `4..=26`.

Before committing, confirm `lg_k = 3` really is rejected on 5.3.0 by running the updated tests; if upstream accepts 3, stop and report.

- [ ] **Step 7: Full verification**

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo fmt --all -- --check
./benches/cpp_reference/compare.sh 100000 --reps 2
```

Expected: all pass; compare.sh prints `Estimates agree on every scenario`.

- [ ] **Step 8: Commit**

```bash
git add vendor/datasketches-cpp apache-datasketches-sys apache-datasketches
git commit -m "chore(deps): upgrade vendored datasketches-cpp to 5.3.0

Picks up the deserialization CVE fixes for HLL, CPC and compact Theta.
REQ's a_priori_rank_error now returns upstream get_RSE directly (5.3.0
fixed it); Theta/Tuple accept lg_k = 4, matching Java."
```

---

### Task 2: Port upstream's 5.3.0 security regression tests

**Files:**
- Modify: `apache-datasketches/tests/hll_sketch_test.rs` (or `hll_union_test.rs` — wherever HLL deserialize tests live), `apache-datasketches/tests/cpc_sketch_test.rs`, `apache-datasketches/tests/theta_sketch_test.rs` (or `theta_compact_smoke_test.rs`), `apache-datasketches/tests/theta_a_not_b_test.rs`

**Interfaces:**
- Consumes: the 5.3.0 bump (Task 1). Upstream test sources are now in the submodule at `vendor/datasketches-cpp/<family>/test/`.

Port each upstream `TEST_CASE` below into the matching Rust test file through the **safe** API, preserving its intent and every assertion that is expressible. Build malformed bytes the way upstream does (usually by serializing a genuine sketch and overwriting specific bytes; copy the offsets and values). Each ported test starts with a comment citing `<upstream file>` and the `TEST_CASE` name at tag 5.3.0, and the CVE where applicable.

| Upstream file (tag 5.3.0) | `TEST_CASE` | Rust expectation |
|---|---|---|
| `hll/test/CouponListTest.cpp` | `coupon list: rejects malformed coupon count in bytes` | `HllSketch::deserialize` → `Err(SketchError::Deserialization(_))` (CVE-2026-103501). Skip the `in stream` variant (no stream API) with a comment. |
| `cpc/test/cpc_sketch_test.cpp` | `cpc sketch: deserialize corrupt num_coupons` | `CpcSketch::deserialize` → `Err(Deserialization)` (CVE-2026-103513) |
| `cpc/test/cpc_sketch_test.cpp` | `cpc sketch: deserialize corrupt lg_k` | `Err(Deserialization)` (CVE-2026-103513) |
| `theta/test/theta_sketch_test.cpp` | `theta sketch: deserialize truncated preamble` | both `CompactThetaSketch::deserialize` and `WrappedCompactThetaSketch::wrap` → `Err` (CVE-2026-103635) |
| `theta/test/theta_sketch_test.cpp` | `theta sketch: deserialize v4 corrupt header` | `CompactThetaSketch::deserialize_compressed` (and `wrap`, if upstream tests the wrapped path) → `Err` |
| `theta/test/theta_sketch_test.cpp` | `theta sketch: deserialize v4 entry bits overflow` | `Err` |
| `theta/test/theta_a_not_b_test.cpp` | `theta a-not-b: empty B with different seed must be ignored` | Build B as an empty sketch serialized with a different seed **by hand-editing the seed-hash bytes of an empty compact image** (this crate exposes no seed API); A minus B returns A's estimate and does not abort. Add a comment that on 5.2.0 this aborted the process through a non-`Result` bridge. |

Use the exact `SketchError` variant the family's wrapper maps deserialize failures to (check the wrapper; most use `Deserialization`). Where an upstream case also asserts the exception *message*, assert only that an error occurred (messages are upstream implementation details).

- [ ] **Step 1: Write the ported tests**
- [ ] **Step 2: Prove they guard the fixes.** For the HLL, CPC and Theta cases, temporarily check out the submodule and vendored copy at 5.2.0 in a scratch copy of the repo under `/tmp` (do **not** modify the working repo's submodule or vendor dir), run the new tests there, and record which fail or crash on 5.2.0. At least the CVE cases should fail (or abort) on 5.2.0; if one passes on 5.2.0 too, say so in the report — it may be pinning nothing.
- [ ] **Step 3: Run on 5.3.0 (working repo)** — `cargo test -p apache-datasketches --all-features --test <each file>` — Expected: PASS.
- [ ] **Step 4: Lint, format, commit**

```bash
cargo clippy --workspace --all-features --all-targets -- -D warnings
git add apache-datasketches/tests
git commit -m "test: pin the datasketches-cpp 5.3.0 deserialization fixes"
```

---

### Task 3: Port upstream's HLL union behavior tests

**Files:**
- Modify: `apache-datasketches/tests/hll_union_test.rs`

**Interfaces:**
- Consumes: the 5.3.0 bump.

Port these `TEST_CASE`s from `vendor/datasketches-cpp/hll/test/HllUnionTest.cpp` (tag 5.3.0) through the safe `HllSketch`/`HllUnion` API, each with a citing comment:

- `hll union: downsampling merge is not empty`
- `hll union: mixed lgConfigK estimate is merge-order independent`
- `hll union: scalar update after downsampling merge`
- `hll union: serialization is grouping independent` (compare the safe API's serialized bytes, e.g. `serialize_compact()`, of unions built in different groupings)

If an upstream case needs an API this crate does not expose, port the parts that are expressible and say what was dropped in the comment.

- [ ] **Step 1: Write the tests**
- [ ] **Step 2: Prove at least the order/grouping tests pin the fix**: in the same kind of `/tmp` scratch copy at 5.2.0 as Task 2, run them and record whether they fail there (upstream added them for the 5.3.0 KxQ fix, so at least one should).
- [ ] **Step 3: Run on 5.3.0** — Expected: PASS.
- [ ] **Step 4: Commit** — `git commit -m "test: pin HLL union merge-order and grouping independence (5.3.0)"`

---

### Task 4: Docs, stale references and CHANGELOG

**Files:**
- Modify: `README.md`, `AGENTS.md`, `CHANGELOG.md`, `apache-datasketches/README.md`, `apache-datasketches/src/req/sketch.rs`, `apache-datasketches-sys/build.rs`, shim/test comments with upstream line references, test-file "ported from … tag 5.2.0" headers, `docs/superpowers/specs/2026-10-23-req-rust-bindings-design.md` (upstream-facts wording for `get_RSE`)

- [ ] **Step 1: Version strings.** `grep -rn "5\.2\.0" --include=*.md --include=*.rs --include=*.cc --include=*.h . | grep -v '^./vendor\|^./apache-datasketches-sys/vendor\|^./target'` and handle each hit:
  - "pinned to tag 5.2.0" / "built against … 5.2.0" (root README, CHANGELOG header paragraph, AGENTS.md) → 5.3.0.
  - Statements about bugs still present in 5.3.0 (REQ `k` truncation, empty-iterator over-read, 20-byte padding, KLL infinite loop) → "5.2.0 and 5.3.0" (or drop the version).
  - Statements about `get_RSE` returning a lower bound → say it was 5.2.0 behavior, fixed in 5.3.0.
  - "Ported from … (tag 5.2.0)" test headers → "(tag 5.2.0; unchanged in 5.3.0)" if the upstream test file did not change between the tags (`git -C vendor/datasketches-cpp diff --quiet 5.2.0 5.3.0 -- <path>`), otherwise "(tag 5.3.0)" and, if the port now lags upstream additions, a note naming the unported cases.
  - CHANGELOG entries for already-released versions are history — do not edit them.
- [ ] **Step 2: Upstream line references.** For every `<header>.hpp:<line>` reference in comments under `apache-datasketches-sys/cpp/`, `apache-datasketches/src/` and `apache-datasketches/tests/`, check it against the 5.3.0 header and fix it (known: `kll_sketch_impl.hpp:779` → `788`; `req_sketch_impl.hpp` refs shift by +1 to +9; `theta_set_difference_base_impl.hpp:39-82` → `39-84`; `theta_sketch_impl.hpp:52,58` → `53,59`). Verify each by opening the file.
- [ ] **Step 3: build.rs comment.** The `-Wno-unnecessary-virtual-specifier` comment names `hll_sketch_alloc`; 5.3.0 made its destructor non-virtual. Reword to name `AuxHashMap` (still `final` with a virtual destructor). Keep the flag.
- [ ] **Step 4: CHANGELOG.** Under `## [Unreleased]`, add (keep existing content):

```markdown
### Security
- **Upgraded the vendored Apache DataSketches C++ from 5.2.0 to 5.3.0**,
  which fixes deserialization vulnerabilities reachable through this crate's
  safe `deserialize`/`wrap` APIs on untrusted bytes:
  [CVE-2026-103501](https://www.cve.org/CVERecord?id=CVE-2026-103501)
  (HLL, heap buffer overflow),
  [CVE-2026-103513](https://www.cve.org/CVERecord?id=CVE-2026-103513)
  (CPC, out-of-bounds read/write) and
  [CVE-2026-103635](https://www.cve.org/CVERecord?id=CVE-2026-103635)
  (compact Theta, out-of-bounds read). KLL and REQ were already guarded by
  this crate's own layout validation; 5.3.0 also removes a latent undefined
  behavior in their deserialize paths. Regression tests pin each fix.
```

and under `### Changed` (create it if absent; merge with existing entries):

```markdown
- Theta and Tuple sketches now accept `lg_k = 4` (the minimum was 5),
  matching Java.
- HLL union estimates and serialized union bytes can differ slightly from
  builds on 5.2.0: upstream repaired a merge-order dependence and made the
  composite estimator Java-bit-exact. HLL flag bit 0x20 is no longer written
  or read.
- Compact Theta sketches holding exactly one item now set the single-item
  flag (matching Java), and empty compact Theta/Tuple sketches serialize a
  seed hash of 0. Old and new bytes deserialize either way.
- Theta bounds on crafted bytes with `theta = 0` now return an error
  instead of infinity/NaN; Theta a-not-b with an empty B carrying a foreign
  seed hash no longer aborts the process.
- `ReqSketch::a_priori_rank_error` is unchanged in value; it now calls
  upstream's `get_RSE` directly, which 5.3.0 fixed.
```

Wrap all Markdown at ~80 columns.

- [ ] **Step 5: Full verification** (same commands as Task 1 Step 7), plus `./benches/cpp_reference/compare.sh --ladder --reps 2` must agree on every scenario (parity only; quote no figures), and the packaged sys crate must build:

```bash
cargo package -p apache-datasketches-sys --no-verify --allow-dirty
v=$(grep '^version' apache-datasketches-sys/Cargo.toml | cut -d'"' -f2)
w=$(mktemp -d) && tar xzf target/package/apache-datasketches-sys-$v.crate -C $w && (cd $w/apache-datasketches-sys-$v && cargo build --all-features) ; rm -rf $w
```

- [ ] **Step 6: Commit** — `git commit -m "docs: update for datasketches-cpp 5.3.0"`
