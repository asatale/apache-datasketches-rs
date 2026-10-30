//! Alignment handling: `deserialize` accepts any alignment (copying first),
//! `wrap` rejects a misaligned non-empty image and ignores alignment for an
//! empty one (which upstream rebuilds as an owned filter).

use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};
use apache_datasketches::SketchError;

/// A buffer whose base is 8-byte aligned (it is a `Vec<u64>`), holding
/// `bytes` at `offset`.
struct Placed {
    words: Vec<u64>,
    offset: usize,
    len: usize,
}

impl Placed {
    fn new(bytes: &[u8], offset: usize) -> Self {
        let mut words = vec![0u64; (offset + bytes.len()).div_ceil(8) + 1];
        let base = words.as_mut_ptr() as *mut u8;
        // SAFETY: the allocation holds at least offset + bytes.len() bytes.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(offset), bytes.len()) };
        Placed {
            words,
            offset,
            len: bytes.len(),
        }
    }

    fn slice(&self) -> &[u8] {
        // SAFETY: a Vec<u64> is valid as bytes; offset + len is in bounds.
        let base = self.words.as_ptr() as *const u8;
        unsafe { std::slice::from_raw_parts(base.add(self.offset), self.len) }
    }
}

fn non_empty() -> BloomFilter {
    let mut f = BloomFilterBuilder::by_accuracy(1_000, 0.01)
        .build()
        .unwrap();
    for i in 0..500u64 {
        f.update_u64(i);
    }
    f
}

fn empty() -> BloomFilter {
    BloomFilterBuilder::by_size(1_024, 3).build().unwrap()
}

#[test]
fn deserialize_accepts_every_alignment() {
    let f = non_empty();
    let bytes = f.serialize();
    for offset in 0..8 {
        let p = Placed::new(&bytes, offset);
        let g = BloomFilter::deserialize(p.slice()).unwrap();
        assert_eq!(g.serialize(), bytes, "offset {offset}");
        for i in 0..1_000u64 {
            assert_eq!(g.query_u64(i), f.query_u64(i), "offset {offset} item {i}");
        }
    }
}

#[test]
fn wrap_rejects_misaligned_non_empty() {
    let bytes = non_empty().serialize();
    for offset in 1..8 {
        let p = Placed::new(&bytes, offset);
        match WrappedBloomFilter::wrap(p.slice()) {
            Err(SketchError::Deserialization(m)) => {
                assert!(m.contains("8-byte aligned"), "{m}")
            }
            Err(e) => panic!("offset {offset}: wrong error {e:?}"),
            Ok(_) => panic!("offset {offset}: misaligned wrap accepted"),
        }
    }
}

#[test]
fn wrap_accepts_aligned_non_empty() {
    let f = non_empty();
    let bytes = f.serialize();
    for offset in [0, 8, 16] {
        let p = Placed::new(&bytes, offset);
        let w = WrappedBloomFilter::wrap(p.slice()).unwrap();
        for i in 0..1_000u64 {
            assert_eq!(w.query_u64(i), f.query_u64(i), "offset {offset} item {i}");
        }
    }
}

#[test]
fn empty_image_ignores_alignment() {
    let f = empty();
    let bytes = f.serialize();
    for offset in 0..8 {
        let p = Placed::new(&bytes, offset);
        let w = WrappedBloomFilter::wrap(p.slice()).unwrap();
        assert_eq!(w.capacity(), f.capacity());
        assert_eq!(
            BloomFilter::deserialize(p.slice()).unwrap().serialize(),
            bytes
        );
    }
}
