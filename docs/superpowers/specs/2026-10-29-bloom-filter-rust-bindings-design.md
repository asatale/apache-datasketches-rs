# Apache DataSketches Bloom Filter Rust Bindings — Design

## Purpose

Add the Bloom filter family — approximate set membership — to
`apache-datasketches`/`apache-datasketches-sys`, binding upstream
datasketches-cpp 5.3.0's `bloom_filter` (`filters/include/`). It is the second
of the four families planned after KLL (REQ, Bloom filter, then Count-Min and
t-digest), each with its own spec → plan → implementation cycle.

This design follows the crate's established three-layer pattern (C++ shim →
cxx bridge → safe wrapper) and the conventions settled by the HLL, CPC, KLL
and REQ families; anything not mentioned here (error-before-FFI validation,
validate-before-upstream deserialization hardening, committed fuzz test,
bench pair cross-check, doc structure) carries over.

**Scope:** `bloom_filter` (an owned, mutable filter) and a read-only,
zero-copy wrapped view over serialized bytes.

**Out of scope:** `writable_wrap` (mutating caller-owned memory),
`initialize_by_accuracy`/`initialize_by_size` (constructing into caller
memory), stream serialization, custom allocators.

## Upstream facts this design rests on

Checked against the vendored 5.3.0 headers:

- `bloom_filter` is `bloom_filter_alloc<std::allocator<uint8_t>>` — not
  templated on an item type, so it binds as one concrete shim type (like HLL
  and CPC), with no sealed item trait.
- Construction is through `bloom_filter::builder`:
  `create_by_accuracy(max_distinct_items, target_false_positive_prob, seed)`
  and `create_by_size(num_bits, num_hashes, seed)`, plus static helpers
  `suggest_num_hashes(max_distinct_items, num_filter_bits)`,
  `suggest_num_hashes(target_fpp)` and
  `suggest_num_filter_bits(max_distinct_items, target_fpp)`. The builder
  throws `std::invalid_argument` for `max_distinct_items == 0`, `num_bits == 0`
  or `>= 2^63`, `num_hashes == 0`, and `target_fpp` not in `(0, 1]`; the
  constructor also rejects more than `MAX_FILTER_SIZE_BITS`
  (`(INT32_MAX - 32) * 8`) bits.
- The default seed when none is given is **random** per filter
  (`generate_random_seed()`).
- `update`, `query` (const) and `query_and_update` take `uint64_t`,
  `uint32_t`, `uint16_t`, `uint8_t`, `int64_t`, `int32_t`, `int16_t`,
  `int8_t`, `double`, `float`, `std::string` and `(const void*, size_t)`.
- `union_with`, `intersect` throw `std::invalid_argument("Incompatible bloom
  filters")` unless `is_compatible` (same capacity, number of hashes and
  seed); `invert`, `reset` exist; accessors `get_capacity`,
  `get_num_hashes`, `get_seed`, `is_empty` are `const`.
- **`get_bits_used()` is not `const`**: after updates it lazily recounts the
  set bits and caches the count (`is_dirty_`, `num_bits_set_`). No other
  `const` member function mutates state (there are no `mutable` members).
- `wrap(bytes, len)` returns a read-only filter over the caller's buffer
  (zero-copy); queries and accessors work; updates throw on a read-only
  filter.
- `deserialize(bytes, len)` (and `wrap`) checks: minimum length, preamble
  longs in `[3, 4]`, serial version, family id, that a non-empty image holds
  the full bit array (`ensure_minimum_memory`), and that the stored
  bits-set count matches the actual count (or is the "dirty" sentinel).
  The constructor rejects `num_hashes == 0`, zero bits and oversize filters.
- An **empty** image is 24 bytes but carries `num_longs` (capacity); upstream
  allocates the full bit array for it. Because of the 32-bit shift below,
  that is at most 2^32 - 64 bits (**512 MiB**, not ~2 GiB as first
  assumed). A genuine large empty filter serializes exactly this way.

