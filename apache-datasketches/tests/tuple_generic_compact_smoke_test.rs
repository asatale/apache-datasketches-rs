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
    // Each key gets a distinct summary value (key * 7), not a shared
    // constant: compaction is the first thing that exercises DynSummary's
    // copy constructor and the clone trampoline, so a summary getting
    // shuffled onto the wrong entry is exactly the new risk this task
    // introduces, and a constant value across every entry cannot detect it.
    let mut s: TupleSketch<Sum> = TupleSketchBuilder::new().build().unwrap();
    for key in 0..500u64 {
        s.update_u64(key, &(key as i64 * 7));
    }
    let compact = s.compact(true);
    assert!((compact.get_estimate() - 500.0).abs() < 1.0);
    assert_eq!(compact.get_num_retained(), 500);
    assert!(compact.is_ordered());

    // Hash order (murmur3) does not let us recover which key produced which
    // hash, but the *multiset* of summary values must survive compaction
    // unchanged. `got.len()` also pins `entries()`'s yield count against
    // `get_num_retained()`, an independently-computed reading, so neither
    // can silently under/over-report alone.
    let mut got: Vec<Sum> = compact.entries().map(|(_, s)| s).collect();
    assert_eq!(got.len(), compact.get_num_retained() as usize);
    got.sort_by_key(|s| s.0);
    let mut expected: Vec<Sum> = (0..500u64).map(|key| Sum(key as i64 * 7)).collect();
    expected.sort_by_key(|s| s.0);
    assert_eq!(got, expected);
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

/// Probe carrier for the `Sync`-detection trick below. `PhantomData<T>` keeps
/// it constructible for any `T`, including non-`Sync` ones.
struct SyncProbe<T>(std::marker::PhantomData<T>);

/// Specialised arm: only applicable when `T: Sync`.
trait ProbeViaSync {
    fn is_sync(&self) -> bool;
}
impl<T: Sync> ProbeViaSync for &SyncProbe<T> {
    fn is_sync(&self) -> bool {
        true
    }
}

/// Fallback arm: applicable for every `T`, but one autoref step further away,
/// so the compiler only reaches it when the specialised arm does not apply.
trait ProbeViaFallback {
    fn is_sync(&self) -> bool;
}
impl<T> ProbeViaFallback for SyncProbe<T> {
    fn is_sync(&self) -> bool {
        false
    }
}

/// `CompactTupleSketch<S>` must NOT be `Sync`: the C++ shim lazily populates a
/// `mutable` entry cache (`entries_`/`entries_built_` in
/// `tuple_generic_compact_shim.h`) from otherwise-`const` methods, so
/// concurrent `&`-access to one instance would be a data race. A plain
/// `fn assert_sync<T: Sync>()` cannot express the negative, so this uses
/// autoref specialization: method resolution on `&&SyncProbe<T>` picks
/// `ProbeViaSync` (returning `true`) when `T: Sync` holds, and only falls
/// through to `ProbeViaFallback` (returning `false`) when it does not. Adding
/// `unsafe impl Sync for CompactTupleSketch` makes this test fail.
#[test]
fn compact_is_not_sync() {
    let probe = SyncProbe::<CompactTupleSketch<Sum>>(std::marker::PhantomData);
    // The double borrow IS load-bearing: with receiver `&SyncProbe<T>`, the
    // candidate-receiver list starts with `&SyncProbe<T>` itself, which
    // already matches `ProbeViaFallback::is_sync(&self)` (`Self =
    // SyncProbe<T>`), so a single borrow resolves there immediately and the
    // probe would degenerate into an always-`false` constant, never reaching
    // `ProbeViaSync`. The double borrow's candidate list reaches
    // `&&SyncProbe<T>` first, which matches `ProbeViaSync::is_sync(&self)`
    // (`Self = &SyncProbe<T>`) whenever `T: Sync` holds. The positive control
    // below (the `u64` case) is what proves this empirically: it asserts a
    // known-`Sync` type probes as `Sync`.
    #[allow(clippy::needless_borrow)]
    let is_sync = (&&probe).is_sync();
    assert!(
        !is_sync,
        "CompactTupleSketch must not be Sync -- the shim's lazily built entry \
         cache makes concurrent &-access a data race"
    );

    // Sanity check that the probe reports `true` for a type that really is
    // `Sync`; without this, a probe that always answered `false` would pass
    // the assertion above vacuously.
    let sync_probe = SyncProbe::<u64>(std::marker::PhantomData);
    #[allow(clippy::needless_borrow)]
    let u64_is_sync = (&&sync_probe).is_sync();
    assert!(
        u64_is_sync,
        "probe is broken: it does not detect Sync at all"
    );
}
