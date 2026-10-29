//! Deterministic mutation fuzzing of `BloomFilter::deserialize` and
//! `WrappedBloomFilter::wrap`.
//!
//! Genuine serialized filters are mutated at random (fixed seed, so every run
//! checks the same inputs). Any buffer that is accepted must then survive
//! every query, update and set operation without panicking or aborting:
//! upstream trusts parts of its input, so whatever the shim's validator lets
//! through reaches upstream's hashing and bit-array code as-is.

mod bloom_common;

use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};
use bloom_common::{DIRTY, EMPTY_FLAG};

/// Mutated buffers.
const ITERATIONS: usize = 20_000;
const SEED: u64 = 0x00C0_FFEE_5EED_0021;
/// Largest capacity, in bits, a mutated empty image may claim. The validator
/// deliberately accepts genuine empty images up to 2^32 - 64 bits, and both
/// deserialize and wrap allocate the whole bit array for them; every
/// accepted filter is also cloned, unioned and re-serialized several times,
/// so keep this small for the runtime.
const MAX_EMPTY_BITS: u32 = 1 << 18;
/// Images with more hashes than this skip the per-item operation loops:
/// each query or update runs `num_hashes` iterations.
const MANY_HASHES: u16 = 256;

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

fn genuine(num_bits: u64, num_hashes: u16, seed: u64, n: u64, rng: &mut SplitMix64) -> BloomFilter {
    let mut f = BloomFilterBuilder::by_size(num_bits, num_hashes)
        .seed(seed)
        .build()
        .unwrap();
    for _ in 0..n {
        f.update_u64(rng.next_u64());
    }
    f
}

/// Genuine images: empty, sparse, dense and inverted filters of several
/// sizes, hash counts and seeds, plus the dirty-sentinel variant of each
/// non-empty one (upstream writes it for a filter updated since its last
/// count; this crate's filters never are, so it is set by hand).
fn corpus(rng: &mut SplitMix64) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (num_bits, num_hashes, seed) in [
        (64u64, 1u16, 0u64),
        (128, 3, 9001),
        (1_000, 7, u64::MAX),
        (4_096, 2, 42),
        (65_536, 11, 0x1234_5678),
    ] {
        let words = num_bits.div_ceil(64);
        for n in [0, 1, words / 2, words * 4, words * 64] {
            out.push(genuine(num_bits, num_hashes, seed, n, rng).serialize());
        }
        let mut inverted = genuine(num_bits, num_hashes, seed, words, rng);
        inverted.invert();
        out.push(inverted.serialize());
    }
    let dirty: Vec<Vec<u8>> = out
        .iter()
        .filter(|b| b[3] & EMPTY_FLAG == 0)
        .map(|b| {
            let mut b = b.clone();
            b[24..32].copy_from_slice(&DIRTY.to_le_bytes());
            b
        })
        .collect();
    out.extend(dirty);
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

/// Targets the header fields upstream trusts.
fn overwrite_header(bytes: &mut [u8], rng: &mut SplitMix64) {
    let mut put = |offset: usize, value: &[u8]| {
        if bytes.len() >= offset + value.len() {
            bytes[offset..offset + value.len()].copy_from_slice(value);
        }
    };
    match rng.below(6) {
        // num_longs: zero, the 32-bit shift wrap points, negative int32.
        0 => {
            let v = [
                0u32,
                1,
                2,
                (1 << 26) - 1,
                1 << 26,
                (1 << 26) + 1,
                1 << 29,
                (1 << 29) + 1,
                0x8000_0000,
                u32::MAX,
            ][rng.below(10)];
            put(16, &v.to_le_bytes());
        }
        // num_hashes, including 0 and the maximum.
        1 => {
            let v = [0u16, 1, 2, 255, 65_535][rng.below(5)];
            put(4, &v.to_le_bytes());
        }
        // Bits-set count: the dirty sentinel, zero, off by one.
        2 => {
            let v = [DIRTY, 0, 1, DIRTY - 1][rng.below(4)];
            put(24, &v.to_le_bytes());
        }
        // Flags: toggle the empty flag or set an unknown bit.
        3 => {
            if bytes.len() > 3 {
                bytes[3] ^= [EMPTY_FLAG, 1, 0x80][rng.below(3)];
            }
        }
        // Preamble longs 3 or 4 regardless of the flag.
        4 => put(0, &[3 + rng.below(2) as u8]),
        // Seed.
        _ => put(8, &rng.next_u64().to_le_bytes()),
    }
}

fn mutate(bytes: &mut Vec<u8>, rng: &mut SplitMix64) {
    for _ in 0..1 + rng.below(4) {
        match rng.below(6) {
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
            4 => overwrite_header(bytes, rng),
            _ => overwrite_field(bytes, rng),
        }
    }
    // Keep a mutated empty image's claimed capacity small (see
    // MAX_EMPTY_BITS). Keyed on the empty flag alone, so the clamp holds
    // whatever the validator does with the preamble.
    if bytes.len() >= 20 && bytes[3] & EMPTY_FLAG != 0 {
        let num_longs = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        if num_longs > MAX_EMPTY_BITS / 64 {
            let clamped = num_longs % (MAX_EMPTY_BITS / 64 + 1);
            bytes[16..20].copy_from_slice(&clamped.to_le_bytes());
        }
    }
}

