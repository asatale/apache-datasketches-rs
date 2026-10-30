use super::input::{BloomInput, BloomInputRef};
use crate::error::SketchError;
use apache_datasketches_sys::bloom_filter::ffi;
use cxx::UniquePtr;

/// An owned, mutable Bloom filter. Build one with
/// [`BloomFilterBuilder`](super::BloomFilterBuilder) or restore one with
/// [`Self::deserialize`].
pub struct BloomFilter {
    pub(crate) inner: UniquePtr<ffi::BloomFilterShim>,
}

// SAFETY: the shim exclusively owns the filter. Every `&self` method maps to
// a `const` upstream method with no `mutable` state (verified in
// `bloom_filter.hpp`); the one caching method, `get_bits_used`, is exposed as
// `&mut self`. So shared references across threads are read-only.
unsafe impl Send for BloomFilter {}
// SAFETY: see the `Send` impl above.
unsafe impl Sync for BloomFilter {}

macro_rules! bloom_methods {
    ($(($ty:ty, $tyname:literal, $update:ident, $query:ident, $qau:ident)),* $(,)?) => {
        $(
            #[doc = concat!("Adds `value` (as `", $tyname, "`) to the filter.")]
            pub fn $update(&mut self, value: $ty) {
                self.inner.pin_mut().$update(value);
            }

            #[doc = concat!("Returns `true` if `value` (as `", $tyname, "`) may have been added (no false negatives; false positives at roughly the configured rate).")]
            pub fn $query(&self, value: $ty) -> bool {
                self.inner.$query(value)
            }

            #[doc = concat!("Returns whether `value` (as `", $tyname, "`) may have been present, then adds it.")]
            pub fn $qau(&mut self, value: $ty) -> bool {
                self.inner.pin_mut().$qau(value)
            }
        )*
    };
}