Found during the hardening task (Task 4), all in 5.3.0
`bloom_filter_impl.hpp`:

- **32-bit size arithmetic / round-trip limit:** `num_longs` is serialized
  as `int32_t` (:438, :468) but read as `uint32_t` (:385), and the capacity
  and byte count are computed as `num_longs << 6` and `num_longs << 3` in 32
  bits (:392, :398, :420). From 2^26 words the capacity wraps: multiples of
  2^29 (including negative `int32_t` values) give capacity 0 with a 0-byte
  array, and `num_longs = 0` does the same. On such a filter `update` /
  `query_and_update` compute `hash % 0` (:651, :749). On arm64 that is not
  SIGFPE: `x % 0 == x`, which makes a wild heap write (SIGSEGV observed).
  Hence upstream can build filters up to `(INT32_MAX - 32) * 8` bits but
  only read back up to **2^32 - 64 bits**. This crate caps the builder
  there, so every filter it builds round-trips.
- **65535 hash functions hang:** every hashing loop is `for (uint16_t i = 1;
  i <= num_hashes_; i++)` (:650, :748, :845), which never terminates for
  65535. The builder, `suggest_num_hashes` and the validator cap
  `num_hashes` at 65534.
- **Bits-set count bug (false negatives):** `update` marks the cached count
  dirty (:644-654), but `query_and_update` adds to the stale cached count
  and clears the dirty flag without recounting (:742-755 via
  `update_num_bits_set` :552-558). After `update(x)` then
  `query_and_update(x)`, the count stays 0, `is_empty()` is true and
  `query(x)` returns false (:843). `bits_used` is also wrong, and serialize
  writes a count upstream's own deserialize rejects. **Workaround:** the shim's
  `update_*` call `query_and_update` and discard the result. The same bits
  are set and the count stays exact, so the filter never goes dirty.
- **Preamble-3 over-read:** a non-empty image claiming 3 preamble longs
  passes `ensure_minimum_memory(len, 24)` (:376). Upstream then reads the
  count and bit array past the buffer (:396-399), because the negative
  `end_ptr - ptr` converts to a huge `size_t`. For `wrap` that gives a view
  whose bit array lies outside the borrowed bytes.
- A non-empty image with `num_hashes == 0` bypasses the constructor check
  and answers every query true. Flag bits other than "empty" are ignored.
- Serialization is deterministic given the seed; there is no randomness in
  updates or queries.
- Upstream's `filters/test/` has no checked-in `.sk` fixtures.

## Architecture

### Feature and vendoring

- New `bloom` feature on both crates; the safe crate adds it to `default`,
  `[package.metadata.docs.rs]`, and the no-family `compile_error!` guard
  (keeping the phrase `no sketch family is enabled`).
- Vendor `filters/include/` into
  `apache-datasketches-sys/vendor/datasketches-cpp` and update the vendor
  README copy recipe, `build.rs`'s `.include(...)` list and the drift test's
  module list.

### C++ shim

`apache-datasketches-sys/cpp/bloom/bloom_filter_shim.{h,cc}`:

- `BloomFilterShim` owns a `datasketches::bloom_filter`.
- `WrappedBloomFilterShim` holds the read-only filter returned by
  `bloom_filter::wrap` over bytes owned by Rust (the safe wrapper guarantees
  the bytes outlive it via a lifetime).
- Free functions, prefixed `bloom_`: constructors by accuracy and by size
  (taking an explicit seed), `bloom_deserialize`, `bloom_wrap`, and the three
  `suggest_*` helpers.
- `validate_bloom_layout(bytes)` runs before upstream in `bloom_deserialize`
  and `bloom_wrap` (see "Hardening").
- Methods for every update/query/query-and-update overload, set operations
  (returning `Result` through cxx because upstream throws on incompatible
  filters), `invert`, `reset`, accessors, `serialize`, `to_string`.

### Bridge

`apache-datasketches-sys/src/bloom_filter.rs`, one bridge (no item-type
duplication). Calls that can throw on user or untrusted input return
`Result`.

