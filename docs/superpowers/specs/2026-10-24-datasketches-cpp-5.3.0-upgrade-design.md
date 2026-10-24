# Upgrade vendored datasketches-cpp 5.2.0 → 5.3.0 — Design

## Purpose

Move the pinned Apache DataSketches C++ library from 5.2.0 to 5.3.0
(released 2026-10-08) for every bound family (HLL, Theta, CPC, Tuple, KLL,
REQ). The main reason is security: 5.3.0 fixes five deserialization CVEs,
three of which are reachable through this crate's safe `deserialize`/`wrap`
APIs on untrusted bytes:

- [CVE-2026-103501](https://www.cve.org/CVERecord?id=CVE-2026-103501)
  (moderate) — heap buffer overflow in HLL sketch deserialization.
- [CVE-2026-103513](https://www.cve.org/CVERecord?id=CVE-2026-103513)
  (moderate) — out-of-bounds read and write in CPC sketch deserialization.
- [CVE-2026-103635](https://www.cve.org/CVERecord?id=CVE-2026-103635)
  (low) — out-of-bounds read in compact Theta sketch deserialization.

(The other two, Count-Min and VarOpt, are in families not yet bound.)

The upgrade also picks up upstream's REQ `get_RSE` fix, which this crate
currently works around.

**Scope:** the version bump, the code changes it requires, regression tests
for the security fixes that affect bound families, one behavior test, and the
CHANGELOG and doc updates.

**Out of scope:** new API from 5.3.0 (Theta's `compact(ordered, trim)`);
changing any workaround 5.3.0 does not fix (listed below); publishing.

## Impact analysis summary

Established against the 5.3.0 headers in a scratch copy of the repo (both the
submodule and the vendored copy replaced), before this design:

- Every shim and bench `.cc` compiles against 5.3.0 with no new warnings.
- Exactly 14 test targets fail, from two causes, and the full workspace
  suite passes once both are addressed; `compare.sh` agrees and `run.sh`
  output (`estimate=`/`bytes=`) is identical to 5.2.0.
  1. **REQ `get_RSE`**: now returns `get_rank_ub(k, 2, rank, 1, n, hra) -
     rank` (the true one-standard-deviation error). The shim's
     `req_{doubles,floats}_a_priori_rank_error` computes `rank - get_RSE(..)`
     for 5.2.0's lower bound, which on 5.3.0 is wrong.
  2. **Theta/Tuple `MIN_LG_K` 5 → 4** (matching Java). Thirteen assertions
     across `apache-datasketches/tests` and `apache-datasketches-sys/tests`
     expect `lg_k = 4` to be rejected.
- Behavior changes that need no code change (CHANGELOG only):
  - HLL union results (and their serialized bytes) can differ from 5.2.0:
    the deferred KxQ rebuild is repaired so union results no longer depend on
    merge order; the composite/bitmap estimator uses an fdlibm `log`
    (Java-bit-exact); RSE constants are full precision. No test pins these.
  - HLL flag bit 0x20 ("full size") is no longer written or read.
  - Theta compact sketches holding exactly one item set the single-item flag
    (flags byte `0x1A` → `0x3A`); byte length unchanged.
  - Empty compact Theta and Tuple sketches serialize seed hash 0; readers
    skip the seed check for empty sketches, so old and new bytes interoperate.
  - Theta a-not-b ignores the seed hash of an empty B. On 5.2.0, A minus a
    deserialized empty B carrying a foreign seed hash threw from a bridge
    function declared without `Result` — a process abort; 5.3.0 removes it.
  - `binomial_bounds::check_theta` rejects `theta <= 0`, so bounds on crafted
    compact bytes with `theta = 0` return `Err` instead of inf/NaN (the bounds
    bridges already return `Result`).
  - KLL and REQ deserialize no longer dereference an empty `std::optional`
    for min/max scratch space (latent UB on our path, now gone).
- A new header, `common/include/fdlibm_log.hpp`, is included by HLL; it is
  already on the include path. The vendored `LICENSE` gains an FDLIBM
  appendix.

### Workarounds that stay

5.3.0 does not fix these; their guards and docs remain, with version
references updated to "5.2.0 and 5.3.0" where they name a version:

- REQ constructor truncates `k` to 8 bits — `ReqSketch::new` stays capped at
  254.
- REQ iterator reads past the end of an empty sketch — shim and Rust guards
  stay.
- REQ serialized size over-counts by 20 bytes for `n = 2..=4` — the
  validator's padding acceptance stays.
- KLL `floor_of_log2_of_fraction` loops forever for `n >= 2^63` — the
  validator bound and `KllSketch::merge` guard stay.
- `quantiles_sorted_view::get_quantile` casts NaN / out-of-range ranks
  unchecked — the shim rank checks stay.
- `validate_kll_layout` and `validate_req_layout` need no logic changes (the
  serialized layouts are unchanged).

## Design

Branch `chore/datasketches-cpp-5.3.0`. Four commits, each leaving the suite
green.

### 1. The bump (one commit)

- Move the `vendor/datasketches-cpp` submodule to tag `5.3.0`.
- Refresh `apache-datasketches-sys/vendor/datasketches-cpp` from it with the
  recipe in `apache-datasketches-sys/vendor/README.md`, including `LICENSE`
  and `NOTICE`. `vendor_drift_test` must pass.
- **REQ:** `req_{doubles,floats}_a_priori_rank_error` in
  `apache-datasketches-sys/cpp/req/req_sketch_shim.cc` return
  `req_sketch<T>::get_RSE(k, rank, hra, n)` directly. Replace the comment at
  that spot (which tells the reader to do exactly this on upgrade) with one
  stating that from 5.3.0 `get_RSE` returns the error directly. Update the
  `ReqSketch::a_priori_rank_error` rustdoc and the README/CHANGELOG/spec text
  that describe `get_RSE` as returning a lower bound, so they describe it as
  a 5.2.0 behavior fixed in 5.3.0. **The public semantics do not change**:
  the existing pinned tests (`a_priori_rank_error(12, 0.5, High, 1000) ≈
  0.0054433`, `0.0` in the exact region, and `0.00108866` vs `0.007` at rank
  0.9 for High vs Low) must pass unmodified — they are the proof.
- **Theta/Tuple `lg_k`:** change each assertion that `lg_k = 4` is rejected
  to assert it is accepted, and add an assertion that `lg_k = 3` is rejected
  in the same places (theta sketch and union, ArrayOfDoubles sketch and
  union, generic tuple sketch and union, and the matching sys link tests).
  Update builder rustdoc and test comments that state the range as `5..=26`
  (or "`MIN_LG_K == 5`") to `4..=26`.

### 2. Security regression tests (one commit)

Port upstream's new 5.3.0 tests for the fixes that affect bound families
into this crate's suites, through the safe API, asserting
`Err(SketchError::Deserialization(_))` (or the family's equivalent) and no
crash:

- **HLL** (CVE-2026-103501): malformed CouponList with a coupon count at or
  above the list capacity.
- **CPC** (CVE-2026-103513): corrupt `lg_k`; corrupt `num_coupons`.
- **Theta** (CVE-2026-103635 and #538): truncated preamble; corrupt serial
  version 4 header; entry-bits overflow.
- **Theta a-not-b**: A minus an empty B whose bytes carry a foreign seed
  hash now succeeds (previously aborted the process).

Each test cites the upstream test it ports (file and case name at tag
5.3.0). Where upstream builds the malformed bytes by mutating a genuine
sketch's serialization, do the same here.

### 3. HLL union behavior test (in commit 2 or its own)

Port upstream's union merge-order / grouping independence test: unions built
from the same sketches in different orders or groupings give identical
estimates and identical serialized bytes. This pins the 5.3.0 KxQ fix.

### 4. Docs and CHANGELOG (one commit)

- Pinned-version strings: root `README.md` ("pinned to tag 5.2.0"), the
  `CHANGELOG.md` header paragraph, `AGENTS.md`, and the "ported from … (tag
  5.2.0)" headers in test files — the ported *content* of existing tests is
  still from 5.2.0, so those headers say "tag 5.2.0 (still current in
  5.3.0)" only where the upstream test is unchanged, else are updated with
  what changed.
- Statements about upstream bugs that remain (REQ `k` truncation etc.) name
  "5.2.0 and 5.3.0" or drop the version.
- Shifted upstream line references in comments (e.g.
  `req_sketch_impl.hpp`, `kll_sketch_impl.hpp:779` → `788`,
  `theta_set_difference_base_impl.hpp`, `theta_sketch_impl.hpp`) are
  corrected against the 5.3.0 sources.
- `build.rs`: the comment on `-Wno-unnecessary-virtual-specifier` names
  `hll_sketch_alloc`, whose destructor is no longer virtual in 5.3.0; reword
  it to name `AuxHashMap`, which still triggers the warning. The flag stays.
- `CHANGELOG.md`, under `[Unreleased]`: a **Security** subsection naming the
  three reachable CVEs with IDs and the families affected (and that KLL/REQ
  were already guarded by this crate's validators); **Changed** entries for
  `lg_k = 4` now accepted by Theta/Tuple, HLL union estimates/bytes possibly
  differing from builds on 5.2.0 (merge-order fix), HLL bit 0x20 ignored,
  the Theta single-item flag, empty compact seed hash 0, `theta = 0` bounds
  returning `Err`; and a note that REQ `a_priori_rank_error` semantics are
  unchanged (the computation moved upstream).

## Verification

- `cargo test --workspace --all-features` (release and debug), clippy and
  rustdoc with `-D warnings`, `cargo fmt --check`, the no-family guard.
- The packaged sys crate builds standalone with all features (the CI
  `package` job's steps).
- `./benches/cpp_reference/compare.sh --ladder` agrees on every scenario.
  It is a parity check only; no figures are quoted.

## Release

Additive and security-relevant; goes under `[Unreleased]` with KLL and REQ.
Because it fixes reachable CVEs, the next release should not wait long after
this lands — the maintainer decides when; publishing needs their go-ahead.
