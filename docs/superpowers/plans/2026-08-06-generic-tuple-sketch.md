# Generic Tuple Sketch (callback core) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `apache_datasketches::tuple::generic::TupleSketch<S>` — a Tuple sketch whose per-entry summary is an arbitrary Rust type defined by a downstream crate, with C++ calling back into Rust to clone and combine summaries.

**Architecture:** One type-erased C++ instantiation, `update_tuple_sketch<DynSummary, DynSummary, DynUpdatePolicy>`, serves every summary type. `DynSummary` is a C++ class holding `std::optional<rust::Box<RustSummary>>`; its move and destroy are `rust::Box`'s own, and its copy constructor calls a clone trampoline. Three `extern "Rust"` functions (`clone`, `union_combine`, `intersection_combine`) are the entire C++→Rust surface. On the Rust side, the sys crate owns the opaque `RustSummary` and a minimal `RawSummaryOps` trait (cxx requires the opaque type to live in the bridge's own crate); the safe crate owns the user-facing `TupleSummary` trait and a private adapter.

**Tech Stack:** Rust 2021, `cxx` 1.x / `cxx-build` 1.x, C++17, vendored `datasketches-cpp` headers (header-only), `thiserror` 1.x.

**Spec:** `docs/superpowers/specs/2026-08-06-generic-tuple-sketch-design.md`

## Global Constraints

- Scope is the **callback core only**. Summary serialization and cross-language interop are a separate, later design. Do not add `serialize`, `deserialize`, a `SerDe`, or any `size_of_item` anywhere in this plan.
- `ArrayOfDoublesSketch` and its family are **not modified, deprecated, or reimplemented** on this framework. The two coexist permanently. Do not touch any existing file under `apache-datasketches/src/tuple/` other than `mod.rs` (to add the `generic` submodule).
- **No new Cargo feature.** Everything here ships under the existing `tuple` feature.
- Errors reuse the existing `crate::error::SketchError` enum. **Do not add a new variant.**
- Every generic type is `Send` and **not** `Sync`. The `+ Send` on `Box<dyn RawSummaryOps + Send>` is load-bearing — without it a sketch could carry a non-`Send` summary across threads while the wrapper claims `Send`.
- `apache-datasketches/src/lib.rs` has `#![warn(missing_docs)]` and must stay clean. Every public item, including trait methods and associated types, needs rustdoc.
- C++ shim code lives in `namespace apache_datasketches_rs`.
- Commit messages must **not** include a `Co-Authored-By` trailer.
- Never edit anything under `vendor/datasketches-cpp/` (the repo-root submodule) or `apache-datasketches-sys/vendor/`.
- **Every `#[cxx::bridge]` free-function name and shared type name must be globally unique across all bridges in the sys crate.** cxx keys the `extern "C"` trampoline on namespace and name only — not parameter types, not the bridge module. This rule is recorded in `AGENTS.md` and enforced by a duplicate-name check in `build.rs`, which will fail the build if violated. All new names in this plan carry a `tuple_generic_` / `rust_summary_` / `Dyn` prefix for this reason.
- `build.rs` asserts its bridge and shim file lists are exhaustive — a missing file is a hard build failure, not a silent skip. Add each new file to the lists in the task that creates it.

### Verified upstream facts this plan depends on

These were confirmed against the vendored headers. An implementer who doubts a design choice should re-read these before deviating.

- The entry table allocates raw memory and placement-news every entry (`theta/include/theta_update_sketch_base_impl.hpp:47,67,211,241,273`). It **never default-constructs** a `Summary`. Move-construction is used on rehash/resize (`:217,246,292`); copy-construction only on whole-sketch copy (`:70`).
- `rust::Box` is move-only, not default-constructible (`Box() = delete`, `rust/cxx.h:298`), its move constructor nulls the source (`:775-777`), and its destructor null-checks (`:796-800`). Moved-from boxes are safe to destroy.
- The update path is fixed (`tuple/include/tuple_sketch_impl.hpp:213-224`): on a new key it calls `policy_.create()` **with no arguments**, then `policy_.update(summary, value)`, then moves the result into the table. On an existing key it calls only `policy_.update(existing, value)`. This is why `DynSummary` needs a disengaged state.
- Union never calls `create()`: it inserts the incoming entry directly on a new key and invokes `policy_(Summary&, const Summary&)` only on collision (`theta/include/theta_union_base_impl.hpp:50-52`). Intersection invokes its policy only on a match (`theta_intersection_base_impl.hpp:70-72`).
- `compact_tuple_sketch`'s `serialize`/`deserialize` are **member templates**, not virtual (`tuple/include/tuple_sketch.hpp:516-558`). They are instantiated only when called, so `compact_tuple_sketch<DynSummary>` compiles without any `serde<DynSummary>` existing. This is what makes deferring serialization possible.
- cxx opaque `extern "Rust"` types must be `Sized` and `Unpin`, and must be defined in the same crate as the bridge. A bare `dyn Trait` cannot be an opaque type — hence the concrete `RustSummary` newtype wrapping a boxed trait object.
- cxx wraps every `extern "Rust"` function in `prevent_unwind` (`cxx-1.0.198/src/unwind.rs`), turning a panic into a deterministic abort rather than UB.

### Cross-cutting naming decisions

Fixed across tasks; a different choice breaks a neighbouring task.

- sys crate opaque type: `RustSummary`. sys crate trait: `RawSummaryOps`. Trampolines: `rust_summary_clone`, `rust_summary_union_combine`, `rust_summary_intersection_combine`.
- C++ summary wrapper: `DynSummary`. Policies: `DynUpdatePolicy`, `DynUnionPolicy`, `DynIntersectionPolicy`.
- C++ type aliases: `dyn_update_sketch`, `dyn_compact_sketch`, `dyn_union`, `dyn_intersection`, `dyn_a_not_b`.
- C++ shim classes: `TupleGenericSketchShim`, `CompactTupleGenericSketchShim`, `TupleGenericUnionShim`, `TupleGenericIntersectionShim`, `TupleGenericAnotBShim`.
- Safe crate module path: `apache_datasketches::tuple::generic`.
- Safe crate types: `TupleSummary` (trait), `TupleSketchBuilder<S>`, `TupleSketch<S>`, `CompactTupleSketch<S>`, `TupleUnionBuilder<S>`, `TupleUnion<S>`, `TupleIntersection<S>`, `TupleAnotB<S>`, `TupleInput<S>` (sealed), `tuple_jaccard_similarity<S>`.
- `JaccardBounds` is **reused** from `apache_datasketches::tuple`, not redefined.
- `ResizeFactor` is **reused** from `apache_datasketches::tuple`, not redefined. The generic builders take the existing `tuple::ResizeFactor` and convert via the existing `From<ResizeFactor> for sys::TupleResizeFactor` impl.

### Verified cxx facts — bridge layout is forced, not chosen

These were established by building throwaway probe crates against cxx 1.0.198. Each failed approach produced the quoted error. **Do not try to "clean up" the layout below; these are the constraints, not preferences.**

1. **`type RustSummary = crate::path::RustSummary;` inside an `extern "Rust"` block does not compile.** cxx rejects it outright: `error[cxxbridge]: type alias in extern "Rust" block is not supported`. The cross-bridge aliasing that works for `extern "C++"` types has no `extern "Rust"` equivalent.

2. **Declaring `type RustSummary;` in two bridges does not compile either**: `error[E0119]: conflicting implementations of trait cxx::private::RustType for type RustSummary`. An `extern "Rust"` opaque type may be declared in **exactly one** bridge module per crate.

3. **Therefore every `extern "C++"` function whose signature mentions `RustSummary` must live in the same bridge module as the `extern "Rust"` block.** In this plan that is the sketch shim (its `update_*` take `&RustSummary`) and the compact shim (its entry accessor returns `Box<RustSummary>`). They share one bridge file, `apache-datasketches-sys/src/tuple_generic.rs`. Union, intersection, a-not-b, and Jaccard never mention `RustSummary` — they only pass shim types — so they get their own bridge files and alias the shim types the ordinary way.

4. **That bridge's generated header must not be included by the shim headers.** Doing so produces a genuine cycle, confirmed by probe: `error: no type named 'TupleGenericSketchShim' in namespace 'apache_datasketches_rs'`, because the generated header re-enters the shim header whose include guard is already set. Use the repo's established pattern — forward-declare in the `.h`, `#include` the generated header only in the `.cc`.

5. **The forward declarations must match cxx's emitted signatures exactly, including `noexcept`.** A mismatch gives `error: exception specification in declaration does not match previous declaration`. cxx emits:

   ```cpp
   namespace apache_datasketches_rs {
     struct RustSummary;
     ::rust::Box<::apache_datasketches_rs::RustSummary>
         rust_summary_clone(::apache_datasketches_rs::RustSummary const &summary) noexcept;
     void rust_summary_union_combine(
         ::apache_datasketches_rs::RustSummary &target,
         ::apache_datasketches_rs::RustSummary const &other) noexcept;
     void rust_summary_intersection_combine(
         ::apache_datasketches_rs::RustSummary &target,
         ::apache_datasketches_rs::RustSummary const &other) noexcept;
   }
   ```

6. **`DynSummary` can be fully header-inline against only a forward declaration of `RustSummary`** — including a `std::optional<rust::Box<RustSummary>>` member and a copy constructor calling the clone trampoline. This was verified by probe (built and round-tripped at runtime), so the policies stay inline and no out-of-line split is needed.

7. `RustSummary` is emitted as `struct RustSummary final : public ::rust::Opaque` with `~RustSummary() = delete`. C++ **cannot** read or write the Rust value's fields — it can only pass the reference back to Rust. Any shim code that tries to reach inside is a mistake.

### Deviation from the spec's repo layout

The spec's "Repo layout additions" section lists a standalone
`src/tuple_generic_summary.rs` alongside separate `src/tuple_generic_sketch.rs`
and `src/tuple_generic_compact.rs` bridges. Finding 3 above makes that
impossible: those three would each need `RustSummary` in scope, and it can be
declared in only one bridge. This plan therefore merges them into a single
`src/tuple_generic.rs`, and the remaining four bridges keep their own files as
the spec describes. Everything else in that layout section is unchanged. The
spec was written before the cxx behaviour was probed; the plan is correct and
the spec's file list on this one point is not.

---

### Task 1: The `extern "Rust"` summary bridge, `DynSummary`, and the policies

The foundation and the riskiest task: this is the repo's first `extern "Rust"` bridge. Everything after it is the familiar three-layer pattern. Its test implements `RawSummaryOps` directly in the sys crate, so it proves the trampolines round-trip without needing the safe-crate trait to exist yet.

**Files:**
- Create: `apache-datasketches-sys/src/tuple_generic.rs`
- Create: `apache-datasketches-sys/cpp/tuple/dyn_summary.h`
- Create: `apache-datasketches-sys/cpp/tuple/dyn_summary.cc`
- Create (test): `apache-datasketches-sys/tests/tuple_generic_summary_link_test.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`
- Modify: `apache-datasketches-sys/build.rs`
- Modify: `apache-datasketches-sys/Cargo.toml`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - Rust (sys): `apache_datasketches_sys::tuple_generic::{RustSummary, RawSummaryOps}`. `RawSummaryOps` has `clone_boxed(&self) -> Box<dyn RawSummaryOps + Send>`, `union_combine(&mut self, other: &dyn RawSummaryOps)`, `intersection_combine(&mut self, other: &dyn RawSummaryOps)`, `as_any(&self) -> &dyn Any`. `RustSummary::new(Box<dyn RawSummaryOps + Send>) -> Self`, `RustSummary::ops(&self) -> &dyn RawSummaryOps`.
  - C++: `apache_datasketches_rs::DynSummary` with `engaged()`, `get()`, `assign_clone_of(const RustSummary&)`; `DynUpdatePolicy`, `DynUnionPolicy`, `DynIntersectionPolicy`; the five type aliases `dyn_update_sketch`, `dyn_compact_sketch`, `dyn_union`, `dyn_intersection`, `dyn_a_not_b`.

- [ ] **Step 1: Write the failing link test**

Create `apache-datasketches-sys/tests/tuple_generic_summary_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{RawSummaryOps, RustSummary};
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};

static CLONES: AtomicUsize = AtomicUsize::new(0);
static DROPS: AtomicUsize = AtomicUsize::new(0);

/// A summary that sums on union and takes the minimum on intersection, so the
/// two combine trampolines are distinguishable if they are ever cross-wired.
#[derive(Debug)]
struct TestSummary(i64);

impl Drop for TestSummary {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

impl RawSummaryOps for TestSummary {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        CLONES.fetch_add(1, Ordering::SeqCst);
        Box::new(TestSummary(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        let other = other.as_any().downcast_ref::<TestSummary>().unwrap();
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        let other = other.as_any().downcast_ref::<TestSummary>().unwrap();
        self.0 = self.0.min(other.0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn summary(v: i64) -> RustSummary {
    RustSummary::new(Box::new(TestSummary(v)))
}

fn value_of(s: &RustSummary) -> i64 {
    s.ops().as_any().downcast_ref::<TestSummary>().unwrap().0
}

#[test]
fn clone_produces_an_independent_copy() {
    let a = summary(7);
    let b = apache_datasketches_sys::tuple_generic::clone_for_test(&a);
    assert_eq!(value_of(&b), 7);
    // Mutating the clone must not affect the original.
    let mut b = b;
    apache_datasketches_sys::tuple_generic::union_for_test(&mut b, &summary(3));
    assert_eq!(value_of(&b), 10);
    assert_eq!(value_of(&a), 7);
}

#[test]
fn union_and_intersection_are_distinct() {
    let mut u = summary(4);
    apache_datasketches_sys::tuple_generic::union_for_test(&mut u, &summary(6));
    assert_eq!(value_of(&u), 10, "union should sum");

    let mut i = summary(4);
    apache_datasketches_sys::tuple_generic::intersection_for_test(&mut i, &summary(6));
    assert_eq!(value_of(&i), 4, "intersection should take the minimum");
}

#[test]
fn clones_and_drops_balance() {
    CLONES.store(0, Ordering::SeqCst);
    DROPS.store(0, Ordering::SeqCst);
    {
        let a = summary(1);
        let _b = apache_datasketches_sys::tuple_generic::clone_for_test(&a);
        let _c = apache_datasketches_sys::tuple_generic::clone_for_test(&a);
    }
    assert_eq!(CLONES.load(Ordering::SeqCst), 2);
    assert_eq!(DROPS.load(Ordering::SeqCst), 3, "original plus two clones");
}
```

Register it in `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "tuple_generic_summary_link_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_summary_link_test`
Expected: FAIL — compile error, `unresolved import apache_datasketches_sys::tuple_generic_summary`.

- [ ] **Step 3: Write the `extern "Rust"` bridge**

Create `apache-datasketches-sys/src/tuple_generic.rs`:

```rust
//! The `extern "Rust"` surface the generic Tuple sketch's C++ layer calls
//! back into, plus the `extern "C++"` shims whose signatures mention a
//! summary.
//!
//! Those have to share one bridge module: cxx allows an `extern "Rust"`
//! opaque type to be declared in exactly one bridge per crate (a second
//! declaration is a conflicting `RustType` impl), and `extern "Rust"` blocks
//! do not support the `type X = crate::path::X;` aliasing that `extern "C++"`
//! blocks use to share a type across bridges. So the sketch and compact
//! shims — the only ones taking or returning a `RustSummary` — live here.
//! Union, intersection, a-not-b, and Jaccard pass only shim types and get
//! their own bridge files.
//!
//! Later tasks add the `extern "C++"` blocks below; this task creates only
//! the `extern "Rust"` one.

use std::any::Any;

/// The operations the C++ layer invokes on a type-erased Rust summary.
///
/// This is the sys crate's minimal internal trait. Users implement
/// `apache_datasketches::tuple::generic::TupleSummary` instead; the safe
/// crate adapts one to the other.
pub trait RawSummaryOps: Any + Send {
    /// Deep-copy this summary. Called when C++ copies a sketch or converts
    /// an update sketch to a compact one.
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send>;

    /// Merge `other` into `self` with union semantics.
    fn union_combine(&mut self, other: &dyn RawSummaryOps);

    /// Merge `other` into `self` with intersection semantics.
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps);

    /// Upcast for the concrete-type recovery the safe crate's adapter does.
    fn as_any(&self) -> &dyn Any;
}

/// A concrete, `Sized + Unpin` newtype around a boxed [`RawSummaryOps`].
///
/// cxx cannot expose a bare `dyn Trait` as an opaque `extern "Rust"` type —
/// opaque types must be `Sized` and `Unpin` — so the trait object is wrapped
/// in this struct, which is what actually crosses the boundary.
pub struct RustSummary {
    ops: Box<dyn RawSummaryOps + Send>,
}

impl RustSummary {
    /// Wraps a boxed summary implementation.
    pub fn new(ops: Box<dyn RawSummaryOps + Send>) -> Self {
        Self { ops }
    }

    /// Borrows the underlying operations object.
    pub fn ops(&self) -> &dyn RawSummaryOps {
        &*self.ops
    }

    /// Mutably borrows the underlying operations object.
    pub fn ops_mut(&mut self) -> &mut dyn RawSummaryOps {
        &mut *self.ops
    }
}

/// Runs `f`, converting any panic into a deliberate abort with a message
/// naming the operation.
///
/// cxx already prevents a panic from unwinding into C++ (it turns one into a
/// deterministic abort via a double-panic guard), so this does not add
/// safety — it adds diagnosis, replacing an unexplained fatal signal with an
/// actionable message. Upstream's combine policy returns `void`, so there is
/// no way to report failure to C++ and have it unwind the insert; continuing
/// would leave the sketch logically undefined, which is worse than stopping.
fn abort_on_panic<F, R>(what: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "apache-datasketches: a TupleSummary::{what} implementation panicked \
                 while called from C++. Panics cannot cross the FFI boundary, and the \
                 sketch cannot be left in a consistent state, so the process is aborting."
            );
            std::process::abort();
        }
    }
}

fn rust_summary_clone(summary: &RustSummary) -> Box<RustSummary> {
    abort_on_panic("clone", || {
        Box::new(RustSummary {
            ops: summary.ops.clone_boxed(),
        })
    })
}

fn rust_summary_union_combine(target: &mut RustSummary, other: &RustSummary) {
    abort_on_panic("union_combine", || {
        target.ops.union_combine(&*other.ops)
    })
}

fn rust_summary_intersection_combine(target: &mut RustSummary, other: &RustSummary) {
    abort_on_panic("intersection_combine", || {
        target.ops.intersection_combine(&*other.ops)
    })
}

/// Test-only wrapper around the clone trampoline.
#[doc(hidden)]
pub fn clone_for_test(summary: &RustSummary) -> Box<RustSummary> {
    rust_summary_clone(summary)
}

/// Test-only wrapper around the union trampoline.
#[doc(hidden)]
pub fn union_for_test(target: &mut RustSummary, other: &RustSummary) {
    rust_summary_union_combine(target, other)
}

/// Test-only wrapper around the intersection trampoline.
#[doc(hidden)]
pub fn intersection_for_test(target: &mut RustSummary, other: &RustSummary) {
    rust_summary_intersection_combine(target, other)
}

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    extern "Rust" {
        type RustSummary;

        fn rust_summary_clone(summary: &RustSummary) -> Box<RustSummary>;
        fn rust_summary_union_combine(target: &mut RustSummary, other: &RustSummary);
        fn rust_summary_intersection_combine(target: &mut RustSummary, other: &RustSummary);
    }
}
```

