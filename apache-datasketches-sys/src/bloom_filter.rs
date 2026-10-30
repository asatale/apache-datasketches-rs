//! cxx bridge for datasketches::bloom_filter (owned) and its read-only
//! wrapped view. Free functions carry the `bloom_` prefix because cxx keys
//! their symbols on name alone (see AGENTS.md).

#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("bloom_filter_shim.h");

        type BloomFilterShim;
        type WrappedBloomFilterShim;

        fn bloom_new_by_accuracy(
            max_distinct_items: u64,
            target_fpp: f64,
            seed: u64,
        ) -> Result<UniquePtr<BloomFilterShim>>;
        fn bloom_new_by_size(
            num_bits: u64,
            num_hashes: u16,
            seed: u64,
        ) -> Result<UniquePtr<BloomFilterShim>>;
        fn bloom_deserialize(bytes: &[u8]) -> Result<UniquePtr<BloomFilterShim>>;
        fn bloom_wrap(bytes: &[u8]) -> Result<UniquePtr<WrappedBloomFilterShim>>;
        fn bloom_suggest_num_hashes(max_distinct_items: u64, num_filter_bits: u64) -> Result<u16>;
        fn bloom_suggest_num_hashes_for_fpp(target_fpp: f64) -> Result<u16>;
        fn bloom_suggest_num_filter_bits(max_distinct_items: u64, target_fpp: f64) -> Result<u64>;

        fn clone_shim(self: &BloomFilterShim) -> UniquePtr<BloomFilterShim>;
        fn update_u64(self: Pin<&mut BloomFilterShim>, value: u64);
        fn query_u64(self: &BloomFilterShim, value: u64) -> bool;
        fn query_and_update_u64(self: Pin<&mut BloomFilterShim>, value: u64) -> bool;
        fn update_i64(self: Pin<&mut BloomFilterShim>, value: i64);
        fn query_i64(self: &BloomFilterShim, value: i64) -> bool;
        fn query_and_update_i64(self: Pin<&mut BloomFilterShim>, value: i64) -> bool;
        fn update_u32(self: Pin<&mut BloomFilterShim>, value: u32);
        fn query_u32(self: &BloomFilterShim, value: u32) -> bool;
        fn query_and_update_u32(self: Pin<&mut BloomFilterShim>, value: u32) -> bool;
        fn update_i32(self: Pin<&mut BloomFilterShim>, value: i32);
        fn query_i32(self: &BloomFilterShim, value: i32) -> bool;
        fn query_and_update_i32(self: Pin<&mut BloomFilterShim>, value: i32) -> bool;
        fn update_u16(self: Pin<&mut BloomFilterShim>, value: u16);
        fn query_u16(self: &BloomFilterShim, value: u16) -> bool;
        fn query_and_update_u16(self: Pin<&mut BloomFilterShim>, value: u16) -> bool;
        fn update_i16(self: Pin<&mut BloomFilterShim>, value: i16);
        fn query_i16(self: &BloomFilterShim, value: i16) -> bool;
        fn query_and_update_i16(self: Pin<&mut BloomFilterShim>, value: i16) -> bool;
        fn update_u8(self: Pin<&mut BloomFilterShim>, value: u8);
        fn query_u8(self: &BloomFilterShim, value: u8) -> bool;
        fn query_and_update_u8(self: Pin<&mut BloomFilterShim>, value: u8) -> bool;
        fn update_i8(self: Pin<&mut BloomFilterShim>, value: i8);
        fn query_i8(self: &BloomFilterShim, value: i8) -> bool;
        fn query_and_update_i8(self: Pin<&mut BloomFilterShim>, value: i8) -> bool;
        fn update_f64(self: Pin<&mut BloomFilterShim>, value: f64);
        fn query_f64(self: &BloomFilterShim, value: f64) -> bool;
        fn query_and_update_f64(self: Pin<&mut BloomFilterShim>, value: f64) -> bool;
        fn update_f32(self: Pin<&mut BloomFilterShim>, value: f32);
        fn query_f32(self: &BloomFilterShim, value: f32) -> bool;
        fn query_and_update_f32(self: Pin<&mut BloomFilterShim>, value: f32) -> bool;
        fn update_str(self: Pin<&mut BloomFilterShim>, value: &str);
        fn query_str(self: &BloomFilterShim, value: &str) -> bool;
        fn query_and_update_str(self: Pin<&mut BloomFilterShim>, value: &str) -> bool;
        fn update_bytes(self: Pin<&mut BloomFilterShim>, value: &[u8]);
        fn query_bytes(self: &BloomFilterShim, value: &[u8]) -> bool;
        fn query_and_update_bytes(self: Pin<&mut BloomFilterShim>, value: &[u8]) -> bool;

        fn union_with_filter(
            self: Pin<&mut BloomFilterShim>,
            other: &BloomFilterShim,
        ) -> Result<()>;
        fn union_with_wrapped(
            self: Pin<&mut BloomFilterShim>,
            other: &WrappedBloomFilterShim,
        ) -> Result<()>;
        fn intersect_with_filter(
            self: Pin<&mut BloomFilterShim>,
            other: &BloomFilterShim,
        ) -> Result<()>;
        fn intersect_with_wrapped(
            self: Pin<&mut BloomFilterShim>,
            other: &WrappedBloomFilterShim,
        ) -> Result<()>;
        fn is_compatible_with_filter(self: &BloomFilterShim, other: &BloomFilterShim) -> bool;
        fn is_compatible_with_wrapped(
            self: &BloomFilterShim,
            other: &WrappedBloomFilterShim,
        ) -> bool;
        fn invert(self: Pin<&mut BloomFilterShim>);
        fn reset(self: Pin<&mut BloomFilterShim>);

        fn is_empty(self: &BloomFilterShim) -> bool;
        fn get_capacity(self: &BloomFilterShim) -> u64;
        fn get_num_hashes(self: &BloomFilterShim) -> u16;
        fn get_seed(self: &BloomFilterShim) -> u64;
        fn get_bits_used(self: Pin<&mut BloomFilterShim>) -> u64;
        fn serialized_size_bytes(self: &BloomFilterShim) -> u64;
        fn serialize_into(self: &BloomFilterShim, out: &mut [u8]) -> Result<()>;
        fn to_string_summary(self: &BloomFilterShim) -> String;

        fn query_u64(self: &WrappedBloomFilterShim, value: u64) -> bool;
        fn query_i64(self: &WrappedBloomFilterShim, value: i64) -> bool;
        fn query_u32(self: &WrappedBloomFilterShim, value: u32) -> bool;
        fn query_i32(self: &WrappedBloomFilterShim, value: i32) -> bool;
        fn query_u16(self: &WrappedBloomFilterShim, value: u16) -> bool;
        fn query_i16(self: &WrappedBloomFilterShim, value: i16) -> bool;
        fn query_u8(self: &WrappedBloomFilterShim, value: u8) -> bool;
        fn query_i8(self: &WrappedBloomFilterShim, value: i8) -> bool;
        fn query_f64(self: &WrappedBloomFilterShim, value: f64) -> bool;
        fn query_f32(self: &WrappedBloomFilterShim, value: f32) -> bool;
        fn query_str(self: &WrappedBloomFilterShim, value: &str) -> bool;
        fn query_bytes(self: &WrappedBloomFilterShim, value: &[u8]) -> bool;
        fn is_compatible_with_filter(
            self: &WrappedBloomFilterShim,
            other: &BloomFilterShim,
        ) -> bool;
        fn is_compatible_with_wrapped(
            self: &WrappedBloomFilterShim,
            other: &WrappedBloomFilterShim,
        ) -> bool;
        fn is_empty(self: &WrappedBloomFilterShim) -> bool;
        fn get_capacity(self: &WrappedBloomFilterShim) -> u64;
        fn get_num_hashes(self: &WrappedBloomFilterShim) -> u16;
        fn get_seed(self: &WrappedBloomFilterShim) -> u64;
        fn get_bits_used(self: &WrappedBloomFilterShim) -> u64;
        fn to_owned(self: &WrappedBloomFilterShim) -> Result<UniquePtr<BloomFilterShim>>;
        fn to_string_summary(self: &WrappedBloomFilterShim) -> String;
    }
}