/// A genuine filter guaranteed compatible with `capacity`, `num_hashes`,
/// `seed`, with a few items in it.
fn partner(capacity: u64, num_hashes: u16, seed: u64, rng: &mut SplitMix64) -> BloomFilter {
    let n = if num_hashes > MANY_HASHES { 1 } else { 20 };
    let mut f = BloomFilterBuilder::by_size(capacity, num_hashes)
        .seed(seed)
        .build()
        .expect("an accepted filter's shape must be buildable");
    for _ in 0..n {
        f.update_u64(rng.next_u64());
    }
    f
}

/// Everything a caller can do with an accepted owned filter. Errors are
/// fine; panics and aborts are not.
fn exercise_owned(f: &BloomFilter, rng: &mut SplitMix64) {
    let few = f.num_hashes() > MANY_HASHES;
    let items = if few { 2 } else { 16 };
    let mut f = f.clone();
    let _ = f.bits_used();
    let _ = f.to_string_summary();
    for _ in 0..items {
        let x = rng.next_u64();
        let _ = f.query_u64(x);
        let _ = f.query_i32(x as i32);
        let _ = f.query_f64(x as f64);
        let _ = f.query_str("fuzz");
        let _ = f.query_bytes(&x.to_le_bytes());
    }
    // Updates first: they divide by the capacity and loop num_hashes times,
    // the paths a bad header reaches.
    let original = f.clone();
    for _ in 0..items {
        let x = rng.next_u64();
        f.update_u64(x);
        assert!(f.query_and_update_u64(x));
        f.update_f32(x as f32);
        let _ = f.query_and_update_str("fuzz");
        assert!(f.query_u64(x));
    }
    let p = partner(f.capacity(), f.num_hashes(), f.seed(), rng);
    assert!(f.is_compatible(&p) && p.is_compatible(&f));
    let f_updated = f;
    let f = original;
    let mut u = f.clone();
    u.union_with(&p).unwrap();
    let mut i = f.clone();
    i.intersect(&p).unwrap();
    let mut q = p.clone();
    q.union_with(&f).unwrap();
    q.intersect(&f).unwrap();
    let _ = u.bits_used() + i.bits_used();
    let mut f = f_updated;
    f.invert();
    // Every filter the crate holds must serialize to bytes it reads back,
    // including after updates, set operations and invert.
    for g in [&f, &u, &i, &q] {
        let bytes = g.serialize();
        let again = BloomFilter::deserialize(&bytes);
        assert!(
            again.is_ok(),
            "re-serialized bytes rejected: {:?}",
            again.err()
        );
        assert!(WrappedBloomFilter::wrap(&bytes).is_ok());
    }
    f.reset();
    assert!(f.is_empty());
}

/// Everything a caller can do with an accepted wrapped view.
fn exercise_wrapped(w: &WrappedBloomFilter<'_>, rng: &mut SplitMix64) {
    let few = w.num_hashes() > MANY_HASHES;
    let items = if few { 2 } else { 16 };
    let _ = w.bits_used();
    let _ = w.to_string_summary();
    for _ in 0..items {
        let x = rng.next_u64();
        let _ = w.query_u64(x);
        let _ = w.query_i8(x as i8);
        let _ = w.query_f32(x as f32);
        let _ = w.query_str("fuzz");
        let _ = w.query_bytes(&x.to_le_bytes());
    }
    let mut owned = w.to_owned();
    assert_eq!(owned.bits_used(), w.bits_used());
    owned.update_u64(rng.next_u64());
    assert!(BloomFilter::deserialize(&owned.serialize()).is_ok());
    let mut p = partner(w.capacity(), w.num_hashes(), w.seed(), rng);
    assert!(w.is_compatible(&p) && p.is_compatible(w));
    p.union_with(w).unwrap();
    p.intersect(w).unwrap();
}

#[test]
fn fuzz() {
    let mut rng = SplitMix64 { state: SEED };
    let corpus = corpus(&mut rng);
    for bytes in &corpus {
        let f = BloomFilter::deserialize(bytes).expect("genuine bytes deserialize");
        exercise_owned(&f, &mut rng);
        let w = WrappedBloomFilter::wrap(bytes).expect("genuine bytes wrap");
        exercise_wrapped(&w, &mut rng);
    }
    let (mut owned_ok, mut wrapped_ok) = (0usize, 0usize);
    for _ in 0..ITERATIONS {
        let mut bytes = corpus[rng.below(corpus.len())].clone();
        mutate(&mut bytes, &mut rng);
        let owned = BloomFilter::deserialize(&bytes);
        let wrapped = WrappedBloomFilter::wrap(&bytes);
        assert_eq!(
            owned.is_ok(),
            wrapped.is_ok(),
            "deserialize and wrap must agree: {:?} vs {:?}",
            owned.as_ref().err(),
            wrapped.as_ref().err()
        );
        if let Ok(f) = owned {
            owned_ok += 1;
            exercise_owned(&f, &mut rng);
        }
        if let Ok(w) = wrapped {
            wrapped_ok += 1;
            exercise_wrapped(&w, &mut rng);
        }
    }
    // Make sure the mutations leave enough filters valid to exercise the
    // post-deserialize paths, not just the rejections.
    assert!(
        owned_ok > ITERATIONS / 20 && wrapped_ok > ITERATIONS / 20,
        "only {owned_ok} / {wrapped_ok} mutated buffers were accepted"
    );
    eprintln!(
        "{owned_ok} of {ITERATIONS} mutated buffers accepted (deserialize), {wrapped_ok} (wrap)"
    );
}