Add to `apache-datasketches-sys/src/lib.rs`, in the `tuple` block:

```rust
#[cfg(feature = "tuple")]
pub mod tuple_generic;
```

Add `"src/tuple_generic.rs"` to the `tuple` bridge list in `build.rs`, and a matching `cargo:rerun-if-changed` line.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_summary_link_test`
Expected: PASS (3 tests).

This proves the Rust side compiles and the trampolines behave, before any C++ depends on them.

- [ ] **Step 5: Write `DynSummary` and the policies**

Create `apache-datasketches-sys/cpp/tuple/dyn_summary.h`:

```cpp
#pragma once
#include <optional>
#include <type_traits>
#include <utility>
#include "rust/cxx.h"
#include "tuple_sketch.hpp"
#include "tuple_union.hpp"
#include "tuple_intersection.hpp"
#include "tuple_a_not_b.hpp"

namespace apache_datasketches_rs {

// Forward declarations of the cxx-generated `extern "Rust"` surface.
//
// We deliberately do NOT `#include "tuple_generic.rs.h"` here. That header
// carries an `include!` back to the shim headers, and including it from a
// shim header produces a genuine cycle: the generated header re-enters this
// one while its include guard is already set, and then fails with
// "no type named 'TupleGenericSketchShim' in namespace". Same rationale as
// the ResizeFactor forward declaration in theta_sketch_shim.h. The
// definitions arrive in dyn_summary.cc, which includes the generated header
// after this one is complete.
//
// These signatures must match cxx's output exactly, INCLUDING `noexcept` —
// a mismatch is "exception specification in declaration does not match
// previous declaration".
struct RustSummary;
::rust::Box<RustSummary> rust_summary_clone(RustSummary const& summary) noexcept;
void rust_summary_union_combine(RustSummary& target, RustSummary const& other) noexcept;
void rust_summary_intersection_combine(RustSummary& target, RustSummary const& other) noexcept;

// A C++ value type wrapping an owned, type-erased Rust summary.
//
// Move and destroy are rust::Box's own. Only the copy constructor needs a
// trampoline, and upstream copy-constructs a summary only when a whole sketch
// is copied or an update sketch is converted to a compact one -- never on the
// update or rehash path, which move.
//
// The optional is required, not defensive. Upstream's update path calls
// policy_.create() with no arguments before policy_.update(), and there is no
// universal identity element for an arbitrary user-defined summary. create()
// therefore returns a disengaged DynSummary and the update policy clones into
// it. The disengaged state is transient: it exists only between those two
// calls (tuple_sketch_impl.hpp:218-220) and is never stored in the table.
class DynSummary {
public:
  DynSummary() = default;

  explicit DynSummary(rust::Box<RustSummary> inner) : inner_(std::move(inner)) {}

  DynSummary(DynSummary&&) noexcept = default;
  DynSummary& operator=(DynSummary&&) noexcept = default;
  ~DynSummary() = default;

  DynSummary(const DynSummary& other) {
    if (other.inner_) inner_.emplace(rust_summary_clone(**other.inner_));
  }

  DynSummary& operator=(const DynSummary& other) {
    if (this != &other) {
      if (other.inner_) inner_.emplace(rust_summary_clone(**other.inner_));
      else inner_.reset();
    }
    return *this;
  }

  bool engaged() const { return inner_.has_value(); }

  RustSummary& get() { return **inner_; }
  const RustSummary& get() const { return **inner_; }

  void assign_clone_of(const RustSummary& other) {
    inner_.emplace(rust_summary_clone(other));
  }

private:
  std::optional<rust::Box<RustSummary>> inner_;
};

// Stateless by design: upstream's jaccard_similarity_base default-constructs
// scratch union and intersection policies internally, so a policy that carried
// configuration would silently misbehave there. These carry nothing -- they
// dispatch through the summary object itself.

struct DynUpdatePolicy {
  DynSummary create() const { return DynSummary(); }

  void update(DynSummary& summary, const DynSummary& value) const {
    if (!summary.engaged()) {
      summary.assign_clone_of(value.get());
    } else {
      rust_summary_union_combine(summary.get(), value.get());
    }
  }
};

struct DynUnionPolicy {
  void operator()(DynSummary& summary, const DynSummary& other) const {
    rust_summary_union_combine(summary.get(), other.get());
  }
  void operator()(DynSummary& summary, DynSummary&& other) const {
    rust_summary_union_combine(summary.get(), other.get());
  }
};

struct DynIntersectionPolicy {
  void operator()(DynSummary& summary, const DynSummary& other) const {
    rust_summary_intersection_combine(summary.get(), other.get());
  }
  void operator()(DynSummary& summary, DynSummary&& other) const {
    rust_summary_intersection_combine(summary.get(), other.get());
  }
};

using dyn_update_sketch =
    datasketches::update_tuple_sketch<DynSummary, DynSummary, DynUpdatePolicy>;
using dyn_compact_sketch = datasketches::compact_tuple_sketch<DynSummary>;
using dyn_union = datasketches::tuple_union<DynSummary, DynUnionPolicy>;
using dyn_intersection =
    datasketches::tuple_intersection<DynSummary, DynIntersectionPolicy>;
using dyn_a_not_b = datasketches::tuple_a_not_b<DynSummary>;

} // namespace apache_datasketches_rs
```

Create `apache-datasketches-sys/cpp/tuple/dyn_summary.cc`:

```cpp
#include "dyn_summary.h"
#include "tuple_generic.rs.h" // the real declarations behind dyn_summary.h's forward declarations

// DynSummary is header-only: every member is defined inline, against nothing
// but a forward declaration of RustSummary (verified to compile and round-trip
// — rust::Box only stores a T*, so an incomplete T is fine for the member, the
// copy constructor, and std::optional). This translation unit exists to
// compile the header once on its own and to hold the static assertions below,
// and it is the one place that pulls in the generated header, proving the
// forward declarations above match it.

namespace apache_datasketches_rs {
namespace {

// Compile-time assertions on the properties upstream's entry table requires
// of a Summary. If any of these regress, the failure appears here rather than
// as an inscrutable template error inside datasketches-cpp.
static_assert(std::is_move_constructible<DynSummary>::value,
              "DynSummary must be move-constructible: the entry table "
              "move-constructs on every rehash and resize");
static_assert(std::is_copy_constructible<DynSummary>::value,
              "DynSummary must be copy-constructible: the entry table "
              "copy-constructs when a whole sketch is copied");
static_assert(std::is_destructible<DynSummary>::value,
              "DynSummary must be destructible");
static_assert(std::is_default_constructible<DynSummary>::value,
              "DynSummary must be default-constructible: DynUpdatePolicy::create() "
              "returns a disengaged one");

} // namespace
} // namespace apache_datasketches_rs
```

Add `"cpp/tuple/dyn_summary.cc"` to the `tuple` `.file()` list in `build.rs`, plus `cargo:rerun-if-changed` lines for both the `.h` and the `.cc`.

- [ ] **Step 6: Verify the C++ compiles**

Run: `cargo build -p apache-datasketches-sys --features tuple`
Expected: succeeds. The static assertions confirm `DynSummary` satisfies every property the entry table needs.

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_summary_link_test`
Expected: still PASS (3 tests).

- [ ] **Step 7: Commit**

```bash
git add apache-datasketches-sys/src/tuple_generic.rs apache-datasketches-sys/src/lib.rs apache-datasketches-sys/cpp/tuple/dyn_summary.h apache-datasketches-sys/cpp/tuple/dyn_summary.cc apache-datasketches-sys/tests/tuple_generic_summary_link_test.rs apache-datasketches-sys/Cargo.toml apache-datasketches-sys/build.rs
git commit -m "feat(tuple): add the extern \"Rust\" summary bridge and DynSummary"
```

---

### Task 2: `TupleSummary`, the adapter, and `TupleSketch<S>`

The first end-to-end vertical slice: the user-facing trait, the adapter that erases it to `RawSummaryOps`, the C++ sketch shim, and the typed Rust façade. Large but coherent — the trait cannot be meaningfully tested without a sketch to put it in.

**On the resize factor:** this bridge takes the resize factor as a plain `u8` holding the literal multiplier (`1`, `2`, `4`, or `8`), not as a shared cxx enum. Reusing `TupleResizeFactor` from the ArrayOfDoubles bridge would require cross-bridge sharing of a *shared enum*, which this plan does not rely on because it has not been verified the way the `extern "C++"` opaque-type aliasing has. A second shared enum would mean a second C++ converter for the same concept. The `u8` avoids both, and the safe-crate builder still presents the existing `tuple::ResizeFactor` to users.

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/tuple_generic_sketch_shim.h`
- Create: `apache-datasketches-sys/cpp/tuple/tuple_generic_sketch_shim.cc`
- Modify: `apache-datasketches-sys/src/tuple_generic.rs` (add the `extern "C++"` block)
- Create (test): `apache-datasketches-sys/tests/tuple_generic_sketch_link_test.rs`
- Create: `apache-datasketches/src/tuple/generic/mod.rs`
- Create: `apache-datasketches/src/tuple/generic/summary.rs`
- Create: `apache-datasketches/src/tuple/generic/builder.rs`
- Create: `apache-datasketches/src/tuple/generic/sketch.rs`
- Create (test): `apache-datasketches/tests/tuple_generic_sketch_smoke_test.rs`
- Modify: `apache-datasketches/src/tuple/mod.rs`, both `Cargo.toml`s, `build.rs`

**Interfaces:**
- Consumes: `RustSummary`, `RawSummaryOps`, `DynSummary`, `DynUpdatePolicy`, `dyn_update_sketch` (Task 1).
- Produces:
  - C++: `TupleGenericSketchShim` with `inner()`; free function `new_tuple_generic_sketch(uint8_t lg_k, uint8_t rf, float p) -> unique_ptr`.
  - Rust (safe): `TupleSummary` trait; `TupleSketchBuilder<S>` with `new/lg_k/resize_factor/p/build`; `TupleSketch<S>` with `pub(crate) inner: UniquePtr<sys::TupleGenericSketchShim>`, the eleven `update_*`, `trim`, `reset`, the eight queries.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/tuple_generic_sketch_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi, RawSummaryOps, RustSummary};
use std::any::Any;

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn summary(v: i64) -> RustSummary {
    RustSummary::new(Box::new(Sum(v)))
}

#[test]
fn construct_update_estimate() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for i in 0..1000u64 {
        sketch.pin_mut().update_u64(i, &summary(1));
    }
    assert!((sketch.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(sketch.get_num_retained(), 1000);
    assert!(!sketch.is_empty());
}

#[test]
fn repeated_key_combines_rather_than_inserting() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for _ in 0..5 {
        sketch.pin_mut().update_u64(42, &summary(10));
    }
    assert_eq!(sketch.get_num_retained(), 1, "same key must not insert twice");
}

#[test]
fn invalid_lg_k_returns_err() {
    assert!(ffi::new_tuple_generic_sketch(4, 8, 1.0).is_err());
}

#[test]
fn invalid_resize_factor_returns_err() {
    assert!(ffi::new_tuple_generic_sketch(12, 3, 1.0).is_err());
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    sketch.pin_mut().update_u64(1, &summary(1));
    assert!(!sketch.is_empty());
    sketch.pin_mut().reset();
    assert!(sketch.is_empty());
    assert_eq!(sketch.get_num_retained(), 0);
}
```

Register in `apache-datasketches-sys/Cargo.toml`:

```toml
[[test]]
name = "tuple_generic_sketch_link_test"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_sketch_link_test`
Expected: FAIL — `cannot find function new_tuple_generic_sketch in module ffi`.

- [ ] **Step 3: Write the sketch shim header**

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_sketch_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include "rust/cxx.h"
#include "dyn_summary.h"

namespace apache_datasketches_rs {

// The resize factor crosses as its literal multiplier (1, 2, 4, or 8) rather
// than as a shared cxx enum, so this bridge needs no cross-bridge type
// sharing. Throws std::invalid_argument on any other value, which cxx turns
// into Result::Err.
datasketches::resize_factor tuple_generic_resize_factor(uint8_t rf);

class TupleGenericSketchShim {
public:
  TupleGenericSketchShim(uint8_t lg_k, uint8_t rf, float p);

  void update_u64(uint64_t key, const RustSummary& value);
  void update_i64(int64_t key, const RustSummary& value);
  void update_u32(uint32_t key, const RustSummary& value);
  void update_i32(int32_t key, const RustSummary& value);
  void update_u16(uint16_t key, const RustSummary& value);
  void update_i16(int16_t key, const RustSummary& value);
  void update_u8(uint8_t key, const RustSummary& value);
  void update_i8(int8_t key, const RustSummary& value);
  void update_f64(double key, const RustSummary& value);
  void update_str(rust::Str key, const RustSummary& value);
  void update_bytes(rust::Slice<const uint8_t> key, const RustSummary& value);