impl BloomFilter {
    /// Reconstructs a filter from bytes produced by [`Self::serialize`].
    ///
    /// The bytes are validated before upstream reads them; malformed or
    /// truncated input returns [`SketchError::Deserialization`]. Images with
    /// a capacity of 2^32 bits or more, or with `u16::MAX` hash functions,
    /// are rejected: upstream misreads the former (its capacity arithmetic is
    /// 32-bit) and cannot use the latter.
    ///
    /// Any alignment of `bytes` is accepted: upstream reads the bit array as
    /// 64-bit words, so misaligned input is copied into an aligned buffer
    /// first.
    ///
    /// An empty filter serializes to 24 bytes but records its capacity;
    /// deserializing allocates the full bit array, which can be up to
    /// 2^32 - 64 bits (512 MiB). When deserializing untrusted bytes, bound the
    /// claimed capacity (or the process's memory) yourself.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        let inner = ffi::bloom_deserialize(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self { inner })
    }

    bloom_methods! {
        (u64, "u64", update_u64, query_u64, query_and_update_u64),
        (i64, "i64", update_i64, query_i64, query_and_update_i64),
        (u32, "u32", update_u32, query_u32, query_and_update_u32),
        (i32, "i32", update_i32, query_i32, query_and_update_i32),
        (u16, "u16", update_u16, query_u16, query_and_update_u16),
        (i16, "i16", update_i16, query_i16, query_and_update_i16),
        (u8, "u8", update_u8, query_u8, query_and_update_u8),
        (i8, "i8", update_i8, query_i8, query_and_update_i8),
        (f64, "f64", update_f64, query_f64, query_and_update_f64),
        (f32, "f32", update_f32, query_f32, query_and_update_f32),
        (&str, "&str", update_str, query_str, query_and_update_str),
        (&[u8], "&[u8]", update_bytes, query_bytes, query_and_update_bytes),
    }

    fn check_compatible(&self, other: &impl BloomInput) -> Result<(), SketchError> {
        if self.is_compatible(other) {
            return Ok(());
        }
        let (cap, hashes, seed) = match other.as_bloom_input() {
            BloomInputRef::Filter(o) => (o.get_capacity(), o.get_num_hashes(), o.get_seed()),
            BloomInputRef::Wrapped(o) => (o.get_capacity(), o.get_num_hashes(), o.get_seed()),
        };
        let msg = if cap != self.capacity() {
            format!("capacity mismatch: {} vs {}", self.capacity(), cap)
        } else if hashes != self.num_hashes() {
            format!(
                "number of hashes mismatch: {} vs {}",
                self.num_hashes(),
                hashes
            )
        } else {
            format!("seed mismatch: {} vs {}", self.seed(), seed)
        };
        Err(SketchError::InvalidConfig(format!(
            "incompatible Bloom filters ({msg})"
        )))
    }

    /// Sets this filter to the union (bitwise OR) of itself and `other`.
    ///
    /// Returns [`SketchError::InvalidConfig`] if the filters differ in
    /// capacity, number of hashes, or seed.
    pub fn union_with(&mut self, other: &impl BloomInput) -> Result<(), SketchError> {
        self.check_compatible(other)?;
        match other.as_bloom_input() {
            BloomInputRef::Filter(o) => self.inner.pin_mut().union_with_filter(o),
            BloomInputRef::Wrapped(o) => self.inner.pin_mut().union_with_wrapped(o),
        }
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Sets this filter to the intersection (bitwise AND) of itself and
    /// `other`.
    ///
    /// Returns [`SketchError::InvalidConfig`] if the filters differ in
    /// capacity, number of hashes, or seed.
    pub fn intersect(&mut self, other: &impl BloomInput) -> Result<(), SketchError> {
        self.check_compatible(other)?;
        match other.as_bloom_input() {
            BloomInputRef::Filter(o) => self.inner.pin_mut().intersect_with_filter(o),
            BloomInputRef::Wrapped(o) => self.inner.pin_mut().intersect_with_wrapped(o),
        }
        .map_err(|e| SketchError::InvalidConfig(e.what().to_string()))
    }

    /// Returns `true` if `other` has the same capacity, number of hashes and
    /// seed as this filter, so the two can be combined.
    pub fn is_compatible(&self, other: &impl BloomInput) -> bool {
        match other.as_bloom_input() {
            BloomInputRef::Filter(o) => self.inner.is_compatible_with_filter(o),
            BloomInputRef::Wrapped(o) => self.inner.is_compatible_with_wrapped(o),
        }
    }

    /// Inverts every bit of the filter. The result no longer has the
    /// no-false-negatives guarantee for previously added items.
    pub fn invert(&mut self) {
        self.inner.pin_mut().invert();
    }

    /// Clears the filter to its empty state, keeping its configuration.
    pub fn reset(&mut self) {
        self.inner.pin_mut().reset();
    }

    /// Returns `true` if no bits are set.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the filter's capacity in bits (a multiple of 64).
    pub fn capacity(&self) -> u64 {
        self.inner.get_capacity()
    }

    /// Returns the number of hash functions.
    pub fn num_hashes(&self) -> u16 {
        self.inner.get_num_hashes()
    }

    /// Returns the hash seed.
    pub fn seed(&self) -> u64 {
        self.inner.get_seed()
    }

    /// Returns the number of bits currently set.
    ///
    /// This takes `&mut self` because upstream recounts the set bits and
    /// caches the result after updates; requiring `&mut` keeps
    /// [`BloomFilter`] safely `Sync`.
    pub fn bits_used(&mut self) -> u64 {
        self.inner.pin_mut().get_bits_used()
    }

    /// Serializes the filter to bytes readable by [`Self::deserialize`] and
    /// [`WrappedBloomFilter::wrap`](super::WrappedBloomFilter::wrap).
    ///
    /// Every filter this crate builds round-trips: the builder caps sizes at
    /// 2^32 - 64 bits because upstream computes the capacity it reads back
    /// in 32 bits.
    pub fn serialize(&self) -> Vec<u8> {
        self.inner.serialize().as_slice().to_vec()
    }

    /// Returns a human-readable summary of the filter.
    pub fn to_string_summary(&self) -> String {
        self.inner.to_string_summary()
    }
}

impl Clone for BloomFilter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_shim(),
        }
    }
}
