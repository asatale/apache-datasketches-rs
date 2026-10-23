//! Deterministic mutation fuzzing of `ReqSketch::deserialize`.
//!
//! Genuine serialized sketches are mutated at random (fixed seed, so every
//! run checks the same inputs). Any buffer that deserializes must then
//! survive every query and mutation without panicking or aborting: upstream
//! trusts its input, so anything the shim's validator lets through reaches
//! upstream's merge, compaction and query code as-is.

#[macro_use]
mod req_common;

use apache_datasketches::req::{RankAccuracy, ReqSketch};
use req_common::TestItem;

/// Mutated buffers per item type.
const ITERATIONS: usize = 20_000;
const SEED: u64 = 0x00C0_FFEE_5EED_0004;

/// splitmix64, as in `kll_common::Normal::next_u64`.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

fn genuine<T: TestItem>(
    k: u16,
    accuracy: RankAccuracy,
    n: u32,
    rng: &mut SplitMix64,
) -> ReqSketch<T> {
    let mut sketch = ReqSketch::<T>::new(k, accuracy).unwrap();
    for _ in 0..n {
        // Integers below 2^24, exact in f32 too.
        sketch.update(T::from_f64((rng.next_u64() % (1 << 20)) as f64));
    }
    sketch
}

fn corpus<T: TestItem>(rng: &mut SplitMix64) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for accuracy in [RankAccuracy::High, RankAccuracy::Low] {
        for n in [0, 1, 3, 4, 5, 12, 50, 1_000, 100_000] {
            out.push(genuine::<T>(12, accuracy, n, rng).serialize());
        }
        // Other k, for other section sizes and level counts.
        for k in [4, 254] {
            out.push(genuine::<T>(k, accuracy, 5_000, rng).serialize());
        }
    }
    out
}

/// Overwrites an aligned 1/2/4/8-byte field with a boundary value.
fn overwrite_field(bytes: &mut [u8], rng: &mut SplitMix64) {
    let width = [1usize, 2, 4, 8][rng.below(4)];
    if bytes.len() < width {
        return;
    }
    let start = rng.below(bytes.len() / width) * width;
    let field = &mut bytes[start..start + width];
    match rng.below(5) {
        0 => field.fill(0),
        1 => {
            field.fill(0);
            field[0] = 1;
        }
        2 => field.fill(0xFF),
        3 => {
            field.fill(0xFF);
            field[width - 1] = 0x7F;
        }
        _ => {
            field.fill(0);
            field[width - 1] = 0x80;
        }
    }
}

fn mutate(bytes: &mut Vec<u8>, rng: &mut SplitMix64) {
    for _ in 0..1 + rng.below(4) {
        match rng.below(5) {
            0 if !bytes.is_empty() => {
                let i = rng.below(bytes.len());
                bytes[i] = rng.next_u64() as u8;
            }
            1 if !bytes.is_empty() => {
                let i = rng.below(bytes.len());
                bytes[i] ^= 1 << rng.below(8);
            }
            2 => {
                let len = rng.below(bytes.len() + 1);
                bytes.truncate(len);
            }
            3 => bytes.push(rng.next_u64() as u8),
            _ => overwrite_field(bytes, rng),
        }
    }
}

/// Everything a caller can do with a deserialized sketch. Errors are fine;
/// panics and aborts are not.
fn exercise<T: TestItem>(s: &ReqSketch<T>, partner: &ReqSketch<T>, rng: &mut SplitMix64) {
    let x = T::from_f64((rng.next_u64() % (1 << 20)) as f64);
    let mut merged = s.clone();
    let _ = merged.merge(partner);
    let mut other = partner.clone();
    let _ = other.merge(s);
    let _ = s.get_quantile(0.5, true);
    let _ = s.get_rank(x, true);
    let _ = s.get_cdf(&[x], true);
    let _ = s.sorted_view().iter().count();
    let _ = s.iter().count();
    let again = ReqSketch::<T>::deserialize(&s.serialize());
    assert!(
        again.is_ok(),
        "an accepted sketch must re-serialize to accepted bytes: {:?}",
        again.err()
    );
    let mut updated = s.clone();
    updated.update(x);
    let _ = updated.get_quantile(0.99, false);
    let _ = merged.get_quantile(0.5, false);
    let _ = other.get_rank(x, false);
}