### Safe layer

`apache-datasketches/src/bloom/`:

| File | Contents |
|---|---|
| `mod.rs` | module docs, re-exports, `DEFAULT_SEED` |
| `builder.rs` | `BloomFilterBuilder` and the `suggest_*` helpers |
| `filter.rs` | `BloomFilter` |
| `wrapped.rs` | `WrappedBloomFilter<'a>` |
| `input.rs` | sealed `BloomInput` trait implemented by both filter types |

## Public API

```rust
/// The seed used when none is given (upstream's datasketches DEFAULT_SEED).
pub const DEFAULT_SEED: u64 = 9001;

pub struct BloomFilterBuilder;
impl BloomFilterBuilder {
    pub fn by_accuracy(max_distinct_items: u64, target_false_positive_prob: f64) -> Self;
    pub fn by_size(num_bits: u64, num_hashes: u16) -> Self;
    pub fn seed(self, seed: u64) -> Self;
    pub fn build(self) -> Result<BloomFilter, SketchError>;

    pub fn suggest_num_hashes(max_distinct_items: u64, num_filter_bits: u64)
        -> Result<u16, SketchError>;
    pub fn suggest_num_hashes_for_fpp(target_false_positive_prob: f64)
        -> Result<u16, SketchError>;
    pub fn suggest_num_filter_bits(max_distinct_items: u64, target_false_positive_prob: f64)
        -> Result<u64, SketchError>;
}

pub struct BloomFilter;                     // Clone + Send + Sync
impl BloomFilter {
    // update_* / query_* / query_and_update_* for:
    // u64, i64, u32, i32, u16, i16, u8, i8, f64, f32, str, bytes
    pub fn update_u64(&mut self, value: u64);
    pub fn query_u64(&self, value: u64) -> bool;
    pub fn query_and_update_u64(&mut self, value: u64) -> bool;
    // ... (same three for each type)

    pub fn union_with(&mut self, other: &impl BloomInput) -> Result<(), SketchError>;
    pub fn intersect(&mut self, other: &impl BloomInput) -> Result<(), SketchError>;
    pub fn is_compatible(&self, other: &impl BloomInput) -> bool;
    pub fn invert(&mut self);
    pub fn reset(&mut self);

    pub fn is_empty(&self) -> bool;
    pub fn capacity(&self) -> u64;          // bits
    pub fn num_hashes(&self) -> u16;
    pub fn seed(&self) -> u64;
    pub fn bits_used(&mut self) -> u64;

    pub fn serialize(&self) -> Vec<u8>;
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError>;
    pub fn to_string_summary(&self) -> String;
}

pub struct WrappedBloomFilter<'a>;          // Send + Sync; borrows the bytes
impl<'a> WrappedBloomFilter<'a> {
    pub fn wrap(bytes: &'a [u8]) -> Result<Self, SketchError>;
    // query_* for the same types
    pub fn query_u64(&self, value: u64) -> bool;
    // ...
    pub fn is_compatible(&self, other: &impl BloomInput) -> bool;
    pub fn is_empty(&self) -> bool;
    pub fn capacity(&self) -> u64;
    pub fn num_hashes(&self) -> u16;
    pub fn seed(&self) -> u64;
    pub fn bits_used(&self) -> u64;
    pub fn to_owned(&self) -> BloomFilter;
    pub fn to_string_summary(&self) -> String;
}
```

Notes:

- **Seed:** every filter uses `DEFAULT_SEED` (9001, the value Theta and Tuple
  already use) unless `.seed(x)` is called, so separately built filters with
  the same size parameters are compatible by default (the common case:
  per-shard filters merged centrally). The rustdoc on `seed` notes that a
  known seed lets an adversary craft colliding inputs, and that a random
  seed defends against that. Deserialized and wrapped filters keep the seed
  they were serialized with.
