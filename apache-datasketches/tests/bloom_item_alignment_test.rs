//! Bloom item hashing: upstream's XXHash64 reads items through `uint64_t*`, so
//! a misaligned `&[u8]`/`&str` item must hash exactly as the same bytes
//! aligned would, without a misaligned read.

use apache_datasketches::bloom::{BloomFilter, BloomFilterBuilder, WrappedBloomFilter};

/// Item bytes held at an explicit byte offset of an 8-byte-aligned base.
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

    fn bytes(&self) -> &[u8] {
        // SAFETY: a Vec<u64> is valid as bytes; offset + len is in bounds.
        let base = self.words.as_ptr() as *const u8;
        unsafe { std::slice::from_raw_parts(base.add(self.offset), self.len) }
    }
}

/// Lengths around the 8-byte word, the 32-byte block, the stack-buffer limit
/// and well above it.
const LENS: [usize; 10] = [1, 7, 8, 9, 31, 32, 33, 100, 256, 1000];

fn item(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + len) as u8).collect()
}

fn fresh() -> BloomFilter {
    BloomFilterBuilder::by_size(4096, 3).build().unwrap()
}

#[test]
fn misaligned_bytes_hash_like_aligned() {
    for len in LENS {
        let data = item(len);
        let mut expected = fresh();
        expected.update_bytes(&data);
        let aligned = Placed::new(&data, 0);
        assert_eq!(aligned.bytes().as_ptr() as usize % 8, 0);
        for offset in 0..8 {
            let p = Placed::new(&data, offset);
            let mut f = fresh();
            assert!(!f.query_bytes(p.bytes()), "len {len} @{offset}");
            assert!(!f.query_and_update_bytes(p.bytes()), "len {len} @{offset}");
            assert!(f.query_and_update_bytes(p.bytes()), "len {len} @{offset}");
            assert!(f.query_bytes(&data), "len {len} @{offset}");
            assert_eq!(f.serialize(), expected.serialize(), "len {len} @{offset}");

            let mut g = fresh();
            g.update_bytes(p.bytes());
            assert_eq!(g.serialize(), expected.serialize(), "len {len} @{offset}");
        }
    }
}

#[test]
fn misaligned_str_hash_like_aligned() {
    for len in LENS {
        let text: String = (0..len).map(|i| (b'a' + (i % 26) as u8) as char).collect();
        let mut expected = fresh();
        expected.update_str(&text);
        for offset in 0..8 {
            let p = Placed::new(text.as_bytes(), offset);
            let s = std::str::from_utf8(p.bytes()).unwrap();
            let mut f = fresh();
            assert!(!f.query_str(s), "len {len} @{offset}");
            assert!(!f.query_and_update_str(s), "len {len} @{offset}");
            assert!(f.query_and_update_str(s), "len {len} @{offset}");
            assert!(f.query_str(&text));
            assert_eq!(f.serialize(), expected.serialize(), "len {len} @{offset}");

            let mut g = fresh();
            g.update_str(s);
            assert_eq!(g.serialize(), expected.serialize(), "len {len} @{offset}");
        }
    }
}

#[test]
fn slicing_a_larger_buffer_is_enough_to_misalign() {
    let data = item(100);
    let mut expected = fresh();
    expected.update_bytes(&data[1..]);
    let mut f = fresh();
    let base = Placed::new(&data, 0);
    f.update_bytes(&base.bytes()[1..]);
    assert_eq!(f.serialize(), expected.serialize());
}

#[test]
fn wrapped_queries_accept_misaligned_items() {
    for len in LENS {
        let data = item(len);
        let text: String = (0..len).map(|i| (b'a' + (i % 26) as u8) as char).collect();
        let mut f = fresh();
        f.update_bytes(&data);
        f.update_str(&text);
        let image = Placed::new(&f.serialize(), 0);
        let w = WrappedBloomFilter::wrap(image.bytes()).unwrap();
        for offset in 0..8 {
            let pb = Placed::new(&data, offset);
            assert!(w.query_bytes(pb.bytes()), "len {len} @{offset}");
            let pt = Placed::new(text.as_bytes(), offset);
            let s = std::str::from_utf8(pt.bytes()).unwrap();
            assert!(w.query_str(s), "len {len} @{offset}");
        }
        // And a miss stays a miss.
        let other = item(len + 1);
        let po = Placed::new(&other, 3);
        assert_eq!(w.query_bytes(po.bytes()), f.query_bytes(&other));
    }
}