fn fuzz<T: TestItem>() {
    let mut rng = SplitMix64 { state: SEED };
    let corpus = corpus::<T>(&mut rng);
    let high = genuine::<T>(12, RankAccuracy::High, 1_000, &mut rng);
    let low = genuine::<T>(12, RankAccuracy::Low, 1_000, &mut rng);
    // Partners with a different k, so merges also cross section sizes.
    let high_254 = genuine::<T>(254, RankAccuracy::High, 1_000, &mut rng);
    let low_254 = genuine::<T>(254, RankAccuracy::Low, 1_000, &mut rng);
    for bytes in &corpus {
        let s = ReqSketch::<T>::deserialize(bytes).expect("genuine bytes deserialize");
        let partner = if s.rank_accuracy() == RankAccuracy::High {
            &high
        } else {
            &low
        };
        exercise(&s, partner, &mut rng);
    }
    let mut accepted = 0usize;
    for _ in 0..ITERATIONS {
        let mut bytes = corpus[rng.below(corpus.len())].clone();
        mutate(&mut bytes, &mut rng);
        if let Ok(s) = ReqSketch::<T>::deserialize(&bytes) {
            accepted += 1;
            // Merge against a partner of the same accuracy (the other gives
            // InvalidConfig before reaching upstream) and, now and then, the
            // other one too. Half the partners have k = 254 instead of 12.
            let wide = rng.below(2) == 0;
            let same_high = s.rank_accuracy() == RankAccuracy::High;
            let pick = |high_side: bool| match (high_side, wide) {
                (true, false) => &high,
                (false, false) => &low,
                (true, true) => &high_254,
                (false, true) => &low_254,
            };
            let partner = pick(if rng.below(8) == 0 {
                !same_high
            } else {
                same_high
            });
            // Every 16th accepted sketch is also fed enough updates to run
            // its compaction paths.
            if accepted.is_multiple_of(16) {
                let mut busy = s.clone();
                for _ in 0..300 {
                    busy.update(T::from_f64((rng.next_u64() % (1 << 20)) as f64));
                }
                let _ = busy.get_quantile(0.5, true);
                let _ = busy.merge(partner);
                // The re-deserialize is not asserted: a sketch accepted here
                // can legitimately grow into bytes the validator rejects
                // (see `ReqSketch::deserialize`).
                let _ = ReqSketch::<T>::deserialize(&busy.serialize());
            }
            exercise(&s, partner, &mut rng);
        }
    }
    // Make sure the mutations leave enough sketches valid to exercise the
    // post-deserialize paths, not just the rejections.
    assert!(
        accepted > ITERATIONS / 50,
        "only {accepted} mutated buffers were accepted"
    );
    eprintln!("{} of {ITERATIONS} mutated buffers accepted", accepted);
}

#[test]
fn fuzz_f64() {
    fuzz::<f64>();
}

#[test]
fn fuzz_f32() {
    fuzz::<f32>();
}

/// The validator must never reject what upstream itself produces: chains of
/// merges across different `k` (the receiver keeps its own section sizes),
/// interleaved with updates, serialize and deserialize again at every step.
fn genuine_merge_chains_round_trip<T: TestItem>() {
    let mut rng = SplitMix64 { state: SEED ^ 1 };
    for accuracy in [RankAccuracy::High, RankAccuracy::Low] {
        for receiver_k in [4, 12, 254] {
            let mut acc = ReqSketch::<T>::new(receiver_k, accuracy).unwrap();
            for (step, (k, n)) in [(254, 3), (4, 30_000), (12, 2), (254, 50_000), (4, 200_000)]
                .into_iter()
                .enumerate()
            {
                let part = genuine::<T>(k, accuracy, n, &mut rng);
                acc.merge(&part).unwrap();
                for _ in 0..1_000 {
                    acc.update(T::from_f64((rng.next_u64() % (1 << 20)) as f64));
                }
                acc = ReqSketch::<T>::deserialize(&acc.serialize()).unwrap_or_else(|e| {
                    panic!("receiver k = {receiver_k}, {accuracy:?}, step {step}: {e:?}")
                });
                let mut back = part.clone();
                back.merge(&acc).unwrap();
                ReqSketch::<T>::deserialize(&back.serialize()).unwrap();
            }
        }
    }
}

#[test]
fn genuine_merge_chains_round_trip_f64() {
    genuine_merge_chains_round_trip::<f64>();
}

#[test]
fn genuine_merge_chains_round_trip_f32() {
    genuine_merge_chains_round_trip::<f32>();
}
