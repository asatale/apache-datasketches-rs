//! Verifies that every summary C++ clones is eventually dropped exactly once.
//!
//! No other test in the suite can see this: a leak or a double-free in the
//! `rust::Box` ownership dance is completely silent from the outside, and Miri
//! cannot help because it does not execute C++ FFI. A counting summary is the
//! only instrument that observes it.
//!
//! The counters are process-global, so these tests must not run concurrently
//! with each other -- they are serialised through a mutex rather than split
//! across test binaries.
//!
//! `CLONES` does double duty, not just balance-checking: it is both the
//! positive control that proves a body actually drove C++ to call back into
//! Rust at all (see `assert_balanced`), and what
//! `self_jaccard_short_circuits_before_cloning_any_summary` uses to tell the
//! pointer-identity early return apart from the ordinary scratch-copy path,
//! since both produce identical `{1,1,1}` bounds.
//!
//! The positive control is deliberately built on `CLONES`, not on a counter
//! bumped in `Counted::create`. Every `S::create` in this crate's
//! `TupleSketch::update_*` runs entirely Rust-side, before anything crosses
//! the FFI boundary (`generic/sketch.rs`'s `update_u64` and friends call
//! `S::create` and only then call into C++); a `create`-side counter would
//! stay positive even if the sketch update, the builder, or the clone
//! trampoline never reached C++ at all, because the temporary would still
//! have been created and dropped on the Rust side. `CLONES` cannot be bumped
//! that way -- it is bumped only inside `Clone::clone`, and the only path
//! C++ has to reach that method is the `rust_summary_clone` trampoline. Every
//! update in this file's tests crosses that trampoline at least once: a
//! brand-new key is disengaged going in, so `DynUpdatePolicy::update`
//! (`dyn_summary.h`) takes the `assign_clone_of` arm, which clones. So
//! `CLONES > 0` after a body runs is proof C++ actually called back into
//! Rust -- not merely that `S::create` ran.
//!
//! Note what that rests on: **inserting a new key**. Updating a key already in
//! the sketch clones nothing — the shim passes a borrowed `RustSummary` and
//! `DynUpdatePolicy::update` combines into the retained summary in place. A
//! body that only re-updates existing keys would therefore leave `CLONES == 0`
//! and trip the positive control for a reason that has nothing to do with the
//! bug it is guarding. Give every new test body at least one fresh key. (This
//! used to be free, because the shim wrapped every update value in a
//! `DynSummary` and so cloned on every call regardless — see
//! `update_clones_once_for_a_new_key_and_never_for_an_existing_one`.)

use apache_datasketches::tuple::generic::{
    tuple_jaccard_similarity, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnionBuilder,
};
use apache_datasketches::tuple::JaccardBounds;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