- **Builder validation in Rust:** `max_distinct_items > 0`; `target_fpp` in
  `(0, 1)` and not NaN (upstream allows exactly 1.0, which is a filter that
  rejects nothing — rejected here as a configuration error). `num_bits` must
  be in `1..=2^32 - 64`, and so must the size `by_accuracy` and
  `suggest_num_filter_bits` compute and the bits argument of
  `suggest_num_hashes`. Upstream's serialized format cannot read back larger
  filters (32-bit size arithmetic, see above). The limit can be raised once
  upstream fixes that. `num_hashes` must be in `1..=65534`, and so must
  `suggest_num_hashes`'s result. Upstream's `suggest_*` casts are unchecked
  (UB out of range), so the formulas are recomputed in Rust first.
  Violations return `InvalidConfig`.
- **`union_with`/`intersect`** check `is_compatible` in Rust first and
  return `InvalidConfig` naming the mismatched parameter; the bridge still
  returns `Result` as a backstop.
- **`bits_used(&mut self)`** on `BloomFilter` because upstream recounts and
  caches through a non-`const` method; this keeps `BloomFilter` honestly
  `Sync`. `WrappedBloomFilter::bits_used` takes `&self`: a read-only wrapped
  filter is never dirty (it cannot be updated), so upstream never writes the
  cache there; the shim documents this invariant.
- **`BloomInput`** is sealed (implemented for `BloomFilter` and
  `WrappedBloomFilter`), so set operations and `is_compatible` accept
  either — the same pattern as Theta's `ThetaInput`.
- **`to_owned`** deserializes from the same bytes into an owned filter (one
  copy).
- **`str`/bytes updates** forward borrowed `(ptr, len)` to upstream's
  `(const void*, size_t)` overloads (no allocation), matching the crate's
  other families; an empty string is ignored consistently with upstream's
  `std::string` overload (verify and match).

### Thread safety

`BloomFilter` is `Send + Sync`: every `&self` method maps to a `const`
upstream method with no hidden mutation; the one caching method is
`&mut self`. `WrappedBloomFilter<'a>` is `Send + Sync` for the same reason
plus the read-only invariant; its lifetime ties it to the borrowed bytes.
Compile-time tests pin `Sync` for both and the borrow (a
`compile_fail` doctest that drops the bytes while a view is alive).

## Errors

| Condition | Error |
|---|---|
| `max_distinct_items == 0`; `target_fpp` NaN or outside `(0, 1)`; `num_bits` (given or computed) 0 or above 2^32 - 64; `num_hashes` 0 or 65535 | `InvalidConfig` |
| `suggest_*` with invalid arguments | `InvalidConfig` |
| `union_with` / `intersect` with an incompatible filter | `InvalidConfig` |
| bytes that do not parse or that `validate_bloom_layout` rejects (`deserialize`, `wrap`) | `Deserialization` |
| any other C++ exception | `Cpp` |

## Hardening

Planned as its own task, as for REQ:

1. **Audit** upstream's bytes paths (`internal_deserialize_or_wrap`, the
   constructors it calls, `bit_array_ops`) and the operations reachable on a
   deserialized or wrapped filter (queries, set operations, `to_owned`,
   serialize) for anything crafted input can reach that 5.3.0's checks miss.
2. **`validate_bloom_layout`** in the shim, bounds-checked, before upstream:
   minimum length; preamble longs and flag bits; `num_hashes >= 1`;
   `num_hashes <= 65534`; `num_longs` in `1..=2^26 - 1` (capacity at most
   2^32 - 64 bits, the 32-bit shift limit); preamble 3 exactly when empty; for a
   non-empty image, the bit array fully present; trailing-byte policy
   matching what upstream itself writes. Plus anything the audit finds.
3. **Empty-image capacity (decided):** accept it as upstream does — a
   genuine large empty filter serializes to 24 bytes — and state clearly in
   the `deserialize` and `wrap` rustdoc that an empty image can make upstream
   allocate up to 512 MiB, so callers handling untrusted input should bound
   it (e.g. check the claimed capacity or cap process memory).