  void trim();
  void reset();

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  const dyn_update_sketch& inner() const { return sketch_; }

private:
  dyn_update_sketch sketch_;
};

std::unique_ptr<TupleGenericSketchShim> new_tuple_generic_sketch(uint8_t lg_k, uint8_t rf, float p);

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the sketch shim implementation**

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_sketch_shim.cc`:

```cpp
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic.rs.h"

namespace apache_datasketches_rs {

datasketches::resize_factor tuple_generic_resize_factor(uint8_t rf) {
  switch (rf) {
    case 1: return datasketches::resize_factor::X1;
    case 2: return datasketches::resize_factor::X2;
    case 4: return datasketches::resize_factor::X4;
    case 8: return datasketches::resize_factor::X8;
    default: throw std::invalid_argument("resize factor must be 1, 2, 4 or 8");
  }
}

namespace {

dyn_update_sketch build_sketch(uint8_t lg_k, uint8_t rf, float p) {
  // Brace-init, not parentheses: `builder b(Policy(x))` is a function
  // declaration under the most vexing parse. The ArrayOfDoubles shim hit
  // exactly this.
  dyn_update_sketch::builder builder{DynUpdatePolicy()};
  builder.set_lg_k(lg_k);
  builder.set_resize_factor(tuple_generic_resize_factor(rf));
  builder.set_p(p);
  return builder.build();
}

// The sketch's Update type is DynSummary, so every update wraps the borrowed
// RustSummary in a DynSummary that clones it only if the policy actually
// inserts. Constructing this wrapper allocates nothing.
DynSummary borrow_as_update(const RustSummary& value) {
  DynSummary wrapper;
  wrapper.assign_clone_of(value);
  return wrapper;
}

} // namespace

TupleGenericSketchShim::TupleGenericSketchShim(uint8_t lg_k, uint8_t rf, float p)
  : sketch_(build_sketch(lg_k, rf, p)) {}

void TupleGenericSketchShim::update_u64(uint64_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_i64(int64_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_u32(uint32_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_i32(int32_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_u16(uint16_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_i16(int16_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_u8(uint8_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_i8(int8_t key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_f64(double key, const RustSummary& value) {
  sketch_.update(key, borrow_as_update(value));
}
void TupleGenericSketchShim::update_str(rust::Str key, const RustSummary& value) {
  sketch_.update(std::string(key), borrow_as_update(value));
}
void TupleGenericSketchShim::update_bytes(rust::Slice<const uint8_t> key, const RustSummary& value) {
  sketch_.update(key.data(), key.size(), borrow_as_update(value));
}

void TupleGenericSketchShim::trim() { sketch_.trim(); }
void TupleGenericSketchShim::reset() { sketch_.reset(); }

double TupleGenericSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double TupleGenericSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double TupleGenericSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool TupleGenericSketchShim::is_empty() const { return sketch_.is_empty(); }
bool TupleGenericSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool TupleGenericSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double TupleGenericSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t TupleGenericSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

std::unique_ptr<TupleGenericSketchShim> new_tuple_generic_sketch(uint8_t lg_k, uint8_t rf, float p) {
  return std::make_unique<TupleGenericSketchShim>(lg_k, rf, p);
}

} // namespace apache_datasketches_rs
```

**Note on `borrow_as_update`:** it clones eagerly, which costs one allocation per `update()` call even when the key already exists. That is a deliberate simplification for correctness first — the spec's "allocation only on insert" claim describes the intended end state. If profiling shows it matters, the fix is a `DynSummary` variant holding a non-owning `const RustSummary*` for the update-value slot; do not attempt that in this task. Record it as a known cost in the task report.

- [ ] **Step 5: Add the `extern "C++"` block to the bridge**

In `apache-datasketches-sys/src/tuple_generic.rs`, after the `extern "Rust"` block and inside the same `ffi` module, add:

```rust
    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");

        type TupleGenericSketchShim;

        fn new_tuple_generic_sketch(lg_k: u8, rf: u8, p: f32) -> Result<UniquePtr<TupleGenericSketchShim>>;

        fn update_u64(self: Pin<&mut TupleGenericSketchShim>, key: u64, value: &RustSummary);
        fn update_i64(self: Pin<&mut TupleGenericSketchShim>, key: i64, value: &RustSummary);
        fn update_u32(self: Pin<&mut TupleGenericSketchShim>, key: u32, value: &RustSummary);
        fn update_i32(self: Pin<&mut TupleGenericSketchShim>, key: i32, value: &RustSummary);
        fn update_u16(self: Pin<&mut TupleGenericSketchShim>, key: u16, value: &RustSummary);
        fn update_i16(self: Pin<&mut TupleGenericSketchShim>, key: i16, value: &RustSummary);
        fn update_u8(self: Pin<&mut TupleGenericSketchShim>, key: u8, value: &RustSummary);
        fn update_i8(self: Pin<&mut TupleGenericSketchShim>, key: i8, value: &RustSummary);
        fn update_f64(self: Pin<&mut TupleGenericSketchShim>, key: f64, value: &RustSummary);
        fn update_str(self: Pin<&mut TupleGenericSketchShim>, key: &str, value: &RustSummary);
        fn update_bytes(self: Pin<&mut TupleGenericSketchShim>, key: &[u8], value: &RustSummary);

        fn trim(self: Pin<&mut TupleGenericSketchShim>);
        fn reset(self: Pin<&mut TupleGenericSketchShim>);

        fn get_estimate(self: &TupleGenericSketchShim) -> f64;
        fn get_lower_bound(self: &TupleGenericSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &TupleGenericSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &TupleGenericSketchShim) -> bool;
        fn is_estimation_mode(self: &TupleGenericSketchShim) -> bool;
        fn is_ordered(self: &TupleGenericSketchShim) -> bool;
        fn get_theta(self: &TupleGenericSketchShim) -> f64;
        fn get_num_retained(self: &TupleGenericSketchShim) -> u32;
    }
```

Add `"cpp/tuple/tuple_generic_sketch_shim.cc"` to the `tuple` `.file()` list in `build.rs` plus its two `rerun-if-changed` lines.

- [ ] **Step 6: Run the link test to verify it passes**

Run: `cargo test -p apache-datasketches-sys --features tuple`
Expected: PASS — `tuple_generic_summary_link_test` (3) and `tuple_generic_sketch_link_test` (5).

- [ ] **Step 7: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_generic_sketch_smoke_test.rs`:

```rust
use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};

/// Sums on union, takes the minimum on intersection — deliberately different
/// so a cross-wired trampoline is detectable.
#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

#[test]
fn construct_update_estimate() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    for i in 0..1000u64 {
        sketch.update_u64(i, &1);
    }
    assert!((sketch.get_estimate() - 1000.0).abs() < 1.0);
    assert_eq!(sketch.get_num_retained(), 1000);
}

#[test]
fn repeated_key_unions_its_summaries() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for _ in 0..4 {
        sketch.update_u64(7, &10);
    }
    assert_eq!(sketch.get_num_retained(), 1);
}

#[test]
fn invalid_config_is_err() {
    assert!(TupleSketchBuilder::<Sum>::new().lg_k(4).build().is_err());
    assert!(TupleSketchBuilder::<Sum>::new().p(0.0).build().is_err());
    assert!(TupleSketchBuilder::<Sum>::new().p(1.5).build().is_err());
}

#[test]
fn reset_empties_the_sketch() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    sketch.update_u64(1, &1);
    assert!(!sketch.is_empty());
    sketch.reset();
    assert!(sketch.is_empty());
}

#[test]
fn every_update_key_type_works() {
    let mut sketch: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    sketch.update_u64(1, &1);
    sketch.update_i64(2, &1);
    sketch.update_u32(3, &1);
    sketch.update_i32(4, &1);
    sketch.update_u16(5, &1);
    sketch.update_i16(6, &1);
    sketch.update_u8(7, &1);
    sketch.update_i8(8, &1);
    sketch.update_f64(9.0, &1);
    sketch.update_str("ten", &1);
    sketch.update_bytes(b"eleven", &1);
    assert_eq!(sketch.get_num_retained(), 11);
}

#[test]
fn sketch_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleSketch<Sum>>();
}
```

Register in `apache-datasketches/Cargo.toml`:

```toml
[[test]]
name = "tuple_generic_sketch_smoke_test"
required-features = ["tuple"]
```

- [ ] **Step 8: Run the smoke test to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_sketch_smoke_test`
Expected: FAIL — `could not find generic in apache_datasketches::tuple`.

- [ ] **Step 9: Write the trait and adapter**

Create `apache-datasketches/src/tuple/generic/summary.rs`:

```rust
use apache_datasketches_sys::tuple_generic::{RawSummaryOps, RustSummary};
use std::any::Any;

/// A user-defined per-entry summary for [`TupleSketch`](super::TupleSketch).
///
/// Implement this on your own type to get a Tuple sketch that carries it.
///
/// # Panics and the FFI boundary
///
/// [`union_combine`](Self::union_combine),
/// [`intersection_combine`](Self::intersection_combine), and
/// [`Clone::clone`] are invoked by C++. A panic cannot unwind across that
/// boundary, and the underlying C++ combine has no way to report failure and
/// roll back an insert, so a panic in any of those three **aborts the
/// process** after printing a diagnostic. Make them total.
///
/// [`create`](Self::create) is different: it runs entirely in Rust before any
/// C++ call, so a panic there is an ordinary Rust panic that propagates to
/// your caller.
pub trait TupleSummary: Clone + Send + 'static {
    /// The value passed to the sketch's `update_*` methods.
    ///
    /// May be unsized — `type Update = str` and `type Update = [f64]` both
    /// work, so callers need not allocate to update.
    type Update: ?Sized;

    /// Builds a summary from a single update value.
    fn create(update: &Self::Update) -> Self;

    /// Merges `other` into `self` with union semantics. Used both when a key
    /// is updated more than once and when two sketches are unioned.
    fn union_combine(&mut self, other: &Self);

    /// Merges `other` into `self` with intersection semantics.
    ///
    /// There is deliberately no default: upstream notes that no intersection
    /// policy is sensible in general, and silently reusing union semantics
    /// would be a correctness trap. If union semantics *are* what you want,
    /// call `self.union_combine(other)` here explicitly.
    fn intersection_combine(&mut self, other: &Self);
}

/// Erases a `TupleSummary` to the sys crate's `RawSummaryOps`.
///
/// Private: users never name this. It exists because cxx requires the opaque
/// `extern "Rust"` type to live in the crate that declares the bridge, so the
/// ergonomic trait here has to be adapted to the minimal trait there.
pub(crate) struct Adapter<S: TupleSummary> {
    pub(crate) value: S,
}

impl<S: TupleSummary> Adapter<S> {
    pub(crate) fn new(value: S) -> Self {
        Self { value }
    }

    /// Recovers `&S` from an erased operand.
    ///
    /// The typed façade makes a mismatch unreachable — a `TupleUnion<S>`
    /// accepts only `TupleSketch<S>` — so a failure here means an internal
    /// invariant broke, not user error.
    fn downcast(other: &dyn RawSummaryOps) -> &S {
        match other.as_any().downcast_ref::<Adapter<S>>() {
            Some(adapter) => &adapter.value,
            None => panic!(
                "apache-datasketches internal invariant violated: a generic Tuple \
                 summary of a different concrete type reached a combine callback. \
                 This should be impossible through the public API; please report it."
            ),
        }
    }
}

impl<S: TupleSummary> RawSummaryOps for Adapter<S> {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Adapter::new(self.value.clone()))
    }

    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        let other = Self::downcast(other);
        self.value.union_combine(other);
    }

    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        let other = Self::downcast(other);
        self.value.intersection_combine(other);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Wraps a user summary in the opaque type that crosses the FFI boundary.
pub(crate) fn erase<S: TupleSummary>(value: S) -> RustSummary {
    RustSummary::new(Box::new(Adapter::new(value)))
}
```

- [ ] **Step 10: Write the builder and the sketch**

Create `apache-datasketches/src/tuple/generic/builder.rs`:

```rust
use super::{summary::TupleSummary, TupleSketch};
use crate::error::SketchError;
use crate::tuple::ResizeFactor;
use std::marker::PhantomData;

/// Builder for [`TupleSketch`], mirroring upstream's
/// `update_tuple_sketch::builder`. `lg_k` defaults to `12`, `resize_factor`
/// to [`ResizeFactor::X8`], and `p` to `1.0` (no sampling). The seed is never
/// exposed.
#[derive(Debug, Clone, Copy)]
pub struct TupleSketchBuilder<S: TupleSummary> {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
    _marker: PhantomData<fn() -> S>,
}

impl<S: TupleSummary> Default for TupleSketchBuilder<S> {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::X8,
            p: 1.0,
            _marker: PhantomData,
        }
    }
}

impl<S: TupleSummary> TupleSketchBuilder<S> {
    /// Creates a builder with default settings (`lg_k = 12`,
    /// `resize_factor = X8`, `p = 1.0`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the target number of retained entries.
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Sets the hash table's growth [`ResizeFactor`].
    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    /// Sets the sampling probability. `1.0` (the default) disables sampling.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Builds the sketch. Returns [`SketchError::InvalidConfig`] if `lg_k` is
    /// out of range or `p` is outside `(0, 1]`.
    pub fn build(self) -> Result<TupleSketch<S>, SketchError> {
        TupleSketch::from_parts(self.lg_k, self.resize_factor, self.p)
    }
}

/// Converts the safe enum to the literal multiplier this bridge passes as a
/// `u8`. See the task note on why this bridge does not share a cxx enum.
pub(crate) fn resize_factor_multiplier(rf: ResizeFactor) -> u8 {
    match rf {
        ResizeFactor::X1 => 1,
        ResizeFactor::X2 => 2,
        ResizeFactor::X4 => 4,
        ResizeFactor::X8 => 8,
    }
}
```

Create `apache-datasketches/src/tuple/generic/sketch.rs`:

```rust
use super::builder::resize_factor_multiplier;
use super::summary::{erase, TupleSummary};
use crate::error::SketchError;
use crate::tuple::ResizeFactor;
use apache_datasketches_sys::tuple_generic::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// A mutable, update-only Tuple sketch carrying a user-defined summary `S`
/// per distinct key. Build one with
/// [`TupleSketchBuilder`](super::TupleSketchBuilder).
pub struct TupleSketch<S: TupleSummary> {
    pub(crate) inner: UniquePtr<sys::TupleGenericSketchShim>,
    pub(crate) _marker: PhantomData<fn() -> S>,
}

// Sound because `S: Send` is a supertrait of `TupleSummary` and the sys-crate
// box is `Box<dyn RawSummaryOps + Send>`. Deliberately not `Sync`.
unsafe impl<S: TupleSummary> Send for TupleSketch<S> {}

impl<S: TupleSummary> TupleSketch<S> {
    pub(crate) fn from_parts(lg_k: u8, rf: ResizeFactor, p: f32) -> Result<Self, SketchError> {
        let inner = sys::new_tuple_generic_sketch(lg_k, resize_factor_multiplier(rf), p)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(Self {
            inner,
            _marker: PhantomData,
        })
    }

    /// Adds a `u64` key. `S::create` runs first, entirely in Rust; the
    /// resulting summary is combined into an existing entry or cloned into a
    /// new one.
    pub fn update_u64(&mut self, key: u64, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_u64(key, &summary);
    }

    /// Adds an `i64` key. See [`Self::update_u64`].
    pub fn update_i64(&mut self, key: i64, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_i64(key, &summary);
    }

    /// Adds a `u32` key. See [`Self::update_u64`].
    pub fn update_u32(&mut self, key: u32, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_u32(key, &summary);
    }

    /// Adds an `i32` key. See [`Self::update_u64`].
    pub fn update_i32(&mut self, key: i32, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_i32(key, &summary);
    }

    /// Adds a `u16` key. See [`Self::update_u64`].
    pub fn update_u16(&mut self, key: u16, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_u16(key, &summary);
    }

    /// Adds an `i16` key. See [`Self::update_u64`].
    pub fn update_i16(&mut self, key: i16, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_i16(key, &summary);
    }

    /// Adds a `u8` key. See [`Self::update_u64`].
    pub fn update_u8(&mut self, key: u8, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_u8(key, &summary);
    }

    /// Adds an `i8` key. See [`Self::update_u64`].
    pub fn update_i8(&mut self, key: i8, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_i8(key, &summary);
    }

    /// Adds an `f64` key. See [`Self::update_u64`].
    pub fn update_f64(&mut self, key: f64, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_f64(key, &summary);
    }

    /// Adds a string key. See [`Self::update_u64`].
    pub fn update_str(&mut self, key: &str, value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_str(key, &summary);
    }

    /// Adds an arbitrary byte-slice key. See [`Self::update_u64`].
    pub fn update_bytes(&mut self, key: &[u8], value: &S::Update) {
        let summary = erase(S::create(value));
        self.inner.pin_mut().update_bytes(key, &summary);
    }

    /// Removes retained entries in excess of the nominal size `k`, lowering
    /// theta to do so. Note this shifts [`Self::get_estimate`].
    pub fn trim(&mut self) {
        self.inner.pin_mut().trim();
    }

    /// Resets this sketch to its initial, empty state.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns the current estimate of the number of distinct keys added.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`], for `num_std_dev` of `1`, `2`, or `3`.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See [`Self::get_lower_bound`].
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if no keys have been added.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the sketch has begun sampling.
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if retained entries are sorted by hash value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold.
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of retained entries.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }
}
```

Create `apache-datasketches/src/tuple/generic/mod.rs`:

```rust
//! Generic Tuple sketches: cardinality estimation where each distinct key
//! carries a summary of a type you define.
//!
//! Implement [`TupleSummary`] on your own type and it becomes usable as a
//! sketch summary. C++ calls back into Rust to clone and combine summaries;
//! see [`TupleSummary`]'s documentation for which methods must not panic.
//!
//! For the common case of a fixed-width array of `f64` per key, prefer
//! [`ArrayOfDoublesSketch`](crate::tuple::ArrayOfDoublesSketch) — it binds a
//! concrete C++ instantiation with no callback overhead.
//!
//! ```
//! # fn main() -> Result<(), apache_datasketches::SketchError> {
//! use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};
//!
//! #[derive(Clone)]
//! struct Count(u64);
//!
//! impl TupleSummary for Count {
//!     type Update = ();
//!     fn create(_: &()) -> Self { Count(1) }
//!     fn union_combine(&mut self, other: &Self) { self.0 += other.0; }
//!     fn intersection_combine(&mut self, other: &Self) { self.0 += other.0; }
//! }
//!
//! let mut sketch: TupleSketch<Count> = TupleSketchBuilder::new().build()?;
//! sketch.update_u64(42, &());
//! println!("estimate: {}", sketch.get_estimate());
//! # Ok(())
//! # }
//! ```

mod builder;
mod sketch;
mod summary;

pub use builder::TupleSketchBuilder;
pub use sketch::TupleSketch;
pub use summary::TupleSummary;
```

In `apache-datasketches/src/tuple/mod.rs`, add to the module list and re-export block:

```rust
pub mod generic;
```

placed after the existing `mod union;` line, and add this bullet to the module doc:

```rust
//! - [`generic`] — Tuple sketches over a summary type you define yourself,
//!   for cases the fixed `f64`-array shape above does not cover.
```

- [ ] **Step 11: Run the smoke test to verify it passes**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS, including `tuple_generic_sketch_smoke_test` (6 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

- [ ] **Step 12: Commit**

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add TupleSummary, the erasure adapter, and TupleSketch<S>"
```

---

### Task 3: `CompactTupleSketch<S>`, `compact()`, and `entries()`

The immutable snapshot and per-entry read access. This is the second and last component whose signatures mention `RustSummary`, so its `extern "C++"` block also goes in `src/tuple_generic.rs`.

Entries come back one at a time — `entry_count()`, `entry_hash(i)`, `entry_summary(i) -> Box<RustSummary>` — rather than as two bulk vectors the way ArrayOfDoubles does it. A bulk `Vec<Box<RustSummary>>` is not expressible across cxx, and per-entry access keeps each summary owned and typed.

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/tuple_generic_compact_shim.h` and `.cc`
- Modify: `apache-datasketches-sys/src/tuple_generic.rs` (second `extern "C++"` block), `cpp/tuple/tuple_generic_sketch_shim.h`/`.cc` (add `compact`)
- Create (test): `apache-datasketches-sys/tests/tuple_generic_compact_link_test.rs`
- Create: `apache-datasketches/src/tuple/generic/compact.rs`
- Modify: `apache-datasketches/src/tuple/generic/{mod.rs,sketch.rs,summary.rs}`
- Create (test): `apache-datasketches/tests/tuple_generic_compact_smoke_test.rs`

**Interfaces:**
- Consumes: `TupleGenericSketchShim::inner()`, `dyn_compact_sketch`, `erase`/`Adapter` (Tasks 1–2).
- Produces:
  - C++: `CompactTupleGenericSketchShim` with `inner()`; free function `tuple_generic_sketch_compact(const TupleGenericSketchShim&, bool ordered)`.
  - Rust (safe): `CompactTupleSketch<S>` with `from_shim`, the eight queries, and `entries() -> impl Iterator<Item = (u64, S)>`; `TupleSketch::<S>::compact(&self, ordered: bool) -> CompactTupleSketch<S>`; `summary::unerase::<S>(&RustSummary) -> S`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/tuple_generic_compact_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi, RawSummaryOps, RustSummary};
use std::any::Any;

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn summary(v: i64) -> RustSummary {
    RustSummary::new(Box::new(Sum(v)))
}

fn value_of(s: &RustSummary) -> i64 {
    s.ops().as_any().downcast_ref::<Sum>().unwrap().0
}

#[test]
fn compact_preserves_estimate_and_entries() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for i in 0..500u64 {
        sketch.pin_mut().update_u64(i, &summary(3));
    }
    let compact = ffi::tuple_generic_sketch_compact(&sketch, true);
    assert!((compact.get_estimate() - 500.0).abs() < 1.0);
    assert_eq!(compact.entry_count(), 500);
    assert!(compact.is_ordered());

    // Every entry carries the summary that was inserted.
    for i in 0..compact.entry_count() {
        assert_eq!(value_of(&compact.entry_summary(i)), 3);
    }
}

#[test]
fn ordered_entry_hashes_are_sorted() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for i in 0..200u64 {
        sketch.pin_mut().update_u64(i, &summary(1));
    }
    let compact = ffi::tuple_generic_sketch_compact(&sketch, true);
    let hashes: Vec<u64> = (0..compact.entry_count()).map(|i| compact.entry_hash(i)).collect();
    let mut sorted = hashes.clone();
    sorted.sort_unstable();
    assert_eq!(hashes, sorted);
}

#[test]
fn repeated_updates_are_unioned_in_the_compact_form() {
    let mut sketch = ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for _ in 0..4 {
        sketch.pin_mut().update_u64(9, &summary(5));
    }
    let compact = ffi::tuple_generic_sketch_compact(&sketch, true);
    assert_eq!(compact.entry_count(), 1);
    assert_eq!(value_of(&compact.entry_summary(0)), 20, "4 x 5 summed");
}
```

Register it in `apache-datasketches-sys/Cargo.toml` as `tuple_generic_compact_link_test` with `required-features = ["tuple"]`.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_compact_link_test`
Expected: FAIL — `cannot find function tuple_generic_sketch_compact in module ffi`.

- [ ] **Step 3: Write the compact shim**

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_compact_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include <vector>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_generic_sketch_shim.h"

namespace apache_datasketches_rs {

class CompactTupleGenericSketchShim {
public:
  explicit CompactTupleGenericSketchShim(dyn_compact_sketch sketch);

  double get_estimate() const;
  double get_lower_bound(uint8_t num_std_dev) const;
  double get_upper_bound(uint8_t num_std_dev) const;
  bool is_empty() const;
  bool is_estimation_mode() const;
  bool is_ordered() const;
  double get_theta() const;
  uint32_t get_num_retained() const;

  // Per-entry access. entry_summary clones, because the caller owns the
  // result and the sketch keeps its own copy.
  uint32_t entry_count() const;
  uint64_t entry_hash(uint32_t index) const;
  rust::Box<RustSummary> entry_summary(uint32_t index) const;

  const dyn_compact_sketch& inner() const { return sketch_; }

private:
  // Materialised once so entry_hash/entry_summary are O(1) rather than
  // walking the sketch's iterator on every call.
  const std::vector<dyn_compact_sketch::Entry>& entries() const;

  dyn_compact_sketch sketch_;
  mutable std::vector<dyn_compact_sketch::Entry> entries_;
  mutable bool entries_built_ = false;
};

std::unique_ptr<CompactTupleGenericSketchShim> tuple_generic_sketch_compact(
    const TupleGenericSketchShim& sketch, bool ordered);

} // namespace apache_datasketches_rs
```

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_compact_shim.cc`:

```cpp
#include "tuple_generic_compact_shim.h"
#include "tuple_generic.rs.h"

namespace apache_datasketches_rs {

CompactTupleGenericSketchShim::CompactTupleGenericSketchShim(dyn_compact_sketch sketch)
  : sketch_(std::move(sketch)) {}

double CompactTupleGenericSketchShim::get_estimate() const { return sketch_.get_estimate(); }
double CompactTupleGenericSketchShim::get_lower_bound(uint8_t num_std_dev) const {
  return sketch_.get_lower_bound(num_std_dev);
}
double CompactTupleGenericSketchShim::get_upper_bound(uint8_t num_std_dev) const {
  return sketch_.get_upper_bound(num_std_dev);
}
bool CompactTupleGenericSketchShim::is_empty() const { return sketch_.is_empty(); }
bool CompactTupleGenericSketchShim::is_estimation_mode() const { return sketch_.is_estimation_mode(); }
bool CompactTupleGenericSketchShim::is_ordered() const { return sketch_.is_ordered(); }
double CompactTupleGenericSketchShim::get_theta() const { return sketch_.get_theta(); }
uint32_t CompactTupleGenericSketchShim::get_num_retained() const { return sketch_.get_num_retained(); }

const std::vector<dyn_compact_sketch::Entry>& CompactTupleGenericSketchShim::entries() const {
  if (!entries_built_) {
    entries_.reserve(sketch_.get_num_retained());
    for (const auto& entry : sketch_) entries_.push_back(entry);
    entries_built_ = true;
  }
  return entries_;
}

uint32_t CompactTupleGenericSketchShim::entry_count() const {
  return static_cast<uint32_t>(entries().size());
}

uint64_t CompactTupleGenericSketchShim::entry_hash(uint32_t index) const {
  return entries().at(index).first;
}

rust::Box<RustSummary> CompactTupleGenericSketchShim::entry_summary(uint32_t index) const {
  return rust_summary_clone(entries().at(index).second.get());
}

std::unique_ptr<CompactTupleGenericSketchShim> tuple_generic_sketch_compact(
    const TupleGenericSketchShim& sketch, bool ordered) {
  return std::make_unique<CompactTupleGenericSketchShim>(sketch.inner().compact(ordered));
}

} // namespace apache_datasketches_rs
```

`entries().at(index)` throws `std::out_of_range` on a bad index, which cxx converts to `Result::Err`; the bridge declares those two accessors as returning `Result`.

- [ ] **Step 4: Add `compact()` to the sketch shim and the bridge block**

In `tuple_generic_sketch_shim.h`, add a forward declaration `class CompactTupleGenericSketchShim;` above the class and this member:

```cpp
  std::unique_ptr<CompactTupleGenericSketchShim> compact(bool ordered) const;
```

In `tuple_generic_sketch_shim.cc`, add `#include "tuple_generic_compact_shim.h"` and, before the closing namespace brace:

```cpp
std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericSketchShim::compact(bool ordered) const {
  return tuple_generic_sketch_compact(*this, ordered);
}
```

In `src/tuple_generic.rs`, extend the existing `unsafe extern "C++"` block with:

```rust
        include!("tuple_generic_compact_shim.h");

        type CompactTupleGenericSketchShim;

        fn tuple_generic_sketch_compact(sketch: &TupleGenericSketchShim, ordered: bool) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn compact(self: &TupleGenericSketchShim, ordered: bool) -> UniquePtr<CompactTupleGenericSketchShim>;

        fn get_estimate(self: &CompactTupleGenericSketchShim) -> f64;
        fn get_lower_bound(self: &CompactTupleGenericSketchShim, num_std_dev: u8) -> Result<f64>;
        fn get_upper_bound(self: &CompactTupleGenericSketchShim, num_std_dev: u8) -> Result<f64>;
        fn is_empty(self: &CompactTupleGenericSketchShim) -> bool;
        fn is_estimation_mode(self: &CompactTupleGenericSketchShim) -> bool;
        fn is_ordered(self: &CompactTupleGenericSketchShim) -> bool;
        fn get_theta(self: &CompactTupleGenericSketchShim) -> f64;
        fn get_num_retained(self: &CompactTupleGenericSketchShim) -> u32;

        fn entry_count(self: &CompactTupleGenericSketchShim) -> u32;
        fn entry_hash(self: &CompactTupleGenericSketchShim, index: u32) -> Result<u64>;
        fn entry_summary(self: &CompactTupleGenericSketchShim, index: u32) -> Result<Box<RustSummary>>;
```

Add the new `.cc` to `build.rs`'s `tuple` file list and its `rerun-if-changed` lines.

- [ ] **Step 5: Run the link tests**

Run: `cargo test -p apache-datasketches-sys --features tuple`
Expected: PASS — all three generic link-test binaries.

- [ ] **Step 6: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_generic_compact_smoke_test.rs`:

```rust
use apache_datasketches::tuple::generic::{
    CompactTupleSketch, TupleSketch, TupleSketchBuilder, TupleSummary,
};

#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sketch(keys: std::ops::Range<u64>, value: i64) -> TupleSketch<Sum> {
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &value);
    }
    s
}

