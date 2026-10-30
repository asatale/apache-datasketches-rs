//! Helpers shared by the Bloom filter integration tests: a hand builder for
//! serialized images (crafted bytes for the hardening tests) and the
//! rejection checks. Each test file pulls this in with `mod bloom_common;`.

// Not every test file uses every helper.
#![allow(dead_code)]

use apache_datasketches::bloom::{BloomFilter, WrappedBloomFilter};
use apache_datasketches::SketchError;

/// Byte 3 bit 2: the image is of an empty filter.
pub const EMPTY_FLAG: u8 = 4;
/// Bits-set count written by a filter updated since its last count.
pub const DIRTY: u64 = u64::MAX;

/// A serialized Bloom filter image (upstream `bloom_filter_impl.hpp`
/// `serialize`, little-endian). `count` and `bits` are written only when
/// given, so a caller can build any preamble/flag combination.
pub struct Image {
    pub preamble_longs: u8,
    pub serial_version: u8,
    pub family: u8,
    pub flags: u8,
    pub num_hashes: u16,
    pub seed: u64,
    pub num_longs: u32,
    pub count: Option<u64>,
    pub bits: Vec<u8>,
}

impl Image {
    /// A non-empty image of `num_longs` words with the bits in `set` set and
    /// a correct count.
    pub fn non_empty(num_longs: u32, num_hashes: u16, seed: u64, set: &[u64]) -> Self {
        let mut bits = vec![0u8; num_longs as usize * 8];
        for &i in set {
            bits[(i / 8) as usize] |= 1 << (i % 8);
        }
        let count = bits.iter().map(|b| b.count_ones() as u64).sum();
        Self {
            preamble_longs: 4,
            serial_version: 1,
            family: 21,
            flags: 0,
            num_hashes,
            seed,
            num_longs,
            count: Some(count),
            bits,
        }
    }

    /// An empty image claiming `num_longs` words.
    pub fn empty(num_longs: u32, num_hashes: u16, seed: u64) -> Self {
        Self {
            preamble_longs: 3,
            serial_version: 1,
            family: 21,
            flags: EMPTY_FLAG,
            num_hashes,
            seed,
            num_longs,
            count: None,
            bits: Vec::new(),
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = vec![
            self.preamble_longs,
            self.serial_version,
            self.family,
            self.flags,
        ];
        out.extend_from_slice(&self.num_hashes.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&self.seed.to_le_bytes());
        out.extend_from_slice(&self.num_longs.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        if let Some(count) = self.count {
            out.extend_from_slice(&count.to_le_bytes());
        }
        out.extend_from_slice(&self.bits);
        out
    }
}

/// Asserts `BloomFilter::deserialize` rejects `bytes`. Should a regression
/// ever accept them, the filter is first put through every operation that
/// reads its capacity (update, query-and-update, query): a crash there is
/// the failure these images were built to provoke.
pub fn assert_deserialize_rejected(bytes: &[u8], what: &str) {
    match BloomFilter::deserialize(bytes) {
        Err(SketchError::Deserialization(_)) => {}
        Err(e) => panic!("{what}: wrong error kind {e:?}"),
        Ok(mut f) => {
            let capacity = f.capacity();
            f.update_u64(1);
            f.update_str("x");
            let _ = f.query_and_update_u64(2);
            let _ = f.query_u64(1);
            panic!("{what}: deserialize accepted the bytes (capacity {capacity})");
        }
    }
}

/// Asserts `WrappedBloomFilter::wrap` rejects `bytes`; on a regression the
/// view is queried first (see [`assert_deserialize_rejected`]).
pub fn assert_wrap_rejected(bytes: &[u8], what: &str) {
    match WrappedBloomFilter::wrap(bytes) {
        Err(SketchError::Deserialization(_)) => {}
        Err(e) => panic!("{what}: wrong error kind {e:?}"),
        Ok(w) => {
            let capacity = w.capacity();
            let _ = w.query_u64(1);
            let _ = w.query_str("x");
            panic!("{what}: wrap accepted the bytes (capacity {capacity})");
        }
    }
}

/// Both of the above.
pub fn assert_rejected(bytes: &[u8], what: &str) {
    assert_deserialize_rejected(bytes, what);
    assert_wrap_rejected(bytes, what);
}
