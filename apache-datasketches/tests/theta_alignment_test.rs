//! Alignment handling for Theta: `deserialize` accepts any alignment (copying
//! first), `wrap` rejects misaligned bytes and, once wrapped from aligned
//! bytes, set operations read only aligned memory.

use apache_datasketches::theta::{
    CompactThetaSketch, ThetaAnotB, ThetaIntersection, ThetaSketchBuilder, ThetaUnionBuilder,
    WrappedCompactThetaSketch,
};
use apache_datasketches::{theta::jaccard_similarity, SketchError};

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

fn sketch_with(n: u64, lg_k: u8) -> CompactThetaSketch {
    let mut s = ThetaSketchBuilder::new().lg_k(lg_k).build().unwrap();
    for i in 0..n {
        s.update_u64(i);
    }
    s.compact(true)
}

/// Named serialized images covering every parser branch: empty, single item,
/// exact ordered/unordered (v3), estimation mode (v3) and compressed (v4,
/// exact and estimation).
fn images() -> Vec<(&'static str, Vec<u8>)> {
    let unordered = {
        let mut s = ThetaSketchBuilder::new().lg_k(12).build().unwrap();
        for i in 0..100u64 {
            s.update_u64(i);
        }
        s.compact(false)
    };
    vec![
        ("empty", sketch_with(0, 12).serialize_compact()),
        ("single", sketch_with(1, 12).serialize_compact()),
        ("exact ordered", sketch_with(100, 12).serialize_compact()),
        ("exact unordered", unordered.serialize_compact()),
        ("estimation", sketch_with(50_000, 10).serialize_compact()),
        ("v4 exact", sketch_with(100, 12).serialize_compressed()),
        (
            "v4 estimation",
            sketch_with(50_000, 10).serialize_compressed(),
        ),
    ]
}

#[test]
fn deserialize_accepts_every_alignment() {
    for (name, bytes) in images() {
        let reference = CompactThetaSketch::deserialize(&bytes).unwrap();
        for offset in 0..8 {
            let p = Placed::new(&bytes, offset);
            for g in [
                CompactThetaSketch::deserialize(p.slice()).unwrap(),
                CompactThetaSketch::deserialize_compressed(p.slice()).unwrap(),
            ] {
                assert_eq!(
                    g.get_estimate(),
                    reference.get_estimate(),
                    "{name} @{offset}"
                );
                assert_eq!(g.get_num_retained(), reference.get_num_retained());
                assert_eq!(
                    g.serialize_compact(),
                    reference.serialize_compact(),
                    "{name} @{offset}"
                );
                assert_eq!(
                    g.serialize_compressed(),
                    reference.serialize_compressed(),
                    "{name} @{offset}"
                );
            }
        }
    }
}

#[test]
fn deserialized_sketch_does_not_borrow_the_input() {
    // Overwrite the source after deserializing: the owned sketch must not
    // retain a pointer into it (or into the aligned temporary).
    for (name, bytes) in images() {
        let reference = CompactThetaSketch::deserialize(&bytes).unwrap();
        for offset in [0, 3] {
            let mut p = Placed::new(&bytes, offset);
            let g = CompactThetaSketch::deserialize(p.slice()).unwrap();
            p.words.iter_mut().for_each(|w| *w = !0);
            assert_eq!(
                g.serialize_compact(),
                reference.serialize_compact(),
                "{name} @{offset}"
            );
        }
    }
}

#[test]
fn wrap_rejects_misaligned() {
    for (name, bytes) in images() {
        for offset in 1..8 {
            let p = Placed::new(&bytes, offset);
            match WrappedCompactThetaSketch::wrap(p.slice()) {
                Err(SketchError::Deserialization(m)) => {
                    assert!(m.contains("8-byte aligned"), "{name} @{offset}: {m}")
                }
                Err(e) => panic!("{name} @{offset}: wrong error {e:?}"),
                Ok(_) => panic!("{name} @{offset}: misaligned wrap accepted"),
            }
        }
    }
}

#[test]
fn wrap_short_input_keeps_its_size_error() {
    // A zero-length slice has an arbitrary (dangling) address; it must still
    // fail as too short rather than as misaligned.
    match WrappedCompactThetaSketch::wrap(&[]) {
        Err(SketchError::Deserialization(m)) => assert!(!m.contains("aligned"), "{m}"),
        other => panic!("unexpected {:?}", other.map(|_| ())),
    }
}

#[test]
fn wrap_accepts_aligned_and_set_ops_work() {
    let a = sketch_with(20_000, 10);
    let b = {
        let mut s = ThetaSketchBuilder::new().lg_k(10).build().unwrap();
        for i in 10_000..30_000u64 {
            s.update_u64(i);
        }
        s.compact(true)
    };
    for compressed in [false, true] {
        let ser = |s: &CompactThetaSketch| {
            if compressed {
                s.serialize_compressed()
            } else {
                s.serialize_compact()
            }
        };
        for offset in [0usize, 8, 16] {
            let pa = Placed::new(&ser(&a), offset);
            let pb = Placed::new(&ser(&b), offset);
            let wa = WrappedCompactThetaSketch::wrap(pa.slice()).unwrap();
            let wb = WrappedCompactThetaSketch::wrap(pb.slice()).unwrap();
            assert_eq!(wa.get_estimate(), a.get_estimate());

            let mut u = ThetaUnionBuilder::new().lg_k(10).build().unwrap();
            u.update(&wa);
            u.update(&wb);
            let mut u_ref = ThetaUnionBuilder::new().lg_k(10).build().unwrap();
            u_ref.update(&a);
            u_ref.update(&b);
            assert_eq!(
                u.get_result(true).serialize_compact(),
                u_ref.get_result(true).serialize_compact()
            );

            let mut i = ThetaIntersection::new();
            i.update(&wa);
            i.update(&wb);
            let mut i_ref = ThetaIntersection::new();
            i_ref.update(&a);
            i_ref.update(&b);
            assert_eq!(
                i.get_result(true).unwrap().serialize_compact(),
                i_ref.get_result(true).unwrap().serialize_compact()
            );

            let d = ThetaAnotB::new().compute(&wa, &wb, true);
            let d_ref = ThetaAnotB::new().compute(&a, &b, true);
            assert_eq!(d.serialize_compact(), d_ref.serialize_compact());

            let j = jaccard_similarity(&wa, &wb);
            let j_ref = jaccard_similarity(&a, &b);
            assert_eq!(j.estimate, j_ref.estimate);
        }
    }
}