#[test]
fn compact_preserves_estimate_and_summaries() {
    let compact = sketch(0..500, 3).compact(true);
    assert!((compact.get_estimate() - 500.0).abs() < 1.0);
    assert_eq!(compact.get_num_retained(), 500);
    assert!(compact.is_ordered());
    assert!(compact.entries().all(|(_, s)| s == Sum(3)));
}

#[test]
fn entries_are_hash_ordered_when_compacted_ordered() {
    let compact = sketch(0..200, 1).compact(true);
    let hashes: Vec<u64> = compact.entries().map(|(h, _)| h).collect();
    assert_eq!(hashes.len(), 200);
    let mut sorted = hashes.clone();
    sorted.sort_unstable();
    assert_eq!(hashes, sorted);
}

#[test]
fn unordered_compaction_reports_itself_unordered() {
    let compact = sketch(0..50, 1).compact(false);
    assert!(!compact.is_ordered());
    assert_eq!(compact.entries().count(), 50);
}

#[test]
fn empty_sketch_compacts_to_empty() {
    let s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    let compact = s.compact(true);
    assert!(compact.is_empty());
    assert_eq!(compact.entries().count(), 0);
}

#[test]
fn compact_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<CompactTupleSketch<Sum>>();
}
```

Register as `tuple_generic_compact_smoke_test` with `required-features = ["tuple"]`.

- [ ] **Step 7: Run to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_compact_smoke_test`
Expected: FAIL — `no CompactTupleSketch in apache_datasketches::tuple::generic`.

- [ ] **Step 8: Write the safe compact wrapper**

Add to `apache-datasketches/src/tuple/generic/summary.rs`:

```rust
/// Recovers an owned `S` from a summary that crossed back from C++.
///
/// Unreachable failure for the same reason as [`Adapter::downcast`]: the
/// typed façade guarantees the concrete type.
pub(crate) fn unerase<S: TupleSummary>(summary: &RustSummary) -> S {
    match summary.ops().as_any().downcast_ref::<Adapter<S>>() {
        Some(adapter) => adapter.value.clone(),
        None => panic!(
            "apache-datasketches internal invariant violated: a generic Tuple summary \
             of a different concrete type was returned from the sketch. This should be \
             impossible through the public API; please report it."
        ),
    }
}
```

Create `apache-datasketches/src/tuple/generic/compact.rs`:

```rust
use super::summary::{unerase, TupleSummary};
use crate::error::SketchError;
use apache_datasketches_sys::tuple_generic::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// An immutable snapshot of a generic Tuple sketch, produced by
/// [`TupleSketch::compact`](super::TupleSketch::compact) or by any set
/// operation's result.
///
/// Serialization is not part of this version; it is the subject of a
/// follow-up design.
pub struct CompactTupleSketch<S: TupleSummary> {
    pub(crate) inner: UniquePtr<sys::CompactTupleGenericSketchShim>,
    pub(crate) _marker: PhantomData<fn() -> S>,
}

unsafe impl<S: TupleSummary> Send for CompactTupleSketch<S> {}

impl<S: TupleSummary> CompactTupleSketch<S> {
    pub(crate) fn from_shim(inner: UniquePtr<sys::CompactTupleGenericSketchShim>) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }

    /// Returns the current estimate of the number of distinct keys.
    pub fn get_estimate(&self) -> f64 {
        self.inner.get_estimate()
    }

    /// Returns the lower bound of the confidence interval around
    /// [`Self::get_estimate`], for `num_std_dev` of `1`, `2`, or `3`.
    pub fn get_lower_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_lower_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns the upper bound of the confidence interval around
    /// [`Self::get_estimate`]. See [`Self::get_lower_bound`].
    pub fn get_upper_bound(&self, num_std_dev: u8) -> Result<f64, SketchError> {
        self.inner
            .get_upper_bound(num_std_dev)
            .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if this sketch represents an empty set.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the estimate is a statistical estimate rather than
    /// an exact count.
    pub fn is_estimation_mode(&self) -> bool {
        self.inner.is_estimation_mode()
    }

    /// Returns `true` if retained entries are sorted by hash value.
    pub fn is_ordered(&self) -> bool {
        self.inner.is_ordered()
    }

    /// Returns the current theta threshold.
    pub fn get_theta(&self) -> f64 {
        self.inner.get_theta()
    }

    /// Returns the number of retained entries.
    pub fn get_num_retained(&self) -> u32 {
        self.inner.get_num_retained()
    }

    /// Iterates the retained entries as `(hash, summary)` pairs.
    ///
    /// Each summary is cloned out of C++, so the items are owned. Ordered by
    /// hash if [`Self::is_ordered`] is `true`.
    pub fn entries(&self) -> impl Iterator<Item = (u64, S)> + '_ {
        (0..self.inner.entry_count()).map(move |i| {
            let hash = self
                .inner
                .entry_hash(i)
                .expect("index derived from entry_count is always in range");
            let summary = self
                .inner
                .entry_summary(i)
                .expect("index derived from entry_count is always in range");
            (hash, unerase::<S>(&summary))
        })
    }
}
```

Add `compact` to `apache-datasketches/src/tuple/generic/sketch.rs`:

```rust
    /// Produces an immutable [`CompactTupleSketch`] snapshot. If `ordered` is
    /// `true`, its entries are sorted by hash value.
    pub fn compact(&self, ordered: bool) -> super::CompactTupleSketch<S> {
        super::CompactTupleSketch::from_shim(self.inner.compact(ordered))
    }
```

Update `apache-datasketches/src/tuple/generic/mod.rs`:

```rust
mod builder;
mod compact;
mod sketch;
mod summary;

pub use builder::TupleSketchBuilder;
pub use compact::CompactTupleSketch;
pub use sketch::TupleSketch;
pub use summary::TupleSummary;
```

- [ ] **Step 9: Run the tests**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS, including `tuple_generic_compact_smoke_test` (5 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

- [ ] **Step 10: Commit**

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add CompactTupleSketch<S> with per-entry summary access"
```

---

### Task 4: Input dispatch and `TupleUnion<S>`

From here on, the bridges never mention `RustSummary` — they pass shim types only — so each gets its own bridge file and aliases the shim types the ordinary `extern "C++"` way.

**Files:**
- Create: `apache-datasketches-sys/cpp/tuple/tuple_generic_union_shim.h` / `.cc`
- Create: `apache-datasketches-sys/src/tuple_generic_union.rs`
- Create: `apache-datasketches-sys/src/tuple_generic_input.rs`
- Create (test): `apache-datasketches-sys/tests/tuple_generic_union_link_test.rs`
- Create: `apache-datasketches/src/tuple/generic/input.rs`, `union.rs`
- Create (test): `apache-datasketches/tests/tuple_generic_union_smoke_test.rs`
- Modify: `apache-datasketches-sys/src/lib.rs`, `apache-datasketches/src/tuple/generic/mod.rs`, both `Cargo.toml`s, `build.rs`

**Interfaces:**
- Consumes: `TupleGenericSketchShim`, `CompactTupleGenericSketchShim`, `dyn_union`, `DynUnionPolicy`, `TupleSummary`, `CompactTupleSketch::from_shim`.
- Produces:
  - Rust (sys): `tuple_generic_input::TupleGenericInputRef<'a>` with `Sketch(&'a TupleGenericSketchShim)` / `Compact(&'a CompactTupleGenericSketchShim)`; `tuple_generic_union::ffi::TupleGenericUnionShim`.
  - Rust (safe): sealed `TupleInput<S>` with hidden `as_input`; `TupleUnionBuilder<S>` (`new`/`lg_k`/`resize_factor`/`p`/`build`); `TupleUnion<S>` with `update(&impl TupleInput<S>)`, `get_result(bool) -> CompactTupleSketch<S>`, `reset()`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/tuple_generic_union_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi as sketch_ffi, RawSummaryOps, RustSummary};
use apache_datasketches_sys::tuple_generic_union::ffi as union_ffi;
use std::any::Any;

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn sketch(
    keys: std::ops::Range<u64>,
    v: i64,
) -> cxx::UniquePtr<sketch_ffi::TupleGenericSketchShim> {
    let mut s = sketch_ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for key in keys {
        s.pin_mut()
            .update_u64(key, &RustSummary::new(Box::new(Sum(v))));
    }
    s
}

fn value_at(c: &sketch_ffi::CompactTupleGenericSketchShim, i: u32) -> i64 {
    c.entry_summary(i)
        .unwrap()
        .ops()
        .as_any()
        .downcast_ref::<Sum>()
        .unwrap()
        .0
}

#[test]
fn union_half_overlap() {
    let a = sketch(0..1000, 1);
    let b = sketch(500..1500, 1);
    let mut u = union_ffi::new_tuple_generic_union(12, 8, 1.0).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_sketch(&b);
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);
}

#[test]
fn union_sums_summaries_on_collision() {
    let a = sketch(7..8, 10);
    let b = sketch(7..8, 32);
    let mut u = union_ffi::new_tuple_generic_union(12, 8, 1.0).unwrap();
    u.pin_mut().update_with_sketch(&a);
    u.pin_mut().update_with_sketch(&b);
    let result = u.get_result(true);
    assert_eq!(result.entry_count(), 1);
    assert_eq!(value_at(&result, 0), 42, "union policy must sum");
}

#[test]
fn union_accepts_compact_input_and_resets() {
    let a = sketch(0..100, 1);
    let compact = a.compact(true);
    let mut u = union_ffi::new_tuple_generic_union(12, 8, 1.0).unwrap();
    u.pin_mut().update_with_compact(&compact);
    assert_eq!(u.get_result(true).get_estimate(), 100.0);
    u.pin_mut().reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn invalid_config_returns_err() {
    assert!(union_ffi::new_tuple_generic_union(4, 8, 1.0).is_err());
    assert!(union_ffi::new_tuple_generic_union(12, 3, 1.0).is_err());
}
```

Register as `tuple_generic_union_link_test`, `required-features = ["tuple"]`.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_union_link_test`
Expected: FAIL — `unresolved import apache_datasketches_sys::tuple_generic_union`.

- [ ] **Step 3: Write the union shim**

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_union_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"

namespace apache_datasketches_rs {

class TupleGenericUnionShim {
public:
  TupleGenericUnionShim(uint8_t lg_k, uint8_t rf, float p);

  void update_with_sketch(const TupleGenericSketchShim& sketch);
  void update_with_compact(const CompactTupleGenericSketchShim& sketch);

  std::unique_ptr<CompactTupleGenericSketchShim> get_result(bool ordered) const;
  void reset();

private:
  dyn_union union_;
};

std::unique_ptr<TupleGenericUnionShim> new_tuple_generic_union(uint8_t lg_k, uint8_t rf, float p);

} // namespace apache_datasketches_rs
```

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_union_shim.cc`:

```cpp
#include "tuple_generic_union_shim.h"

namespace apache_datasketches_rs {

namespace {
dyn_union build_union(uint8_t lg_k, uint8_t rf, float p) {
  // Brace-init: `builder b(Policy())` is a function declaration.
  dyn_union::builder builder{DynUnionPolicy()};
  builder.set_lg_k(lg_k);
  builder.set_resize_factor(tuple_generic_resize_factor(rf));
  builder.set_p(p);
  return builder.build();
}
} // namespace

TupleGenericUnionShim::TupleGenericUnionShim(uint8_t lg_k, uint8_t rf, float p)
  : union_(build_union(lg_k, rf, p)) {}

void TupleGenericUnionShim::update_with_sketch(const TupleGenericSketchShim& sketch) {
  union_.update(sketch.inner());
}

void TupleGenericUnionShim::update_with_compact(const CompactTupleGenericSketchShim& sketch) {
  union_.update(sketch.inner());
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericUnionShim::get_result(bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(union_.get_result(ordered));
}

void TupleGenericUnionShim::reset() { union_.reset(); }

std::unique_ptr<TupleGenericUnionShim> new_tuple_generic_union(uint8_t lg_k, uint8_t rf, float p) {
  return std::make_unique<TupleGenericUnionShim>(lg_k, rf, p);
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the union bridge and the input enum**

Create `apache-datasketches-sys/src/tuple_generic_union.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");
        include!("tuple_generic_compact_shim.h");
        include!("tuple_generic_union_shim.h");

        type TupleGenericSketchShim = crate::tuple_generic::ffi::TupleGenericSketchShim;
        type CompactTupleGenericSketchShim = crate::tuple_generic::ffi::CompactTupleGenericSketchShim;

        type TupleGenericUnionShim;

        fn new_tuple_generic_union(lg_k: u8, rf: u8, p: f32) -> Result<UniquePtr<TupleGenericUnionShim>>;

        fn update_with_sketch(self: Pin<&mut TupleGenericUnionShim>, sketch: &TupleGenericSketchShim);
        fn update_with_compact(self: Pin<&mut TupleGenericUnionShim>, sketch: &CompactTupleGenericSketchShim);

        fn get_result(self: &TupleGenericUnionShim, ordered: bool) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn reset(self: Pin<&mut TupleGenericUnionShim>);
    }
}
```

Create `apache-datasketches-sys/src/tuple_generic_input.rs`:

```rust
use crate::tuple_generic::ffi::{CompactTupleGenericSketchShim, TupleGenericSketchShim};

