use super::{BloomFilter, WrappedBloomFilter};
use apache_datasketches_sys::bloom_filter::ffi;

/// The two concrete shim types a Bloom filter set operation can read from.
#[doc(hidden)]
pub enum BloomInputRef<'a> {
    /// An owned filter.
    Filter(&'a ffi::BloomFilterShim),
    /// A zero-copy wrapped view.
    Wrapped(&'a ffi::WrappedBloomFilterShim),
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::BloomFilter {}
    impl<'a> Sealed for super::WrappedBloomFilter<'a> {}
}

/// Either Bloom filter type ([`BloomFilter`] or [`WrappedBloomFilter`]) can
/// be passed to [`BloomFilter::union_with`], [`BloomFilter::intersect`] and
/// `is_compatible`. This trait is sealed: the shims only have concrete
/// overloads for these two exact types.
pub trait BloomInput: sealed::Sealed {
    #[doc(hidden)]
    fn as_bloom_input(&self) -> BloomInputRef<'_>;
}

impl BloomInput for BloomFilter {
    fn as_bloom_input(&self) -> BloomInputRef<'_> {
        BloomInputRef::Filter(&self.inner)
    }
}

impl<'a> BloomInput for WrappedBloomFilter<'a> {
    fn as_bloom_input(&self) -> BloomInputRef<'_> {
        BloomInputRef::Wrapped(&self.inner)
    }
}
