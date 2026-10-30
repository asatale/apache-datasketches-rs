use super::input::{BloomInput, BloomInputRef};
use super::BloomFilter;
use crate::error::SketchError;
use apache_datasketches_sys::bloom_filter::ffi;
use cxx::UniquePtr;
use std::marker::PhantomData;

/// A zero-copy, read-only view over an already-serialized Bloom filter's
/// bytes, queryable and usable as set-operation input without a full
/// [`BloomFilter::deserialize`]. Build one with [`Self::wrap`].
///
/// The lifetime `'a` ties this view to the borrowed bytes; it cannot outlive
/// them.
///
/// ```compile_fail
/// use apache_datasketches::bloom::{BloomFilterBuilder, WrappedBloomFilter};
/// let w = {
///     let bytes = BloomFilterBuilder::by_size(1024, 3).build().unwrap().serialize();
///     WrappedBloomFilter::wrap(&bytes).unwrap()
/// };
/// let _ = w.capacity();
/// ```
pub struct WrappedBloomFilter<'a> {
    pub(crate) inner: UniquePtr<ffi::WrappedBloomFilterShim>,
    _bytes: PhantomData<&'a [u8]>,
}

// SAFETY: a read-only view over borrowed bytes; it is never dirty (no cached
// state is written through `&self`), and the lifetime ties it to the bytes.
unsafe impl<'a> Send for WrappedBloomFilter<'a> {}
// SAFETY: see the `Send` impl above.
unsafe impl<'a> Sync for WrappedBloomFilter<'a> {}

macro_rules! wrapped_queries {
    ($(($ty:ty, $tyname:literal, $query:ident)),* $(,)?) => {
        $(
            #[doc = concat!("Returns `true` if `value` (as `", $tyname, "`) may have been added (no false negatives; false positives at roughly the configured rate).")]
            pub fn $query(&self, value: $ty) -> bool {
                self.inner.$query(value)
            }
        )*
    };
}

impl<'a> WrappedBloomFilter<'a> {
    /// Wraps a serialized filter's bytes without copying them.
    ///
    /// The bytes are validated as for
    /// [`BloomFilter::deserialize`](super::BloomFilter::deserialize), with the
    /// same limits; malformed or truncated input returns
    /// [`SketchError::Deserialization`].
    ///
    /// # Errors
    ///
    /// Besides malformed input, returns [`SketchError::Deserialization`] if
    /// `bytes` holds a non-empty filter and `bytes.as_ptr()` is not 8-byte
    /// aligned: the view reads the bytes in place as 64-bit words, so it
    /// cannot copy them. (An empty image is rebuilt as an owned filter from
    /// its header alone, so its alignment is not checked.) A `Vec<u8>` from
    /// [`BloomFilter::serialize`](super::BloomFilter::serialize) is in
    /// practice aligned by the allocator, but Rust does not guarantee that;
    /// when alignment cannot be ensured, use
    /// [`BloomFilter::deserialize`](super::BloomFilter::deserialize) or copy
    /// the bytes into an aligned buffer (for example a `Vec<u64>` viewed as
    /// bytes) first.
    ///
    /// Wrapping a non-empty image does not allocate a bit array, but wrapping
    /// an *empty* one does: an empty filter serializes to 24 bytes but records
    /// its capacity, and upstream builds an owned empty filter of that
    /// capacity, up to 2^32 - 64 bits (512 MiB). When wrapping untrusted
    /// bytes, bound the claimed capacity (or the process's memory) yourself.
    pub fn wrap(bytes: &'a [u8]) -> Result<Self, SketchError> {
        let inner = ffi::bloom_wrap(bytes)
            .map_err(|e| SketchError::Deserialization(e.what().to_string()))?;
        Ok(Self {
            inner,
            _bytes: PhantomData,
        })
    }

    wrapped_queries! {
        (u64, "u64", query_u64),
        (i64, "i64", query_i64),
        (u32, "u32", query_u32),
        (i32, "i32", query_i32),
        (u16, "u16", query_u16),
        (i16, "i16", query_i16),
        (u8, "u8", query_u8),
        (i8, "i8", query_i8),
        (f64, "f64", query_f64),
        (f32, "f32", query_f32),
        (&str, "&str", query_str),
        (&[u8], "&[u8]", query_bytes),
    }

    /// Returns `true` if `other` has the same capacity, number of hashes and
    /// seed as this filter, so the two can be combined.
    pub fn is_compatible(&self, other: &impl BloomInput) -> bool {
        match other.as_bloom_input() {
            BloomInputRef::Filter(o) => self.inner.is_compatible_with_filter(o),
            BloomInputRef::Wrapped(o) => self.inner.is_compatible_with_wrapped(o),
        }
    }

    /// Returns `true` if no bits are set.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the filter's capacity in bits.
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

    /// Returns the number of bits set.
    pub fn bits_used(&self) -> u64 {
        self.inner.get_bits_used()
    }

    /// Copies the view into an independent, mutable [`BloomFilter`].
    ///
    /// # Panics
    ///
    /// Only if allocating the copy fails. The copy allocates the full bit
    /// array, which for a view of an empty image can be up to 512 MiB.
    // Named to mirror the upstream API; not the `ToOwned` trait.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_owned(&self) -> BloomFilter {
        BloomFilter {
            inner: self
                .inner
                .to_owned()
                .expect("copying a wrapped Bloom filter failed (out of memory?)"),
        }
    }

    /// Returns a human-readable summary of the filter.
    pub fn to_string_summary(&self) -> String {
        self.inner.to_string_summary()
    }
}
