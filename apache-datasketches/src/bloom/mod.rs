//! Bloom filters: approximate set membership with no false negatives.
//!
//! A Bloom filter answers "may this item have been added?". A `false` answer
//! is always correct; a `true` answer is wrong at roughly the configured
//! false-positive rate.
//!
//! ```
//! use apache_datasketches::bloom::{BloomFilterBuilder, WrappedBloomFilter};
//!
//! let mut filter = BloomFilterBuilder::by_accuracy(1_000, 0.01).build().unwrap();
//! for i in 0..100u64 {
//!     filter.update_u64(i);
//! }
//! assert!(filter.query_u64(42));
//! assert!(!filter.query_u64(1_000_000));
//!
//! // Serialize, then query the bytes in place without a full deserialize.
//! let bytes = filter.serialize();
//! let view = WrappedBloomFilter::wrap(&bytes).unwrap();
//! assert!(view.query_u64(42));
//! ```
//!
//! The types:
//!
//! - [`BloomFilterBuilder`] — configures and builds a filter.
//! - [`BloomFilter`] — the owned, mutable filter.
//! - [`WrappedBloomFilter`] — a zero-copy, read-only view over serialized
//!   bytes.
//! - [`BloomInput`] — sealed trait letting either filter type be an argument
//!   to set operations.

mod builder;
mod filter;
mod input;
mod wrapped;

pub use builder::BloomFilterBuilder;
pub use filter::BloomFilter;
pub use input::BloomInput;
pub use wrapped::WrappedBloomFilter;

/// The default hash seed used when [`BloomFilterBuilder::seed`] is not
/// called. Filters must share a seed to be combined.
pub const DEFAULT_SEED: u64 = 9001;

/// Largest number of hash functions that works upstream: its hashing loops
/// (`for (uint16_t i = 1; i <= num_hashes_; i++)`, bloom_filter_impl.hpp
/// :650, :748, :845) never terminate for `u16::MAX`.
pub(crate) const MAX_NUM_HASHES: u16 = u16::MAX - 1;

/// Largest filter size, in bits, that upstream accepts.
pub(crate) const MAX_FILTER_SIZE_BITS: u64 = (i32::MAX as u64 - 32) * 8;