/// A borrowed reference to one of the two generic Tuple sketch shim types,
/// used to dispatch set-operation calls to the right C++ overload.
///
/// A plain Rust enum rather than a bridge type: cxx supports only
/// payload-free shared enums, and these variants carry references to opaque
/// C++ types.
pub enum TupleGenericInputRef<'a> {
    /// A mutable, update-only sketch.
    Sketch(&'a TupleGenericSketchShim),
    /// An immutable, compact sketch.
    Compact(&'a CompactTupleGenericSketchShim),
}
```

Add both modules to `apache-datasketches-sys/src/lib.rs` under `#[cfg(feature = "tuple")]`, add the union bridge to `build.rs`'s bridge list (**not** `tuple_generic_input.rs` — it is a plain module, not a bridge), add the `.cc` to the file list, and add the `rerun-if-changed` lines.

- [ ] **Step 5: Run the link test**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_union_link_test`
Expected: PASS (4 tests).

- [ ] **Step 6: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_generic_union_smoke_test.rs`:

```rust
use apache_datasketches::tuple::generic::{
    TupleSketch, TupleSketchBuilder, TupleSummary, TupleUnion, TupleUnionBuilder,
};

#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sketch(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<Sum> {
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

#[test]
fn union_half_overlap() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().lg_k(12).build().unwrap();
    u.update(&sketch(0..1000, 1));
    u.update(&sketch(500..1500, 1));
    assert_eq!(u.get_result(true).get_estimate(), 1500.0);
}

#[test]
fn union_accepts_both_input_types() {
    let a = sketch(0..100, 1);
    let b = sketch(50..150, 1).compact(true);
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&a);
    u.update(&b);
    assert_eq!(u.get_result(true).get_estimate(), 150.0);
}

#[test]
fn union_sums_summaries_on_collision() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&sketch(7..8, 10));
    u.update(&sketch(7..8, 32));
    let entries: Vec<(u64, Sum)> = u.get_result(true).entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, Sum(42));
}

#[test]
fn union_reset_empties_result() {
    let mut u: TupleUnion<Sum> = TupleUnionBuilder::new().build().unwrap();
    u.update(&sketch(0..100, 1));
    assert!(!u.get_result(true).is_empty());
    u.reset();
    assert!(u.get_result(true).is_empty());
}

#[test]
fn invalid_config_is_err() {
    assert!(TupleUnionBuilder::<Sum>::new().lg_k(4).build().is_err());
    assert!(TupleUnionBuilder::<Sum>::new().p(1.5).build().is_err());
}

#[test]
fn union_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleUnion<Sum>>();
}
```

Register as `tuple_generic_union_smoke_test`, `required-features = ["tuple"]`.

- [ ] **Step 7: Run to verify it fails**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_union_smoke_test`
Expected: FAIL — `no TupleUnion in apache_datasketches::tuple::generic`.

- [ ] **Step 8: Write the safe input trait and union**

Create `apache-datasketches/src/tuple/generic/input.rs`:

```rust
use super::{CompactTupleSketch, TupleSketch, TupleSummary};
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;

mod sealed {
    use super::TupleSummary;
    pub trait Sealed {}
    impl<S: TupleSummary> Sealed for super::TupleSketch<S> {}
    impl<S: TupleSummary> Sealed for super::CompactTupleSketch<S> {}
}

/// Either generic Tuple sketch type can be fed into this module's set
/// operations. Sealed — the shims have concrete overloads for these two types
/// only.
pub trait TupleInput<S: TupleSummary>: sealed::Sealed {
    #[doc(hidden)]
    fn as_input(&self) -> TupleGenericInputRef<'_>;
}

impl<S: TupleSummary> TupleInput<S> for TupleSketch<S> {
    fn as_input(&self) -> TupleGenericInputRef<'_> {
        TupleGenericInputRef::Sketch(&self.inner)
    }
}

impl<S: TupleSummary> TupleInput<S> for CompactTupleSketch<S> {
    fn as_input(&self) -> TupleGenericInputRef<'_> {
        TupleGenericInputRef::Compact(&self.inner)
    }
}
```

Create `apache-datasketches/src/tuple/generic/union.rs`:

```rust
use super::builder::resize_factor_multiplier;
use super::{CompactTupleSketch, TupleInput, TupleSummary};
use crate::error::SketchError;
use crate::tuple::ResizeFactor;
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;
use apache_datasketches_sys::tuple_generic_union::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// Builder for [`TupleUnion`]. `lg_k` defaults to `12`, `resize_factor` to
/// [`ResizeFactor::X8`], `p` to `1.0`.
#[derive(Debug, Clone, Copy)]
pub struct TupleUnionBuilder<S: TupleSummary> {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
    _marker: PhantomData<fn() -> S>,
}

impl<S: TupleSummary> Default for TupleUnionBuilder<S> {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::X8,
            p: 1.0,
            _marker: PhantomData,
        }
    }
}

impl<S: TupleSummary> TupleUnionBuilder<S> {
    /// Creates a builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the target number of retained entries.
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Sets the hash table's growth [`ResizeFactor`].
    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    /// Sets the sampling probability.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Builds the union. Returns [`SketchError::InvalidConfig`] if `lg_k` is
    /// out of range or `p` is outside `(0, 1]`.
    pub fn build(self) -> Result<TupleUnion<S>, SketchError> {
        let inner = sys::new_tuple_generic_union(
            self.lg_k,
            resize_factor_multiplier(self.resize_factor),
            self.p,
        )
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))?;
        Ok(TupleUnion {
            inner,
            _marker: PhantomData,
        })
    }
}

/// A streaming union over generic Tuple sketches. Summaries for a key present
/// in more than one input are merged with
/// [`TupleSummary::union_combine`](super::TupleSummary::union_combine).
pub struct TupleUnion<S: TupleSummary> {
    inner: UniquePtr<sys::TupleGenericUnionShim>,
    _marker: PhantomData<fn() -> S>,
}

unsafe impl<S: TupleSummary> Send for TupleUnion<S> {}

impl<S: TupleSummary> TupleUnion<S> {
    /// Merges the given sketch into the running result.
    ///
    /// Infallible: unlike ArrayOfDoubles there is no `num_values` to agree
    /// on — the type system already guarantees both operands carry `S`.
    pub fn update(&mut self, input: &impl TupleInput<S>) {
        match input.as_input() {
            TupleGenericInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            TupleGenericInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
        }
    }

    /// Returns the union's current result. If `ordered` is `true`, entries
    /// are sorted by hash value.
    pub fn get_result(&self, ordered: bool) -> CompactTupleSketch<S> {
        CompactTupleSketch::from_shim(self.inner.get_result(ordered))
    }

    /// Resets this union to its initial, empty state.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }
}
```

Update `mod.rs` to declare `mod input; mod union;` and re-export `TupleInput`, `TupleUnion`, `TupleUnionBuilder`.

- [ ] **Step 9: Run the tests, then commit**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS, including `tuple_generic_union_smoke_test` (6 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add generic input dispatch and TupleUnion<S>"
```

---

### Task 5: `TupleIntersection<S>`

Structurally the same as Task 4. Two differences: no builder (upstream's type has a plain constructor, matching `ThetaIntersection` and `ArrayOfDoublesIntersection`), and `get_result` returns `Result` because of the existing `EmptyIntersection` case. The intersection policy calls `intersection_combine`, so the asymmetric test summary is what proves the two trampolines are not cross-wired.

**Files:** `cpp/tuple/tuple_generic_intersection_shim.h`/`.cc`, `src/tuple_generic_intersection.rs`, `tests/tuple_generic_intersection_link_test.rs`, `apache-datasketches/src/tuple/generic/intersection.rs`, `tests/tuple_generic_intersection_smoke_test.rs`, plus the usual registrations.

**Interfaces:**
- Consumes: the same shim types and `TupleInput<S>` as Task 4; `dyn_intersection`, `DynIntersectionPolicy`.
- Produces: `TupleIntersection<S>` with `new()`, `Default`, `update(&impl TupleInput<S>)`, `get_result(bool) -> Result<CompactTupleSketch<S>, SketchError>`, `has_result() -> bool`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/tuple_generic_intersection_link_test.rs`, reusing the same `Sum` helper as the union link test (copy it in — each integration test is its own crate):

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi as sketch_ffi, RawSummaryOps, RustSummary};
use apache_datasketches_sys::tuple_generic_intersection::ffi as isect_ffi;
use std::any::Any;

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn sketch(
    keys: std::ops::Range<u64>,
    v: i64,
) -> cxx::UniquePtr<sketch_ffi::TupleGenericSketchShim> {
    let mut s = sketch_ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for key in keys {
        s.pin_mut()
            .update_u64(key, &RustSummary::new(Box::new(Sum(v))));
    }
    s
}

#[test]
fn intersection_half_overlap() {
    let mut i = isect_ffi::new_tuple_generic_intersection();
    assert!(!i.has_result());
    i.pin_mut().update_with_sketch(&sketch(0..1000, 1));
    i.pin_mut().update_with_sketch(&sketch(500..1500, 1));
    assert!(i.has_result());
    assert_eq!(i.get_result(true).unwrap().get_estimate(), 500.0);
}

#[test]
fn intersection_uses_the_intersection_trampoline_not_the_union_one() {
    let mut i = isect_ffi::new_tuple_generic_intersection();
    i.pin_mut().update_with_sketch(&sketch(7..8, 10));
    i.pin_mut().update_with_sketch(&sketch(7..8, 32));
    let result = i.get_result(true).unwrap();
    assert_eq!(result.entry_count(), 1);
    let v = result
        .entry_summary(0)
        .unwrap()
        .ops()
        .as_any()
        .downcast_ref::<Sum>()
        .unwrap()
        .0;
    assert_eq!(v, 10, "min, not sum -- a value of 42 means the trampolines are crossed");
}

#[test]
fn get_result_without_update_is_err() {
    let i = isect_ffi::new_tuple_generic_intersection();
    assert!(!i.has_result());
    assert!(i.get_result(true).is_err());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_intersection_link_test`
Expected: FAIL — `unresolved import apache_datasketches_sys::tuple_generic_intersection`.

- [ ] **Step 3: Write the intersection shim**

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_intersection_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"

namespace apache_datasketches_rs {

class TupleGenericIntersectionShim {
public:
  TupleGenericIntersectionShim();

  void update_with_sketch(const TupleGenericSketchShim& sketch);
  void update_with_compact(const CompactTupleGenericSketchShim& sketch);

  std::unique_ptr<CompactTupleGenericSketchShim> get_result(bool ordered) const;
  bool has_result() const;

private:
  dyn_intersection intersection_;
};

std::unique_ptr<TupleGenericIntersectionShim> new_tuple_generic_intersection();

} // namespace apache_datasketches_rs
```

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_intersection_shim.cc`:

```cpp
#include "tuple_generic_intersection_shim.h"

namespace apache_datasketches_rs {

TupleGenericIntersectionShim::TupleGenericIntersectionShim()
  : intersection_(datasketches::DEFAULT_SEED, DynIntersectionPolicy()) {}

void TupleGenericIntersectionShim::update_with_sketch(const TupleGenericSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

void TupleGenericIntersectionShim::update_with_compact(const CompactTupleGenericSketchShim& sketch) {
  intersection_.update(sketch.inner());
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericIntersectionShim::get_result(bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(intersection_.get_result(ordered));
}

bool TupleGenericIntersectionShim::has_result() const { return intersection_.has_result(); }

std::unique_ptr<TupleGenericIntersectionShim> new_tuple_generic_intersection() {
  return std::make_unique<TupleGenericIntersectionShim>();
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the bridge**

Create `apache-datasketches-sys/src/tuple_generic_intersection.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");
        include!("tuple_generic_compact_shim.h");
        include!("tuple_generic_intersection_shim.h");

        type TupleGenericSketchShim = crate::tuple_generic::ffi::TupleGenericSketchShim;
        type CompactTupleGenericSketchShim = crate::tuple_generic::ffi::CompactTupleGenericSketchShim;

        type TupleGenericIntersectionShim;

        fn new_tuple_generic_intersection() -> UniquePtr<TupleGenericIntersectionShim>;

        fn update_with_sketch(self: Pin<&mut TupleGenericIntersectionShim>, sketch: &TupleGenericSketchShim);
        fn update_with_compact(self: Pin<&mut TupleGenericIntersectionShim>, sketch: &CompactTupleGenericSketchShim);

        fn get_result(self: &TupleGenericIntersectionShim, ordered: bool) -> Result<UniquePtr<CompactTupleGenericSketchShim>>;
        fn has_result(self: &TupleGenericIntersectionShim) -> bool;
    }
}
```

Register the module in `lib.rs`, the bridge and `.cc` in `build.rs`, plus `rerun-if-changed` lines.

- [ ] **Step 5: Run the link test**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_intersection_link_test`
Expected: PASS (3 tests).

- [ ] **Step 6: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_generic_intersection_smoke_test.rs`:

```rust
use apache_datasketches::tuple::generic::{
    TupleIntersection, TupleSketch, TupleSketchBuilder, TupleSummary,
};
use apache_datasketches::SketchError;

#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sketch(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<Sum> {
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

#[test]
fn intersection_half_overlap() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(0..1000, 1));
    i.update(&sketch(500..1500, 1));
    assert_eq!(i.get_result(true).unwrap().get_estimate(), 500.0);
}

#[test]
fn intersection_uses_intersection_semantics_not_union() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(7..8, 10));
    i.update(&sketch(7..8, 32));
    let entries: Vec<(u64, Sum)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, Sum(10), "min; Sum(42) would mean union semantics leaked in");
}

#[test]
fn intersection_accepts_both_input_types() {
    let mut i: TupleIntersection<Sum> = TupleIntersection::new();
    i.update(&sketch(0..100, 1));
    i.update(&sketch(50..150, 1).compact(true));
    assert_eq!(i.get_result(true).unwrap().get_estimate(), 50.0);
}

#[test]
fn get_result_before_update_is_empty_intersection_err() {
    let i: TupleIntersection<Sum> = TupleIntersection::new();
    assert!(!i.has_result());
    assert!(matches!(i.get_result(true), Err(SketchError::EmptyIntersection)));
}

#[test]
fn intersection_is_send_and_default() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleIntersection<Sum>>();
    let _ = TupleIntersection::<Sum>::default();
}
```

- [ ] **Step 7: Run to verify it fails, then write the wrapper**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_intersection_smoke_test`
Expected: FAIL — `no TupleIntersection in apache_datasketches::tuple::generic`.

Create `apache-datasketches/src/tuple/generic/intersection.rs`:

```rust
use super::{CompactTupleSketch, TupleInput, TupleSummary};
use crate::error::SketchError;
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;
use apache_datasketches_sys::tuple_generic_intersection::ffi as sys;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// Computes the intersection of generic Tuple sketches fed via
/// [`Self::update`]. Summaries of keys present in every input are merged with
/// [`TupleSummary::intersection_combine`](super::TupleSummary::intersection_combine).
///
/// No builder: upstream's type has a plain constructor, matching
/// [`ArrayOfDoublesIntersection`](crate::tuple::ArrayOfDoublesIntersection).
pub struct TupleIntersection<S: TupleSummary> {
    inner: UniquePtr<sys::TupleGenericIntersectionShim>,
    _marker: PhantomData<fn() -> S>,
}

unsafe impl<S: TupleSummary> Send for TupleIntersection<S> {}

impl<S: TupleSummary> Default for TupleIntersection<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: TupleSummary> TupleIntersection<S> {
    /// Creates an intersection with no result yet — call [`Self::update`] at
    /// least once before [`Self::get_result`].
    pub fn new() -> Self {
        Self {
            inner: sys::new_tuple_generic_intersection(),
            _marker: PhantomData,
        }
    }

    /// Narrows the running result to also require membership in `input`.
    pub fn update(&mut self, input: &impl TupleInput<S>) {
        match input.as_input() {
            TupleGenericInputRef::Sketch(s) => self.inner.pin_mut().update_with_sketch(s),
            TupleGenericInputRef::Compact(c) => self.inner.pin_mut().update_with_compact(c),
        }
    }

    /// Returns the current result, or [`SketchError::EmptyIntersection`] if
    /// [`Self::update`] has never been called.
    pub fn get_result(&self, ordered: bool) -> Result<CompactTupleSketch<S>, SketchError> {
        if !self.inner.has_result() {
            return Err(SketchError::EmptyIntersection);
        }
        let inner = self
            .inner
            .get_result(ordered)
            .map_err(|e| SketchError::Cpp(e.what().to_string()))?;
        Ok(CompactTupleSketch::from_shim(inner))
    }

    /// Returns `true` if [`Self::update`] has been called at least once.
    pub fn has_result(&self) -> bool {
        self.inner.has_result()
    }
}
```

Declare and re-export it from `mod.rs`.

