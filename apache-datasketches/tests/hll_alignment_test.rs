//! `HllSketch::deserialize` accepts any alignment. The case that reached
//! undefined behavior upstream is an HLL4 image with a non-empty auxiliary
//! map, which a real sketch rarely produces, so it is forged here.

use apache_datasketches::hll::{HllSketch, TargetHllType};
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

/// A compact HLL4 image at lg_k 4 whose slot 3 holds the aux token and whose
/// aux map holds one pair (slot 3, value 20).
fn forged_hll4_with_aux() -> Vec<u8> {
    const LG_K: u8 = 4;
    let mut b = vec![0u8; 40 + 8 + 4];
    b[0] = 10; // preInts
    b[1] = 1; // serVer
    b[2] = 7; // family
    b[3] = LG_K;
    b[4] = 0; // lgArr
    b[5] = 16 | 8; // out-of-order | compact
    b[6] = 0; // curMin
    b[7] = 0x02; // HLL mode, HLL4 target
    b[8..16].copy_from_slice(&0f64.to_le_bytes()); // hip accumulator
    b[16..24].copy_from_slice(&15f64.to_le_bytes()); // kxq0
    b[24..32].copy_from_slice(&0f64.to_le_bytes()); // kxq1
    b[32..36].copy_from_slice(&15u32.to_le_bytes()); // curMin count
    b[36..40].copy_from_slice(&1u32.to_le_bytes()); // aux count
    b[41] = 0xF0; // slot 3 = aux token (high nibble of byte 1)
    let pair: u32 = 3 | (20 << 26);
    b[48..52].copy_from_slice(&pair.to_le_bytes());
    b
}

fn assert_round_trips_at_every_offset(bytes: &[u8]) {
    let reference = HllSketch::deserialize(bytes).unwrap();
    for offset in 0..8 {
        let p = Placed::new(bytes, offset);
        let g = HllSketch::deserialize(p.slice()).unwrap();
        assert_eq!(
            g.get_estimate(),
            reference.get_estimate(),
            "offset {offset}"
        );
        assert_eq!(
            g.serialize_compact(),
            reference.serialize_compact(),
            "offset {offset}"
        );
        assert_eq!(
            g.serialize_updatable(),
            reference.serialize_updatable(),
            "offset {offset}"
        );
    }
}

#[test]
fn forged_hll4_with_aux_map_is_accepted_aligned() {
    let bytes = forged_hll4_with_aux();
    let s = HllSketch::deserialize(&bytes).expect("upstream accepts the forged image");
    assert!(s.get_estimate().is_finite());
}

#[test]
fn hll4_with_aux_map_deserializes_at_every_offset() {
    assert_round_trips_at_every_offset(&forged_hll4_with_aux());
}

#[test]
fn ordinary_images_deserialize_at_every_offset() {
    for tgt in [
        TargetHllType::Hll4,
        TargetHllType::Hll6,
        TargetHllType::Hll8,
    ] {
        for n in [0u64, 3, 20, 100_000] {
            let mut s = HllSketch::new(12, tgt).unwrap();
            for i in 0..n {
                s.update_u64(i);
            }
            assert_round_trips_at_every_offset(&s.serialize_compact());
            assert_round_trips_at_every_offset(&s.serialize_updatable());
        }
    }
}

#[test]
fn empty_input_is_rejected_not_crashed() {
    assert!(matches!(
        HllSketch::deserialize(&[]),
        Err(SketchError::Deserialization(_))
    ));
    // An aligned, non-dangling empty subslice of a real buffer.
    let p = Placed::new(&forged_hll4_with_aux(), 0);
    assert!(matches!(
        HllSketch::deserialize(&p.slice()[8..8]),
        Err(SketchError::Deserialization(_))
    ));
}

#[test]
fn updatable_image_of_forged_aux_map_round_trips_at_every_offset() {
    // The updatable image is built by re-serializing the forged sketch, then
    // deserialized again at every offset; this reads the aux map the way
    // AuxHashMap-internal.hpp:82 does for an updatable HLL4 image.
    let updatable = HllSketch::deserialize(&forged_hll4_with_aux())
        .unwrap()
        .serialize_updatable();
    assert_round_trips_at_every_offset(&updatable);
}
