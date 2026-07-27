use apache_datasketches_sys::cpc_sketch::ffi as sys;

/// Eagerly initializes CPC's global decompression tables, used during
/// serialization/deserialization.
///
/// Upstream lazily self-initializes these tables on first use via a
/// function-local static with a dynamic initializer; C++11 onward
/// guarantees this lazy path is initialized exactly once even under
/// concurrent access (concurrent callers block until the first caller
/// finishes, they don't race), so there is no correctness hazard from
/// skipping `init()`. Calling it eagerly is a latency optimization: it
/// moves the one-time cost of allocating and building the tables off the
/// hot path, and avoids every thread that happens to hit the lazy path
/// first stalling behind whichever one wins the initialization race.
pub fn init() {
    sys::cpc_init();
}