- [ ] **Step 8: Run the tests, then commit**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS, including `tuple_generic_intersection_smoke_test` (5 tests).

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add TupleIntersection<S>"
```

---

### Task 6: `TupleAnotB<S>`

Stateless set difference. Upstream's `tuple_a_not_b::compute` is a template over both operand types, so the shim provides four concrete overloads and the safe wrapper dispatches with one `compute`. A-not-b needs no policy at all — the result keeps operand `a`'s summaries unchanged.

**Files:** `cpp/tuple/tuple_generic_a_not_b_shim.h`/`.cc`, `src/tuple_generic_a_not_b.rs`, `tests/tuple_generic_a_not_b_link_test.rs`, `apache-datasketches/src/tuple/generic/a_not_b.rs`, `tests/tuple_generic_a_not_b_smoke_test.rs`, plus registrations in `lib.rs`, `mod.rs`, both `Cargo.toml`s and `build.rs`.

**Interfaces:**
- Consumes: `TupleGenericSketchShim`, `CompactTupleGenericSketchShim`, `dyn_a_not_b`, `TupleInput<S>`, `CompactTupleSketch::from_shim`.
- Produces: C++ `TupleGenericAnotBShim` with `compute_sketch_sketch`, `compute_sketch_compact`, `compute_compact_sketch`, `compute_compact_compact`, each `(a, b, bool ordered) const`; free function `new_tuple_generic_a_not_b()`. Rust: `TupleAnotB<S>` with `new()`, `Default`, `compute(&self, a: &impl TupleInput<S>, b: &impl TupleInput<S>, ordered: bool) -> CompactTupleSketch<S>`.

**Note on the operand-transposition risk:** four overloads with identical bodies differing only in parameter types are easy to cross-wire, and a symmetric fixture cannot detect it. Both the link test and the smoke test below use an asymmetric fixture (`a = 0..1000`, `b = 0..500`, so `a - b` estimates 500 while `b - a` estimates 0) specifically so a swap in either mixed overload fails a test. This is the direct lesson from the ArrayOfDoubles a-not-b review.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/tuple_generic_a_not_b_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi as sketch_ffi, RawSummaryOps, RustSummary};
use apache_datasketches_sys::tuple_generic_a_not_b::ffi as anb_ffi;
use std::any::Any;

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn sketch(
    keys: std::ops::Range<u64>,
    v: i64,
) -> cxx::UniquePtr<sketch_ffi::TupleGenericSketchShim> {
    let mut s = sketch_ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for key in keys {
        s.pin_mut()
            .update_u64(key, &RustSummary::new(Box::new(Sum(v))));
    }
    s
}

/// Asymmetric on purpose: a - b estimates 500, b - a estimates 0, so a
/// transposed mixed-type overload changes an asserted value.
#[test]
fn all_four_overloads_preserve_operand_order() {
    let a = sketch(0..1000, 1);
    let b = sketch(0..500, 1);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc = anb_ffi::new_tuple_generic_a_not_b();

    assert_eq!(calc.compute_sketch_sketch(&a, &b, true).get_estimate(), 500.0);
    assert_eq!(calc.compute_sketch_compact(&a, &cb, true).get_estimate(), 500.0);
    assert_eq!(calc.compute_compact_sketch(&ca, &b, true).get_estimate(), 500.0);
    assert_eq!(calc.compute_compact_compact(&ca, &cb, true).get_estimate(), 500.0);

    // Reversed: every combination must now be empty.
    assert_eq!(calc.compute_sketch_sketch(&b, &a, true).get_estimate(), 0.0);
    assert_eq!(calc.compute_sketch_compact(&b, &ca, true).get_estimate(), 0.0);
    assert_eq!(calc.compute_compact_sketch(&cb, &a, true).get_estimate(), 0.0);
    assert_eq!(calc.compute_compact_compact(&cb, &ca, true).get_estimate(), 0.0);
}

#[test]
fn result_preserves_operand_a_summaries() {
    let a = sketch(0..1, 17);
    let b = sketch(100..101, 3);
    let calc = anb_ffi::new_tuple_generic_a_not_b();
    let result = calc.compute_sketch_sketch(&a, &b, true);
    assert_eq!(result.entry_count(), 1);
    let v = result
        .entry_summary(0)
        .unwrap()
        .ops()
        .as_any()
        .downcast_ref::<Sum>()
        .unwrap()
        .0;
    assert_eq!(v, 17, "a's summary must pass through unchanged");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_a_not_b_link_test`
Expected: FAIL — `unresolved import apache_datasketches_sys::tuple_generic_a_not_b`.

- [ ] **Step 3: Write the a-not-b shim**

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_a_not_b_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include <memory>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"

namespace apache_datasketches_rs {

class TupleGenericAnotBShim {
public:
  TupleGenericAnotBShim();

  std::unique_ptr<CompactTupleGenericSketchShim> compute_sketch_sketch(const TupleGenericSketchShim& a, const TupleGenericSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactTupleGenericSketchShim> compute_sketch_compact(const TupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactTupleGenericSketchShim> compute_compact_sketch(const CompactTupleGenericSketchShim& a, const TupleGenericSketchShim& b, bool ordered) const;
  std::unique_ptr<CompactTupleGenericSketchShim> compute_compact_compact(const CompactTupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b, bool ordered) const;

private:
  dyn_a_not_b a_not_b_;
};

std::unique_ptr<TupleGenericAnotBShim> new_tuple_generic_a_not_b();

} // namespace apache_datasketches_rs
```

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_a_not_b_shim.cc`:

```cpp
#include "tuple_generic_a_not_b_shim.h"

namespace apache_datasketches_rs {

TupleGenericAnotBShim::TupleGenericAnotBShim() : a_not_b_() {}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericAnotBShim::compute_sketch_sketch(const TupleGenericSketchShim& a, const TupleGenericSketchShim& b, bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericAnotBShim::compute_sketch_compact(const TupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b, bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericAnotBShim::compute_compact_sketch(const CompactTupleGenericSketchShim& a, const TupleGenericSketchShim& b, bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<CompactTupleGenericSketchShim> TupleGenericAnotBShim::compute_compact_compact(const CompactTupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b, bool ordered) const {
  return std::make_unique<CompactTupleGenericSketchShim>(a_not_b_.compute(a.inner(), b.inner(), ordered));
}

std::unique_ptr<TupleGenericAnotBShim> new_tuple_generic_a_not_b() {
  return std::make_unique<TupleGenericAnotBShim>();
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the bridge**

Create `apache-datasketches-sys/src/tuple_generic_a_not_b.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");
        include!("tuple_generic_compact_shim.h");
        include!("tuple_generic_a_not_b_shim.h");

        type TupleGenericSketchShim = crate::tuple_generic::ffi::TupleGenericSketchShim;
        type CompactTupleGenericSketchShim = crate::tuple_generic::ffi::CompactTupleGenericSketchShim;

        type TupleGenericAnotBShim;

        fn new_tuple_generic_a_not_b() -> UniquePtr<TupleGenericAnotBShim>;

        fn compute_sketch_sketch(self: &TupleGenericAnotBShim, a: &TupleGenericSketchShim, b: &TupleGenericSketchShim, ordered: bool) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn compute_sketch_compact(self: &TupleGenericAnotBShim, a: &TupleGenericSketchShim, b: &CompactTupleGenericSketchShim, ordered: bool) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn compute_compact_sketch(self: &TupleGenericAnotBShim, a: &CompactTupleGenericSketchShim, b: &TupleGenericSketchShim, ordered: bool) -> UniquePtr<CompactTupleGenericSketchShim>;
        fn compute_compact_compact(self: &TupleGenericAnotBShim, a: &CompactTupleGenericSketchShim, b: &CompactTupleGenericSketchShim, ordered: bool) -> UniquePtr<CompactTupleGenericSketchShim>;
    }
}
```

Register the module, the bridge, the `.cc`, and the `rerun-if-changed` lines.

- [ ] **Step 5: Run the link test**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_a_not_b_link_test`
Expected: PASS (2 tests).

- [ ] **Step 6: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_generic_a_not_b_smoke_test.rs`:

```rust
use apache_datasketches::tuple::generic::{
    TupleAnotB, TupleSketch, TupleSketchBuilder, TupleSummary,
};

#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sketch(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<Sum> {
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

#[test]
fn all_four_combinations_preserve_operand_order() {
    let a = sketch(0..1000, 1);
    let b = sketch(0..500, 1);
    let ca = a.compact(true);
    let cb = b.compact(true);
    let calc: TupleAnotB<Sum> = TupleAnotB::new();

    assert_eq!(calc.compute(&a, &b, true).get_estimate(), 500.0);
    assert_eq!(calc.compute(&a, &cb, true).get_estimate(), 500.0);
    assert_eq!(calc.compute(&ca, &b, true).get_estimate(), 500.0);
    assert_eq!(calc.compute(&ca, &cb, true).get_estimate(), 500.0);

    assert_eq!(calc.compute(&b, &a, true).get_estimate(), 0.0);
    assert_eq!(calc.compute(&cb, &ca, true).get_estimate(), 0.0);
}

#[test]
fn result_preserves_operand_a_summaries() {
    let calc: TupleAnotB<Sum> = TupleAnotB::new();
    let result = calc.compute(&sketch(0..1, 17), &sketch(100..101, 3), true);
    let entries: Vec<(u64, Sum)> = result.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, Sum(17));
}

#[test]
fn a_not_b_self_is_empty() {
    let a = sketch(0..100, 1);
    let calc: TupleAnotB<Sum> = TupleAnotB::new();
    assert!(calc.compute(&a, &a, true).is_empty());
}

#[test]
fn a_not_b_is_send_and_default() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleAnotB<Sum>>();
    let _ = TupleAnotB::<Sum>::default();
}
```

- [ ] **Step 7: Run to verify it fails, then write the wrapper**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_a_not_b_smoke_test`
Expected: FAIL — `no TupleAnotB in apache_datasketches::tuple::generic`.

Create `apache-datasketches/src/tuple/generic/a_not_b.rs`:

```rust
use super::{CompactTupleSketch, TupleInput, TupleSummary};
use apache_datasketches_sys::tuple_generic_a_not_b::ffi as sys;
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// Computes the set difference `a - b` over generic Tuple sketches.
/// Retained entries keep `a`'s summaries unchanged — a-not-b has no combine
/// policy. Stateless between calls.
pub struct TupleAnotB<S: TupleSummary> {
    inner: UniquePtr<sys::TupleGenericAnotBShim>,
    _marker: PhantomData<fn() -> S>,
}

unsafe impl<S: TupleSummary> Send for TupleAnotB<S> {}

impl<S: TupleSummary> Default for TupleAnotB<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: TupleSummary> TupleAnotB<S> {
    /// Creates a reusable a-not-b calculator.
    pub fn new() -> Self {
        Self {
            inner: sys::new_tuple_generic_a_not_b(),
            _marker: PhantomData,
        }
    }

    /// Computes `a - b`: keys in `a` that are not in `b`, carrying `a`'s
    /// summaries. If `ordered` is `true`, the result's entries are sorted by
    /// hash value.
    pub fn compute(
        &self,
        a: &impl TupleInput<S>,
        b: &impl TupleInput<S>,
        ordered: bool,
    ) -> CompactTupleSketch<S> {
        let inner = match (a.as_input(), b.as_input()) {
            (TupleGenericInputRef::Sketch(a), TupleGenericInputRef::Sketch(b)) => {
                self.inner.compute_sketch_sketch(a, b, ordered)
            }
            (TupleGenericInputRef::Sketch(a), TupleGenericInputRef::Compact(b)) => {
                self.inner.compute_sketch_compact(a, b, ordered)
            }
            (TupleGenericInputRef::Compact(a), TupleGenericInputRef::Sketch(b)) => {
                self.inner.compute_compact_sketch(a, b, ordered)
            }
            (TupleGenericInputRef::Compact(a), TupleGenericInputRef::Compact(b)) => {
                self.inner.compute_compact_compact(a, b, ordered)
            }
        };
        CompactTupleSketch::from_shim(inner)
    }
}
```

Declare and re-export it from `mod.rs`.

- [ ] **Step 8: Run the tests, then commit**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS, including `tuple_generic_a_not_b_smoke_test` (4 tests).

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add TupleAnotB<S>"
```

---

### Task 7: `tuple_jaccard_similarity<S>`

Assembles Jaccard from upstream's generic `jaccard_similarity_base`, instantiated with the union and intersection types from Tasks 4 and 5. Upstream ships no ready-made alias for this.

**This is where the stateless-policy constraint pays off.** `jaccard_similarity_base::jaccard()` internally builds a scratch union via `typename Union::builder()` and a scratch intersection via `Intersection(seed)`, both with **default-constructed** policies. `DynUnionPolicy` and `DynIntersectionPolicy` carry no state — they dispatch through the summary object — so default construction is trivially correct. Do not add fields to those structs.

**Shared struct naming:** this bridge declares its own `TupleGenericJaccardBoundsFfi` rather than reusing the ArrayOfDoubles bridge's `TupleJaccardBoundsFfi`. Cross-bridge sharing of a *shared struct* is not something this plan relies on, and the global-uniqueness rule requires a distinct name anyway. The safe crate maps it onto the existing `tuple::JaccardBounds`, so users see one type.

**Files:** `cpp/tuple/tuple_generic_jaccard_shim.h`/`.cc`, `src/tuple_generic_jaccard.rs`, `tests/tuple_generic_jaccard_link_test.rs`, `apache-datasketches/src/tuple/generic/jaccard.rs`, `tests/tuple_generic_jaccard_smoke_test.rs`, plus registrations.

**Interfaces:**
- Consumes: `dyn_union`, `dyn_intersection`, `DynSummary`, both shim types, `TupleInput<S>`, `crate::tuple::JaccardBounds`.
- Produces: shared struct `TupleGenericJaccardBoundsFfi { lower_bound: f64, estimate: f64, upper_bound: f64 }`; free functions `tuple_generic_jaccard_sketch_sketch`, `_sketch_compact`, `_compact_sketch`, `_compact_compact`; Rust `tuple_jaccard_similarity<S>(a: &impl TupleInput<S>, b: &impl TupleInput<S>) -> JaccardBounds`.

- [ ] **Step 1: Write the failing sys-crate link test**

Create `apache-datasketches-sys/tests/tuple_generic_jaccard_link_test.rs`:

```rust
#![cfg(feature = "tuple")]

use apache_datasketches_sys::tuple_generic::{ffi as sketch_ffi, RawSummaryOps, RustSummary};
use apache_datasketches_sys::tuple_generic_jaccard::ffi as jac_ffi;
use std::any::Any;

#[derive(Debug)]
struct Sum(i64);