4. **Committed fuzz regression test** (`bloom_deserialize_fuzz_test.rs`):
   deterministic, fixed-seed mutation of genuine filters (empty, sparse,
   dense, several sizes and seeds); every accepted mutant goes through
   queries, `union_with`/`intersect` with a compatible genuine filter,
   `serialize`→`deserialize`, and `to_owned` (for `wrap`) without aborting.
   Mutations that would claim huge empty capacities are bounded so the test
   stays fast. It must fail if the validator is removed, or the task reports
   that upstream's own checks already suffice.

## Testing

All in `apache-datasketches/tests/` unless noted.

- **`bloom_filter_smoke_test.rs`** — happy paths for every overload family.
- **`bloom_filter_test.rs`** — port of upstream
  `filters/test/bloom_filter_test.cpp` (10 `TEST_CASE`s: invalid constructor
  args, standard constructors, basic operations, inversion, incompatible set
  operations, basic union, basic intersection, empty serialization,
  non-empty serialization, inconsistent num bits set is rejected).
  Allocation and stream tests are skipped and named in the header;
  `bit_array_ops_test.cpp` (internal) is not ported.
- **`bloom_validation_test.rs`** — every row of the error table.
- **`bloom_serialization_compat_test.rs`** — golden bytes derived by hand
  from upstream `serialize` (empty and non-empty, default and custom seed),
  round trips, and `wrap` agreeing with `deserialize`.
- **`bloom_wrapped_test.rs`** — zero-copy queries equal the owned filter's;
  `to_owned`; set operations accepting a wrapped filter as the argument;
  the lifetime `compile_fail` doctest lives on `WrappedBloomFilter`.
- **`bloom_concurrency_test.rs`** — `BloomFilter` shared through `Arc` and
  queried concurrently; per-thread filters unioned.
- **`bloom_deserialize_fuzz_test.rs`** — see "Hardening".
- **`apache-datasketches-sys/tests/bloom_filter_link_test.rs`**.

Exact assertions are allowed throughout: the filter is deterministic given
its seed. False-positive-rate checks use the configured probability with a
statistical margin over enough queries, with a fixed seed so they are
reproducible.

## Benchmarks

`apache-datasketches/examples/bench_bloom_update.rs` and
`benches/cpp_reference/bloom_update.cc`: same `max_distinct_items`, target
fpp and seed on both sides; scenarios `distinct` (updates), `query` (half
hits, half misses), `str`, `union`, `ser`, `deser`; same `--reps`/`--ladder`
handling. The cross-check is exact (no randomness): each scenario prints
bits used, the query hit count where applicable, and serialized bytes;
`compare.py` diffs the labelled values generically. Registered in `run.sh`
and `compare.sh`; documented in the bench README (noting that the Rust
`deser`/wrap path pays for the shim's validation).

## CI

Add `--no-default-features --features bloom` to the feature matrix and
update the default-leg family count. The `benches` job picks up the pair.

## Documentation

Shaped like the other families:

- `apache-datasketches/README.md`: a "Bloom filters" section with a snippet,
  a per-type list (`BloomFilterBuilder`, `BloomFilter`,
  `WrappedBloomFilter`, `BloomInput`), a "what differs" list (membership not
  a sketch estimate; the default seed and its security note; `bits_used`
  takes `&mut self`; set operations need compatible filters; the empty-image
  allocation warning), and the example's coverage; families list and count.
- Crate docs in `lib.rs`; `bloom/mod.rs` module docs with a doctest.
- `apache-datasketches-sys/README.md`: `bloom` bullet and toml snippet.
- Root `README.md`: families list, example command, count.
- `AGENTS.md`: families, commands, vendored-modules list, a short Bloom
  paragraph.
- `apache-datasketches/examples/bloom.rs`.
- `CHANGELOG.md`: an entry under `[Unreleased]`; and correct the 0.3.0
  heading's date from 2026-10-27 to **2026-10-29** (the actual publish
  date).

## Release

Additive; goes under `[Unreleased]`. Versions are bumped and crates
published only with the maintainer's go-ahead, then tagged per the repo's
release rule.
