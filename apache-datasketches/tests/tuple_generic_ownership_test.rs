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
//! Two of the counters double as path discriminators, not just balance
//! checks: `CREATED` is the positive control that proves a body actually
//! created summaries at all (a balance of zero is otherwise ambiguous between
//! "everything was dropped" and "nothing was ever created"), and `CLONES` is
//! what `self_jaccard_short_circuits_before_cloning_any_summary` uses to tell
//! the pointer-identity early return apart from the ordinary scratch-copy
//! path, since both produce identical `{1,1,1}` bounds.

use apache_datasketches::tuple::generic::{
    tuple_jaccard_similarity, TupleAnotB, TupleIntersection, TupleSketch, TupleSketchBuilder,
    TupleSummary, TupleUnionBuilder,
};
use apache_datasketches::tuple::JaccardBounds;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

static LIVE: AtomicI64 = AtomicI64::new(0);
/// Counts only `Clone::clone`, which is what the C++ side reaches through the
/// `rust_summary_clone` trampoline. `create` does not bump it.
static CLONES: AtomicI64 = AtomicI64::new(0);
/// Bumped by every `Counted::new` call (both direct `create`s and the ones
/// `Clone::clone` makes). The positive control for `assert_balanced`: a
/// balance of zero is consistent both with "everything was dropped" and with
/// "nothing was ever created", and only this counter tells the two apart.
static CREATED: AtomicI64 = AtomicI64::new(0);

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
        CREATED.fetch_add(1, Ordering::SeqCst);
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
/// Also asserts `CREATED > 0` as a positive control: without it, a body that
/// creates zero summaries (because `update_u64` silently no-ops, the builder
/// drops every update, or the clone trampoline is never reached at all) would
/// still pass the balance assertion below with `LIVE == 0`, since "everything
/// was dropped" and "nothing was ever created" are indistinguishable from
/// `LIVE` alone.
fn assert_balanced(body: impl FnOnce()) {
    let _guard = lock();
    LIVE.store(0, Ordering::SeqCst);
    CREATED.store(0, Ordering::SeqCst);
    body();
    let live = LIVE.load(Ordering::SeqCst);
    assert_eq!(
        live, 0,
        "summaries created but never dropped (negative means double-drop): {live}"
    );
    let created = CREATED.load(Ordering::SeqCst);
    assert!(
        created > 0,
        "no summaries were ever created -- the balance assertion above is \
         vacuous without this control; created = {created}"
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