impl RawSummaryOps for Sum {
    fn clone_boxed(&self) -> Box<dyn RawSummaryOps + Send> {
        Box::new(Sum(self.0))
    }
    fn union_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 += other.as_any().downcast_ref::<Sum>().unwrap().0;
    }
    fn intersection_combine(&mut self, other: &dyn RawSummaryOps) {
        self.0 = self.0.min(other.as_any().downcast_ref::<Sum>().unwrap().0);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn sketch(
    keys: std::ops::Range<u64>,
    v: i64,
) -> cxx::UniquePtr<sketch_ffi::TupleGenericSketchShim> {
    let mut s = sketch_ffi::new_tuple_generic_sketch(12, 8, 1.0).unwrap();
    for key in keys {
        s.pin_mut()
            .update_u64(key, &RustSummary::new(Box::new(Sum(v))));
    }
    s
}

#[test]
fn identical_sketches_are_fully_similar() {
    let a = sketch(0..1000, 1);
    let b = sketch(0..1000, 1);
    let bounds = jac_ffi::tuple_generic_jaccard_sketch_sketch(&a, &b);
    assert_eq!(bounds.estimate, 1.0);
}

#[test]
fn disjoint_sketches_are_dissimilar() {
    let a = sketch(0..1000, 1);
    let b = sketch(2000..3000, 1);
    assert_eq!(jac_ffi::tuple_generic_jaccard_sketch_sketch(&a, &b).estimate, 0.0);
}

#[test]
fn half_overlap_is_about_one_third_in_all_four_combinations() {
    let a = sketch(0..1000, 1);
    let b = sketch(500..1500, 1);
    let ca = a.compact(true);
    let cb = b.compact(true);
    for bounds in [
        jac_ffi::tuple_generic_jaccard_sketch_sketch(&a, &b),
        jac_ffi::tuple_generic_jaccard_sketch_compact(&a, &cb),
        jac_ffi::tuple_generic_jaccard_compact_sketch(&ca, &b),
        jac_ffi::tuple_generic_jaccard_compact_compact(&ca, &cb),
    ] {
        assert!((bounds.estimate - 1.0 / 3.0).abs() < 0.01);
        assert!(bounds.lower_bound <= bounds.estimate);
        assert!(bounds.estimate <= bounds.upper_bound);
    }
}

#[test]
fn summary_values_do_not_affect_the_result() {
    let baseline = jac_ffi::tuple_generic_jaccard_sketch_sketch(&sketch(0..1000, 1), &sketch(500..1500, 1));
    let different = jac_ffi::tuple_generic_jaccard_sketch_sketch(&sketch(0..1000, 99), &sketch(500..1500, -7));
    assert_eq!(baseline.estimate, different.estimate);
    assert_eq!(baseline.lower_bound, different.lower_bound);
    assert_eq!(baseline.upper_bound, different.upper_bound);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_jaccard_link_test`
Expected: FAIL — `unresolved import apache_datasketches_sys::tuple_generic_jaccard`.

- [ ] **Step 3: Write the jaccard shim**

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_jaccard_shim.h`:

```cpp
#pragma once
#include <cstdint>
#include "rust/cxx.h"
#include "dyn_summary.h"
#include "tuple_jaccard_similarity.hpp"
#include "tuple_generic_sketch_shim.h"
#include "tuple_generic_compact_shim.h"

namespace apache_datasketches_rs {

struct TupleGenericJaccardBoundsFfi;

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_sketch_sketch(const TupleGenericSketchShim& a, const TupleGenericSketchShim& b);
TupleGenericJaccardBoundsFfi tuple_generic_jaccard_sketch_compact(const TupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b);
TupleGenericJaccardBoundsFfi tuple_generic_jaccard_compact_sketch(const CompactTupleGenericSketchShim& a, const TupleGenericSketchShim& b);
TupleGenericJaccardBoundsFfi tuple_generic_jaccard_compact_compact(const CompactTupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b);

} // namespace apache_datasketches_rs
```

Create `apache-datasketches-sys/cpp/tuple/tuple_generic_jaccard_shim.cc`:

```cpp
#include "tuple_generic_jaccard_shim.h"
#include "tuple_generic_jaccard.rs.h"

namespace apache_datasketches_rs {

namespace {

// Upstream provides no jaccard alias for a generic Tuple summary, so we
// instantiate the same generic template it uses internally.
//
// jaccard() builds a scratch union via `typename Union::builder()` and a
// scratch intersection via `Intersection(seed)`, both with default-constructed
// policies. DynUnionPolicy and DynIntersectionPolicy are stateless, so that is
// correct -- do not give them fields. jaccard() also reads only
// get_num_retained()/get_theta64()/is_empty() from those scratch results, never
// a summary's contents, so the callbacks it triggers cannot affect the bounds.
using dyn_jaccard = datasketches::jaccard_similarity_base<
    dyn_union,
    dyn_intersection,
    datasketches::pair_extract_key<uint64_t, DynSummary>>;

TupleGenericJaccardBoundsFfi to_ffi(const std::array<double, 3>& result) {
  return TupleGenericJaccardBoundsFfi{result[0], result[1], result[2]};
}

} // namespace

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_sketch_sketch(const TupleGenericSketchShim& a, const TupleGenericSketchShim& b) {
  return to_ffi(dyn_jaccard::jaccard(a.inner(), b.inner()));
}

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_sketch_compact(const TupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b) {
  return to_ffi(dyn_jaccard::jaccard(a.inner(), b.inner()));
}

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_compact_sketch(const CompactTupleGenericSketchShim& a, const TupleGenericSketchShim& b) {
  return to_ffi(dyn_jaccard::jaccard(a.inner(), b.inner()));
}

TupleGenericJaccardBoundsFfi tuple_generic_jaccard_compact_compact(const CompactTupleGenericSketchShim& a, const CompactTupleGenericSketchShim& b) {
  return to_ffi(dyn_jaccard::jaccard(a.inner(), b.inner()));
}

} // namespace apache_datasketches_rs
```

- [ ] **Step 4: Write the bridge**

Create `apache-datasketches-sys/src/tuple_generic_jaccard.rs`:

```rust
#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    /// Distinct from the ArrayOfDoubles bridge's `TupleJaccardBoundsFfi`:
    /// cxx emits one C++ definition per shared type into the bridge
    /// namespace, and names must be globally unique across bridges.
    struct TupleGenericJaccardBoundsFfi {
        lower_bound: f64,
        estimate: f64,
        upper_bound: f64,
    }

    unsafe extern "C++" {
        include!("tuple_generic_sketch_shim.h");
        include!("tuple_generic_compact_shim.h");
        include!("tuple_generic_jaccard_shim.h");

        type TupleGenericSketchShim = crate::tuple_generic::ffi::TupleGenericSketchShim;
        type CompactTupleGenericSketchShim = crate::tuple_generic::ffi::CompactTupleGenericSketchShim;

        fn tuple_generic_jaccard_sketch_sketch(a: &TupleGenericSketchShim, b: &TupleGenericSketchShim) -> TupleGenericJaccardBoundsFfi;
        fn tuple_generic_jaccard_sketch_compact(a: &TupleGenericSketchShim, b: &CompactTupleGenericSketchShim) -> TupleGenericJaccardBoundsFfi;
        fn tuple_generic_jaccard_compact_sketch(a: &CompactTupleGenericSketchShim, b: &TupleGenericSketchShim) -> TupleGenericJaccardBoundsFfi;
        fn tuple_generic_jaccard_compact_compact(a: &CompactTupleGenericSketchShim, b: &CompactTupleGenericSketchShim) -> TupleGenericJaccardBoundsFfi;
    }
}
```

Register the module, the bridge, the `.cc`, and the `rerun-if-changed` lines.

- [ ] **Step 5: Run the link test**

Run: `cargo test -p apache-datasketches-sys --features tuple --test tuple_generic_jaccard_link_test`
Expected: PASS (4 tests).

- [ ] **Step 6: Write the failing safe-crate smoke test**

Create `apache-datasketches/tests/tuple_generic_jaccard_smoke_test.rs`:

```rust
use apache_datasketches::tuple::generic::{
    tuple_jaccard_similarity, TupleSketch, TupleSketchBuilder, TupleSummary,
};

#[derive(Clone, Debug, PartialEq)]
struct Sum(i64);

impl TupleSummary for Sum {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Sum(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sketch(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<Sum> {
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

#[test]
fn identical_sketches_are_fully_similar() {
    let bounds = tuple_jaccard_similarity(&sketch(0..1000, 1), &sketch(0..1000, 1));
    assert_eq!(bounds.estimate, 1.0);
}

#[test]
fn disjoint_sketches_are_dissimilar() {
    assert_eq!(
        tuple_jaccard_similarity(&sketch(0..1000, 1), &sketch(2000..3000, 1)).estimate,
        0.0
    );
}

#[test]
fn half_overlap_accepts_all_four_combinations() {
    let a = sketch(0..1000, 1);
    let b = sketch(500..1500, 1);
    let ca = a.compact(true);
    let cb = b.compact(true);
    for bounds in [
        tuple_jaccard_similarity(&a, &b),
        tuple_jaccard_similarity(&a, &cb),
        tuple_jaccard_similarity(&ca, &b),
        tuple_jaccard_similarity(&ca, &cb),
    ] {
        assert!((bounds.estimate - 1.0 / 3.0).abs() < 0.01);
        assert!(bounds.lower_bound <= bounds.estimate);
        assert!(bounds.estimate <= bounds.upper_bound);
    }
}

#[test]
fn summary_values_do_not_affect_the_result() {
    let baseline = tuple_jaccard_similarity(&sketch(0..1000, 1), &sketch(500..1500, 1));
    let different = tuple_jaccard_similarity(&sketch(0..1000, 99), &sketch(500..1500, -7));
    assert_eq!(baseline, different);
}
```

- [ ] **Step 7: Run to verify it fails, then write the wrapper**

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_jaccard_smoke_test`
Expected: FAIL — `no tuple_jaccard_similarity in apache_datasketches::tuple::generic`.

Create `apache-datasketches/src/tuple/generic/jaccard.rs`:

```rust
use super::{TupleInput, TupleSummary};
use crate::tuple::JaccardBounds;
use apache_datasketches_sys::tuple_generic_input::TupleGenericInputRef;
use apache_datasketches_sys::tuple_generic_jaccard::ffi as sys;

impl From<sys::TupleGenericJaccardBoundsFfi> for JaccardBounds {
    fn from(ffi: sys::TupleGenericJaccardBoundsFfi) -> Self {
        Self {
            lower_bound: ffi.lower_bound,
            estimate: ffi.estimate,
            upper_bound: ffi.upper_bound,
        }
    }
}

/// Estimates the Jaccard index (intersection-over-union) of two generic
/// Tuple sketches.
///
/// Only the keys affect the result — per-entry summaries do not, and no
/// summary callback influences the returned bounds.
pub fn tuple_jaccard_similarity<S: TupleSummary>(
    a: &impl TupleInput<S>,
    b: &impl TupleInput<S>,
) -> JaccardBounds {
    let ffi = match (a.as_input(), b.as_input()) {
        (TupleGenericInputRef::Sketch(a), TupleGenericInputRef::Sketch(b)) => {
            sys::tuple_generic_jaccard_sketch_sketch(a, b)
        }
        (TupleGenericInputRef::Sketch(a), TupleGenericInputRef::Compact(b)) => {
            sys::tuple_generic_jaccard_sketch_compact(a, b)
        }
        (TupleGenericInputRef::Compact(a), TupleGenericInputRef::Sketch(b)) => {
            sys::tuple_generic_jaccard_compact_sketch(a, b)
        }
        (TupleGenericInputRef::Compact(a), TupleGenericInputRef::Compact(b)) => {
            sys::tuple_generic_jaccard_compact_compact(a, b)
        }
    };
    ffi.into()
}
```

Note the `impl From<...> for JaccardBounds` lives here, in the safe crate, which owns `JaccardBounds` — so the orphan rule is satisfied.

Declare and re-export `tuple_jaccard_similarity` from `mod.rs`.

- [ ] **Step 8: Run the tests, then commit**

Run: `cargo test -p apache-datasketches --features tuple`
Expected: PASS, including `tuple_generic_jaccard_smoke_test` (4 tests).

Run: `cargo doc -p apache-datasketches --features tuple --no-deps`
Expected: no warnings.

```bash
git add apache-datasketches-sys apache-datasketches
git commit -m "feat(tuple): add tuple_jaccard_similarity for generic summaries"
```

---

### Task 8: Conformance, ownership accounting, and the deep test suite

The public API is complete after Task 7; this task is the coverage that proves it. Four test files, each targeting something the per-component smoke tests structurally cannot reach.

**Files:**
- Create (test): `apache-datasketches/tests/tuple_generic_conformance_test.rs`
- Create (test): `apache-datasketches/tests/tuple_generic_ownership_test.rs`
- Create (test): `apache-datasketches/tests/tuple_generic_summary_kinds_test.rs`
- Create (test): `apache-datasketches/tests/tuple_generic_panic_test.rs`
- Modify: `apache-datasketches/Cargo.toml`

**Interfaces:** consumes the complete public API from Tasks 2–7. Produces no library API.

- [ ] **Step 1: Behavioural conformance against ArrayOfDoubles**

Create `apache-datasketches/tests/tuple_generic_conformance_test.rs`. The shipped `ArrayOfDoublesSketch` is a tested reference implementation of exactly this shape, so reproducing its behaviour through the generic framework validates the whole callback core at once. Byte-level parity is the serialization design's job; this is behavioural.

```rust
//! Cross-checks the generic framework against the shipped ArrayOfDoubles
//! family. A summary of `Vec<f64>` summed per index is precisely what
//! ArrayOfDoubles implements as a concrete C++ instantiation, so the two must
//! agree on every observable quantity for the same inputs. Any divergence is
//! a bug in the callback core, since the reference side is already tested.

use apache_datasketches::tuple::generic::{
    TupleSketch, TupleSketchBuilder, TupleSummary, TupleUnionBuilder,
};
use apache_datasketches::tuple::{ArrayOfDoublesSketchBuilder, ArrayOfDoublesUnionBuilder};

#[derive(Clone, Debug, PartialEq)]
struct Doubles(Vec<f64>);

impl TupleSummary for Doubles {
    type Update = [f64];
    fn create(update: &[f64]) -> Self {
        Doubles(update.to_vec())
    }
    fn union_combine(&mut self, other: &Self) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a += *b;
        }
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.union_combine(other);
    }
}

#[test]
fn sketch_matches_array_of_doubles_in_exact_mode() {
    let mut generic: TupleSketch<Doubles> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    let mut reference = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();

    for i in 0..1000u64 {
        generic.update_u64(i, &[1.0, 2.0][..]);
        reference.update_u64(i, &[1.0, 2.0]).unwrap();
    }

    assert_eq!(generic.get_estimate(), reference.get_estimate());
    assert_eq!(generic.get_num_retained(), reference.get_num_retained());
    assert_eq!(generic.get_theta(), reference.get_theta());
    assert_eq!(generic.is_estimation_mode(), reference.is_estimation_mode());
}

#[test]
fn sketch_matches_array_of_doubles_in_estimation_mode() {
    let mut generic: TupleSketch<Doubles> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    let mut reference = ArrayOfDoublesSketchBuilder::new()
        .lg_k(12)
        .num_values(2)
        .build()
        .unwrap();

    for i in 0..50_000u64 {
        generic.update_u64(i, &[1.0, 2.0][..]);
        reference.update_u64(i, &[1.0, 2.0]).unwrap();
    }

    assert!(generic.is_estimation_mode());
    assert_eq!(generic.get_estimate(), reference.get_estimate());
    assert_eq!(generic.get_num_retained(), reference.get_num_retained());
    assert_eq!(generic.get_theta(), reference.get_theta());

    // Same keys retained, same summed values.
    let mut g: Vec<(u64, Vec<f64>)> = generic.compact(true).entries().map(|(h, s)| (h, s.0)).collect();
    let mut r: Vec<(u64, Vec<f64>)> = reference.compact(true).entries().collect();
    g.sort_by_key(|(h, _)| *h);
    r.sort_by_key(|(h, _)| *h);
    assert_eq!(g, r);
}

#[test]
fn repeated_keys_sum_the_same_way() {
    let mut generic: TupleSketch<Doubles> = TupleSketchBuilder::new().build().unwrap();
    let mut reference = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    for _ in 0..5 {
        generic.update_u64(42, &[1.0, 2.0][..]);
        reference.update_u64(42, &[1.0, 2.0]).unwrap();
    }
    let g: Vec<(u64, Vec<f64>)> = generic.compact(true).entries().map(|(h, s)| (h, s.0)).collect();
    let r: Vec<(u64, Vec<f64>)> = reference.compact(true).entries().collect();
    assert_eq!(g, r);
    assert_eq!(g[0].1, vec![5.0, 10.0]);
}

#[test]
fn union_matches_array_of_doubles_in_estimation_mode() {
    let mut ga: TupleSketch<Doubles> = TupleSketchBuilder::new().build().unwrap();
    let mut gb: TupleSketch<Doubles> = TupleSketchBuilder::new().build().unwrap();
    let mut ra = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();
    let mut rb = ArrayOfDoublesSketchBuilder::new().num_values(2).build().unwrap();

    for i in 0..30_000u64 {
        ga.update_u64(i, &[1.0, 2.0][..]);
        ra.update_u64(i, &[1.0, 2.0]).unwrap();
    }
    for i in 15_000..45_000u64 {
        gb.update_u64(i, &[1.0, 2.0][..]);
        rb.update_u64(i, &[1.0, 2.0]).unwrap();
    }

    let mut gu = TupleUnionBuilder::<Doubles>::new().build().unwrap();
    gu.update(&ga);
    gu.update(&gb);
    let mut ru = ArrayOfDoublesUnionBuilder::new().num_values(2).build().unwrap();
    ru.update(&ra).unwrap();
    ru.update(&rb).unwrap();

    let g = gu.get_result(true);
    let r = ru.get_result(true);
    assert!(g.is_estimation_mode());
    assert_eq!(g.get_estimate(), r.get_estimate());
    assert_eq!(g.get_num_retained(), r.get_num_retained());
}
```

Register as `tuple_generic_conformance_test`, `required-features = ["tuple"]`.

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_conformance_test`
Expected: PASS (4 tests). **If any assertion fails, stop and report** — a divergence from the reference implementation is a real defect in the callback core, not a test to adjust.

- [ ] **Step 2: Clone/drop accounting**

Create `apache-datasketches/tests/tuple_generic_ownership_test.rs`. This is the highest-value test in the suite: a leak or double-free in the `rust::Box` ownership dance is silent, and Miri cannot help because it does not execute C++ FFI.

```rust
//! Verifies that every summary C++ clones is eventually dropped exactly once.
//!
//! The counters are process-global, so these tests must not run concurrently
//! with each other -- they are serialised through a mutex rather than split
//! across test binaries.

use apache_datasketches::tuple::generic::{
    TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder, TupleSummary, TupleUnionBuilder,
};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

static LIVE: AtomicI64 = AtomicI64::new(0);

fn lock() -> MutexGuard<'static, ()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    // A poisoned mutex just means a previous test failed; the counter reset
    // below makes each test independent anyway.
    match M.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Debug)]
struct Counted(i64);

impl Counted {
    fn new(v: i64) -> Self {
        LIVE.fetch_add(1, Ordering::SeqCst);
        Counted(v)
    }
}

impl Clone for Counted {
    fn clone(&self) -> Self {
        Counted::new(self.0)
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        LIVE.fetch_sub(1, Ordering::SeqCst);
    }
}

impl TupleSummary for Counted {
    type Update = i64;
    fn create(update: &i64) -> Self {
        Counted::new(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sketch(keys: std::ops::Range<u64>) -> TupleSketch<Counted> {
    let mut s: TupleSketch<Counted> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &1);
    }
    s
}

/// Runs `body`, then asserts every summary it created has been dropped.
fn assert_balanced(body: impl FnOnce()) {
    let _guard = lock();
    LIVE.store(0, Ordering::SeqCst);
    body();
    assert_eq!(
        LIVE.load(Ordering::SeqCst),
        0,
        "summaries created but never dropped (negative means double-drop)"
    );
}

#[test]
fn plain_updates_balance() {
    assert_balanced(|| {
        let _ = sketch(0..1000);
    });
}

#[test]
fn table_resize_past_k_balances() {
    // 50k keys against the default k = 4096 forces many rehash/resize cycles,
    // which is the mass-move path.
    assert_balanced(|| {
        let _ = sketch(0..50_000);
    });
}

#[test]
fn compact_balances() {
    assert_balanced(|| {
        let s = sketch(0..5_000);
        let _c = s.compact(true);
    });
}

#[test]
fn entries_iteration_balances() {
    assert_balanced(|| {
        let c = sketch(0..2_000).compact(true);
        let total: i64 = c.entries().map(|(_, s)| s.0).sum();
        assert!(total > 0);
    });
}

#[test]
fn union_balances() {
    assert_balanced(|| {
        let mut u = TupleUnionBuilder::<Counted>::new().build().unwrap();
        u.update(&sketch(0..10_000));
        u.update(&sketch(5_000..15_000));
        let _ = u.get_result(true);
    });
}

#[test]
fn intersection_balances() {
    assert_balanced(|| {
        let mut i = TupleIntersection::<Counted>::new();
        i.update(&sketch(0..10_000));
        i.update(&sketch(5_000..15_000));
        let _ = i.get_result(true).unwrap();
    });
}

#[test]
fn a_not_b_balances() {
    assert_balanced(|| {
        let calc = TupleAnotB::<Counted>::new();
        let _ = calc.compute(&sketch(0..10_000), &sketch(5_000..15_000), true);
    });
}
```

Register as `tuple_generic_ownership_test`, `required-features = ["tuple"]`.

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_ownership_test -- --test-threads=1`
Expected: PASS (7 tests). **A non-zero balance is a real leak or double-free — report it rather than relaxing the assertion.**

- [ ] **Step 3: Four kinds of summary**

Create `apache-datasketches/tests/tuple_generic_summary_kinds_test.rs`, exercising the trait's range and the estimation-mode gap the ArrayOfDoubles family had to retrofit.

```rust
//! Four summary shapes, each stressing something different, plus
//! estimation-mode coverage for every set operation from the outset.

use apache_datasketches::tuple::generic::{
    tuple_jaccard_similarity, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnionBuilder,
};

/// 1. Trivial Copy summary.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Count(u64);

impl TupleSummary for Count {
    type Update = ();
    fn create(_: &()) -> Self {
        Count(1)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
}

/// 2. Heap-owning summary.
#[derive(Clone, Debug, PartialEq)]
struct Tags(Vec<String>);

impl TupleSummary for Tags {
    type Update = str;
    fn create(update: &str) -> Self {
        Tags(vec![update.to_string()])
    }
    fn union_combine(&mut self, other: &Self) {
        self.0.extend(other.0.iter().cloned());
        self.0.sort();
        self.0.dedup();
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0.retain(|t| other.0.contains(t));
    }
}

/// 3. Unsized Update that differs from the summary type.
#[derive(Clone, Debug, PartialEq)]
struct LenHistogram([u32; 4]);

impl TupleSummary for LenHistogram {
    type Update = str;
    fn create(update: &str) -> Self {
        let mut h = [0u32; 4];
        h[update.len().min(3)] = 1;
        LenHistogram(h)
    }
    fn union_combine(&mut self, other: &Self) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a += *b;
        }
    }
    fn intersection_combine(&mut self, other: &Self) {
        for (a, b) in self.0.iter_mut().zip(other.0.iter()) {
            *a = (*a).min(*b);
        }
    }
}

/// 4. Union and intersection semantics genuinely differ, so a cross-wired
///    trampoline is detectable.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SumOrMin(i64);

impl TupleSummary for SumOrMin {
    type Update = i64;
    fn create(update: &i64) -> Self {
        SumOrMin(*update)
    }
    fn union_combine(&mut self, other: &Self) {
        self.0 += other.0;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.0 = self.0.min(other.0);
    }
}

fn sum_or_min(keys: std::ops::Range<u64>, v: i64) -> TupleSketch<SumOrMin> {
    let mut s: TupleSketch<SumOrMin> = TupleSketchBuilder::new().build().unwrap();
    for key in keys {
        s.update_u64(key, &v);
    }
    s
}

#[test]
fn copy_summary_counts_occurrences() {
    let mut s: TupleSketch<Count> = TupleSketchBuilder::new().build().unwrap();
    for _ in 0..7 {
        s.update_u64(1, &());
    }
    let entries: Vec<(u64, Count)> = s.compact(true).entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1, Count(7));
}