static LIVE: AtomicI64 = AtomicI64::new(0);
/// Counts only `Clone::clone`, which is what the C++ side reaches through the
/// `rust_summary_clone` trampoline. `create` does not bump it. This is also
/// the positive control for `assert_balanced` -- see the file header for why
/// a `create`-side counter would not do.
static CLONES: AtomicI64 = AtomicI64::new(0);

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
        CLONES.fetch_add(1, Ordering::SeqCst);
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
///
/// Also asserts `CLONES > 0` as a positive control: without it, a body whose
/// `update_u64` silently no-ops, whose builder drops every update, or whose
/// path into C++ never reaches the clone trampoline at all would still pass
/// the balance assertion below with `LIVE == 0`, since "everything was
/// dropped" and "nothing ever crossed into C++" are indistinguishable from
/// `LIVE` alone. `CLONES` closes that gap because it can only be bumped from
/// inside `Clone::clone`, which C++ reaches solely through
/// `rust_summary_clone` -- so `CLONES > 0` is proof C++ called back into
/// Rust, not merely that a Rust-side `S::create` ran. See the file header for
/// why a `create`-side counter would not have this property.
///
/// `self_jaccard_short_circuits_before_cloning_any_summary` resets and reads
/// `CLONES` itself mid-body to compare the pointer-identity early return
/// against the ordinary path; that is compatible with this control because
/// the check here only looks at `CLONES`'s value once `body` has fully
/// returned. That test's last act that touches `CLONES` is the "two separate
/// sketches" `tuple_jaccard_similarity` call, which clones every retained
/// summary; the assertions and drops that follow it inside the body do not
/// touch `CLONES`, so it is left positive regardless of the test's internal
/// resets.
fn assert_balanced(body: impl FnOnce()) {
    let _guard = lock();
    LIVE.store(0, Ordering::SeqCst);
    CLONES.store(0, Ordering::SeqCst);
    body();
    let live = LIVE.load(Ordering::SeqCst);
    assert_eq!(
        live, 0,
        "summaries created but never dropped (negative means double-drop): {live}"
    );
    let clones = CLONES.load(Ordering::SeqCst);
    assert!(
        clones > 0,
        "no summary was ever cloned across the FFI boundary -- the balance \
         assertion above is vacuous without this control, since it cannot \
         tell \"nothing ever reached C++\" apart from \"everything was \
         dropped\"; clones = {clones}"
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

#[test]
fn jaccard_balances() {
    assert_balanced(|| {
        let _ = tuple_jaccard_similarity(&sketch(0..10_000), &sketch(5_000..15_000));
    });
}

/// `jaccard()` opens with `reinterpret_cast<const void*>(&a) == &b`, so passing
/// the same sketch twice returns `{1,1,1}` without building the scratch union
/// and intersection.
///
/// Lives here rather than in the summary-kinds file because the assertion that
/// makes it non-vacuous needs the clone counter: two equal-but-*separate*
/// sketches also return `{1,1,1}`, via the later `identical_sets` check, so the
/// bounds alone cannot tell the two paths apart. The clone count can -- the
/// early return performs zero, the separate-sketch path thousands.
#[test]
fn self_jaccard_short_circuits_before_cloning_any_summary() {
    assert_balanced(|| {
        let a = sketch(0..2_000);
        let b = sketch(0..2_000);
        let expected = JaccardBounds {
            lower_bound: 1.0,
            estimate: 1.0,
            upper_bound: 1.0,
        };

        CLONES.store(0, Ordering::SeqCst);
        let self_bounds = tuple_jaccard_similarity(&a, &a);
        let self_clones = CLONES.load(Ordering::SeqCst);

        CLONES.store(0, Ordering::SeqCst);
        let pair_bounds = tuple_jaccard_similarity(&a, &b);
        let pair_clones = CLONES.load(Ordering::SeqCst);

        assert_eq!(self_bounds, expected);
        assert_eq!(pair_bounds, expected);
        assert_eq!(
            self_clones, 0,
            "self-jaccard must take the pointer-identity early return, \
             which clones nothing"
        );
        assert!(
            pair_clones > 0,
            "two separate sketches must go through the scratch union, \
             which clones every retained summary"
        );
    });
}

/// Pins the clone count of the update path itself, which is the thing the
/// shim's borrow optimisation buys and the thing a regression would undo.
///
/// The shim hands C++ a borrowed `const RustSummary&` and lets
/// `DynUpdatePolicy::update` decide whether to clone. So:
///
/// - a brand-new key clones exactly once, to populate the new entry;
/// - a key already present clones **zero** times — it goes straight to
///   `union_combine` on the retained summary.
///
/// Before that change the shim wrapped every update value in a `DynSummary`
/// via `assign_clone_of`, so these numbers were 2 and 1 respectively: an
/// update to an existing key allocated and dropped a summary for nothing, and
/// a theta-rejected key paid for a clone that upstream never even looked at
/// (`update_tuple_sketch::update` screens the key before reading the value).
///
/// Exact equality rather than an upper bound: an off-by-one here is precisely
/// the regression worth catching, and the counts are deterministic — two keys
/// at the default `lg_k` are nowhere near theta, so neither update is screened.
#[test]
fn update_clones_once_for_a_new_key_and_never_for_an_existing_one() {
    assert_balanced(|| {
        let mut s: TupleSketch<Counted> = TupleSketchBuilder::new().build().unwrap();

        s.update_u64(1, &1);

        CLONES.store(0, Ordering::SeqCst);
        s.update_u64(1, &1);
        let existing_key_clones = CLONES.load(Ordering::SeqCst);

        // Done last so the harness's `CLONES > 0` positive control still holds
        // when the body returns.
        CLONES.store(0, Ordering::SeqCst);
        s.update_u64(2, &1);
        let new_key_clones = CLONES.load(Ordering::SeqCst);

        assert_eq!(
            existing_key_clones, 0,
            "updating a key already in the sketch must not clone -- it combines \
             into the retained summary in place"
        );
        assert_eq!(
            new_key_clones, 1,
            "a brand-new key must clone exactly once, to populate its entry"
        );
        assert_eq!(s.get_num_retained(), 2);
    });
}