#[test]
fn heap_owning_summary_survives_round_trip() {
    let mut s: TupleSketch<Tags> = TupleSketchBuilder::new().build().unwrap();
    s.update_u64(1, "alpha");
    s.update_u64(1, "beta");
    s.update_u64(2, "gamma");
    let mut entries: Vec<(u64, Tags)> = s.compact(true).entries().collect();
    entries.sort_by_key(|(_, t)| t.0.len());
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|(_, t)| t.0 == vec!["alpha".to_string(), "beta".to_string()]));
    assert!(entries.iter().any(|(_, t)| t.0 == vec!["gamma".to_string()]));
}

#[test]
fn unsized_update_type_works() {
    let mut s: TupleSketch<LenHistogram> = TupleSketchBuilder::new().build().unwrap();
    s.update_u64(1, "ab");
    s.update_u64(1, "xy");
    let entries: Vec<(u64, LenHistogram)> = s.compact(true).entries().collect();
    assert_eq!(entries[0].1, LenHistogram([0, 0, 2, 0]));
}

#[test]
fn union_and_intersection_use_different_trampolines() {
    let a = sum_or_min(7..8, 10);
    let b = sum_or_min(7..8, 32);

    let mut u = TupleUnionBuilder::<SumOrMin>::new().build().unwrap();
    u.update(&a);
    u.update(&b);
    let unioned: Vec<(u64, SumOrMin)> = u.get_result(true).entries().collect();
    assert_eq!(unioned[0].1, SumOrMin(42), "union must sum");

    let mut i = TupleIntersection::<SumOrMin>::new();
    i.update(&a);
    i.update(&b);
    let intersected: Vec<(u64, SumOrMin)> = i.get_result(true).unwrap().entries().collect();
    assert_eq!(intersected[0].1, SumOrMin(10), "intersection must take the min");
}

#[test]
fn union_in_estimation_mode() {
    let mut u = TupleUnionBuilder::<SumOrMin>::new().build().unwrap();
    u.update(&sum_or_min(0..30_000, 1));
    u.update(&sum_or_min(15_000..45_000, 1));
    let result = u.get_result(true);
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 45_000.0).abs() < 45_000.0 * 0.03);
}

#[test]
fn intersection_in_estimation_mode() {
    let mut i = TupleIntersection::<SumOrMin>::new();
    i.update(&sum_or_min(0..30_000, 1));
    i.update(&sum_or_min(15_000..45_000, 1));
    let result = i.get_result(true).unwrap();
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 15_000.0).abs() < 15_000.0 * 0.05);
}

#[test]
fn a_not_b_in_estimation_mode() {
    let calc = TupleAnotB::<SumOrMin>::new();
    let result = calc.compute(&sum_or_min(0..30_000, 1), &sum_or_min(15_000..45_000, 1), true);
    assert!(result.is_estimation_mode());
    assert!((result.get_estimate() - 15_000.0).abs() < 15_000.0 * 0.05);
}

#[test]
fn jaccard_in_estimation_mode() {
    let bounds = tuple_jaccard_similarity(
        &sum_or_min(0..30_000, 1),
        &sum_or_min(15_000..45_000, 1),
    );
    assert!((bounds.estimate - 1.0 / 3.0).abs() < 0.05);
    // Non-degenerate interval: this holds only in estimation mode.
    assert!(bounds.lower_bound < bounds.upper_bound);
}
```

Register as `tuple_generic_summary_kinds_test`, `required-features = ["tuple"]`.

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_summary_kinds_test`
Expected: PASS (8 tests).

- [ ] **Step 4: Panic containment**

Create `apache-datasketches/tests/tuple_generic_panic_test.rs`. We claim a clean abort with a diagnostic; without this test that claim is untested prose.

```rust
//! A panic inside a trampolined TupleSummary method must abort the process
//! with a message naming the method, rather than unwinding into C++.
//!
//! Verified by re-invoking this test binary as a child process, because the
//! behaviour under test terminates the process.

use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};

const CHILD_ENV: &str = "TUPLE_GENERIC_PANIC_CHILD";

#[derive(Clone, Debug)]
struct Exploding;

impl TupleSummary for Exploding {
    type Update = ();
    fn create(_: &()) -> Self {
        Exploding
    }
    fn union_combine(&mut self, _other: &Self) {
        panic!("deliberate panic from union_combine");
    }
    fn intersection_combine(&mut self, _other: &Self) {
        panic!("deliberate panic from intersection_combine");
    }
}

#[test]
fn panicking_union_combine_aborts_with_a_diagnostic() {
    if std::env::var(CHILD_ENV).is_ok() {
        let mut s: TupleSketch<Exploding> = TupleSketchBuilder::new().build().unwrap();
        // First update inserts (clone path); the second hits the same key and
        // therefore calls union_combine from C++.
        s.update_u64(1, &());
        s.update_u64(1, &());
        unreachable!("the second update should have aborted the process");
    }

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "panicking_union_combine_aborts_with_a_diagnostic", "--nocapture"])
        .env(CHILD_ENV, "1")
        .output()
        .expect("failed to spawn the child test process");

    assert!(
        !output.status.success(),
        "child was expected to abort, but exited successfully"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("TupleSummary::union_combine"),
        "abort message should name the panicking method; stderr was:\n{stderr}"
    );
}

/// `create` runs entirely Rust-side before any C++ call, so a panic there is
/// an ordinary catchable Rust panic — the documented contrast with the three
/// trampolined methods.
#[test]
fn panicking_create_is_an_ordinary_rust_panic() {
    #[derive(Clone, Debug)]
    struct PanicsOnCreate;

    impl TupleSummary for PanicsOnCreate {
        type Update = ();
        fn create(_: &()) -> Self {
            panic!("deliberate panic from create");
        }
        fn union_combine(&mut self, _: &Self) {}
        fn intersection_combine(&mut self, _: &Self) {}
    }

    let mut s: TupleSketch<PanicsOnCreate> = TupleSketchBuilder::new().build().unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        s.update_u64(1, &());
    }));
    assert!(result.is_err(), "a panic in create must be catchable");
}
```

Register as `tuple_generic_panic_test`, `required-features = ["tuple"]`.

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_panic_test`
Expected: PASS (2 tests). The child process's abort output appears in the log; that is expected.

- [ ] **Step 5: Cross-thread concurrency**

The per-component smoke tests each assert `Send` at compile time, but nothing actually moves a generic sketch between threads. Since the summary is a boxed trait object owned by C++, a `Send` bound that compiles but is unsound would show up only at runtime.

Create `apache-datasketches/tests/tuple_generic_concurrency_test.rs`:

```rust
//! Every generic Tuple type is `Send` but not `Sync`, matching the rest of
//! the crate. These tests move real sketches across threads rather than only
//! asserting the bound compiles.

use apache_datasketches::tuple::generic::{
    CompactTupleSketch, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnion, TupleUnionBuilder,
};

#[derive(Clone, Debug, PartialEq)]
struct Tally {
    hits: u64,
    label: String,
}

impl TupleSummary for Tally {
    type Update = str;
    fn create(update: &str) -> Self {
        Tally {
            hits: 1,
            label: update.to_string(),
        }
    }
    fn union_combine(&mut self, other: &Self) {
        self.hits += other.hits;
    }
    fn intersection_combine(&mut self, other: &Self) {
        self.hits = self.hits.min(other.hits);
    }
}

#[test]
fn all_types_are_send() {
    fn assert_send<T: Send>() {}
    assert_send::<TupleSketch<Tally>>();
    assert_send::<CompactTupleSketch<Tally>>();
    assert_send::<TupleUnion<Tally>>();
    assert_send::<TupleIntersection<Tally>>();
    assert_send::<TupleAnotB<Tally>>();
}

#[test]
fn a_sketch_built_on_one_thread_is_usable_on_another() {
    let handle = std::thread::spawn(|| {
        let mut sketch: TupleSketch<Tally> = TupleSketchBuilder::new().build().unwrap();
        for i in 0..1_000u64 {
            sketch.update_u64(i, "worker");
        }
        sketch.compact(true)
    });
    let compact = handle.join().unwrap();

    // The heap-owning summaries must survive the move intact.
    assert_eq!(compact.entries().count(), 1_000);
    assert!(compact.entries().all(|(_, t)| t == Tally { hits: 1, label: "worker".to_string() }));
}

#[test]
fn per_thread_sketches_merge_correctly() {
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            std::thread::spawn(move || {
                let mut sketch: TupleSketch<Tally> = TupleSketchBuilder::new().build().unwrap();
                for i in (t * 2_500)..((t + 1) * 2_500) {
                    sketch.update_u64(i, "shard");
                }
                sketch.compact(true)
            })
        })
        .collect();

    let mut union = TupleUnionBuilder::<Tally>::new().build().unwrap();
    for handle in handles {
        union.update(&handle.join().unwrap());
    }
    let result = union.get_result(true);
    assert!((result.get_estimate() - 10_000.0).abs() < 10_000.0 * 0.03);
}
```

Register as `tuple_generic_concurrency_test`, `required-features = ["tuple"]`.

Run: `cargo test -p apache-datasketches --features tuple --test tuple_generic_concurrency_test`
Expected: PASS (3 tests).

- [ ] **Step 6: Run everything, then commit**

Run: `cargo test --workspace --all-features -- --test-threads=1`
Expected: all pass. (Single-threaded because of the ownership test's global counters.)

```bash
git add apache-datasketches/tests apache-datasketches/Cargo.toml
git commit -m "test(tuple): add generic conformance, ownership, summary-kind, panic and concurrency tests"
```

---

### Task 9: Example, documentation, and full-matrix verification

**Files:**
- Create: `apache-datasketches/examples/tuple_generic.rs`
- Modify: `apache-datasketches/Cargo.toml` (`[[example]]`), `apache-datasketches/src/lib.rs`, `README.md`, `apache-datasketches/README.md`, `AGENTS.md`

**Interfaces:** consumes the complete public API. Produces no library API.

- [ ] **Step 1: Write the example**

Create `apache-datasketches/examples/tuple_generic.rs`:

```rust
//! Demonstrates generic Tuple sketches: cardinality estimation where each
//! distinct key carries a summary of a type you define in Rust.
//!
//! Run with:
//!   cargo run --example tuple_generic --features tuple

use apache_datasketches::tuple::generic::{
    tuple_jaccard_similarity, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnionBuilder,
};

/// Per-user session statistics. This is the sort of summary the fixed
/// `f64`-array shape of `ArrayOfDoublesSketch` cannot express: it mixes
/// counters with a max and a set of strings.
#[derive(Clone, Debug)]
struct Activity {
    sessions: u32,
    revenue_cents: u64,
    largest_order_cents: u64,
    countries: Vec<String>,
}

/// What a single event contributes.
struct Event<'a> {
    revenue_cents: u64,
    country: &'a str,
}

impl TupleSummary for Activity {
    type Update = Event<'static>;

    fn create(event: &Event<'static>) -> Self {
        Activity {
            sessions: 1,
            revenue_cents: event.revenue_cents,
            largest_order_cents: event.revenue_cents,
            countries: vec![event.country.to_string()],
        }
    }

    fn union_combine(&mut self, other: &Self) {
        self.sessions += other.sessions;
        self.revenue_cents += other.revenue_cents;
        self.largest_order_cents = self.largest_order_cents.max(other.largest_order_cents);
        self.countries.extend(other.countries.iter().cloned());
        self.countries.sort();
        self.countries.dedup();
    }

    fn intersection_combine(&mut self, other: &Self) {
        // For an intersection we want only what both sides saw.
        self.sessions = self.sessions.min(other.sessions);
        self.revenue_cents = self.revenue_cents.min(other.revenue_cents);
        self.largest_order_cents = self.largest_order_cents.min(other.largest_order_cents);
        self.countries.retain(|c| other.countries.contains(c));
    }
}

fn main() {
    let mut january: TupleSketch<Activity> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    for user in 0..10_000u64 {
        january.update_u64(
            user,
            &Event {
                revenue_cents: 250 + (user % 100),
                country: if user % 2 == 0 { "GB" } else { "US" },
            },
        );
    }

    let mut february: TupleSketch<Activity> = TupleSketchBuilder::new().lg_k(12).build().unwrap();
    for user in 5_000..15_000u64 {
        february.update_u64(
            user,
            &Event {
                revenue_cents: 400,
                country: "US",
            },
        );
    }

    println!("January unique users:  {:.0}", january.get_estimate());
    println!("February unique users: {:.0}", february.get_estimate());

    // Union: everyone who appeared in either month, with their activity merged.
    let mut union = TupleUnionBuilder::<Activity>::new().lg_k(12).build().unwrap();
    union.update(&january);
    union.update(&february);
    let combined = union.get_result(true);
    println!("Users across both months: {:.0}", combined.get_estimate());

    // Per-entry summaries are the point of a Tuple sketch. Scale the retained
    // sample back up by 1/theta to estimate population totals.
    let retained_revenue: u64 = combined.entries().map(|(_, a)| a.revenue_cents).sum();
    let biggest_order = combined
        .entries()
        .map(|(_, a)| a.largest_order_cents)
        .max()
        .unwrap_or(0);
    println!(
        "Estimated total revenue: {:.2} (from {} retained entries, theta = {:.4})",
        (retained_revenue as f64 / combined.get_theta()) / 100.0,
        combined.get_num_retained(),
        combined.get_theta()
    );
    println!("Largest single order seen: {:.2}", biggest_order as f64 / 100.0);

    // Intersection: users active in both months.
    let mut intersection = TupleIntersection::<Activity>::new();
    intersection.update(&january);
    intersection.update(&february);
    match intersection.get_result(true) {
        Ok(returning) => println!("Returning users: {:.0}", returning.get_estimate()),
        Err(e) => println!("No intersection result: {e}"),
    }

    // A-not-b: users who churned after January.
    let churned = TupleAnotB::<Activity>::new().compute(&january, &february, true);
    println!("Churned after January: {:.0}", churned.get_estimate());

    // Jaccard similarity of the two months' audiences.
    let similarity = tuple_jaccard_similarity(&january, &february);
    println!(
        "Audience overlap (Jaccard): {:.3} (range [{:.3}, {:.3}])",
        similarity.estimate, similarity.lower_bound, similarity.upper_bound
    );
}
```

Register it:

```toml
[[example]]
name = "tuple_generic"
required-features = ["tuple"]
```

- [ ] **Step 2: Run the example**

Run: `cargo run -p apache-datasketches --example tuple_generic --features tuple`
Expected: prints without panicking. January and February each around `10000`, union around `15000`, returning users around `5000`, churned around `5000`, Jaccard around `0.333`.

- [ ] **Step 3: Update the crate-level docs**

In `apache-datasketches/src/lib.rs`, extend the `tuple` feature bullet so it mentions both shapes:

```rust
//! - `tuple` (feature `tuple`) — Tuple sketches, in two shapes. The
//!   ArrayOfDoubles form carries a fixed-width array of `f64` per distinct
//!   key (summed on collision); the generic form in `tuple::generic` carries
//!   a summary type you define in Rust. Both support union, intersection,
//!   a-not-b, and Jaccard similarity.
```

- [ ] **Step 4: Update both READMEs**

In `apache-datasketches/README.md`, add a subsection to the existing "Tuple sketches" section, after the ArrayOfDoubles API bullet list:

```markdown
### Generic summaries

When a fixed array of `f64` is the wrong shape, implement `TupleSummary` on
your own type and use `tuple::generic::TupleSketch<S>`:

```rust
use apache_datasketches::tuple::generic::{TupleSketch, TupleSketchBuilder, TupleSummary};

#[derive(Clone)]
struct Count(u64);

impl TupleSummary for Count {
    type Update = ();
    fn create(_: &()) -> Self { Count(1) }
    fn union_combine(&mut self, other: &Self) { self.0 += other.0; }
    fn intersection_combine(&mut self, other: &Self) { self.0 += other.0; }
}

let mut sketch: TupleSketch<Count> = TupleSketchBuilder::new().build()?;
sketch.update_u64(42, &());
```

`TupleUnion<S>`, `TupleIntersection<S>`, `TupleAnotB<S>`, and
`tuple_jaccard_similarity` mirror their ArrayOfDoubles counterparts, and
`CompactTupleSketch<S>::entries()` yields `(hash, S)` pairs.

C++ calls back into Rust to clone and combine summaries. A panic in
`union_combine`, `intersection_combine`, or `Clone::clone` aborts the process
— panics cannot cross the FFI boundary — so make those total. A panic in
`create` is an ordinary Rust panic. Serialization of generic sketches is not
supported yet; use `ArrayOfDoublesSketch` if you need to persist a sketch.

See `examples/tuple_generic.rs` for a complete runnable demo.
```

In the root `README.md`, extend the `tuple` checklist entry to mention both shapes in one sentence, matching the surrounding style.

- [ ] **Step 5: Update `AGENTS.md`**

Add a short paragraph to the "Layering" section noting that `tuple` now contains a second, type-erased family whose C++ layer calls back into Rust, that its `extern "Rust"` opaque type may be declared in only one bridge (so the sketch and compact bridges share `src/tuple_generic.rs`), and that shim headers must forward-declare the trampolines rather than include the generated header. Keep it to the file's existing voice and length.

- [ ] **Step 6: Verify the whole matrix**

Run each and expect success with no warnings:

```bash
cargo build --workspace
cargo build --workspace --features apache-datasketches/tuple
cargo build --workspace --all-features
cargo test --workspace --all-features -- --test-threads=1
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo doc --workspace --all-features --no-deps
cargo package -p apache-datasketches-sys --allow-dirty --no-verify
```

The `--all-features` build matters most: it is the only configuration where every bridge compiles together, which is what `build.rs`'s duplicate-name check and the `tuple_generic_*` prefixes exist to keep working.

Note `cargo fmt --all -- --check` is expected to report the four pre-existing cross-product FFI table files and nothing else. If it reports any file you touched, run `cargo fmt` on that file only.

- [ ] **Step 7: Commit**

```bash
git add apache-datasketches/examples/tuple_generic.rs apache-datasketches/Cargo.toml apache-datasketches/src/lib.rs README.md apache-datasketches/README.md AGENTS.md
git commit -m "docs(tuple): add generic Tuple example and document the feature"
```

---

## Task Summary

| Task | Deliverable |
|------|-------------|
| 1 | `extern "Rust"` summary bridge, `DynSummary`, the three policies |
| 2 | `TupleSummary` trait, erasure adapter, `TupleSketch<S>` + builder |
| 3 | `CompactTupleSketch<S>`, `compact()`, `entries()` |
| 4 | Input dispatch + `TupleUnion<S>` |
| 5 | `TupleIntersection<S>` |
| 6 | `TupleAnotB<S>` |
| 7 | `tuple_jaccard_similarity<S>` |
| 8 | Conformance, ownership accounting, summary kinds, panic containment |
| 9 | Example, docs, full-matrix verification |
