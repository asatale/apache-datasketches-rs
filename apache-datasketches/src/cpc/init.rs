use apache_datasketches_sys::cpc_sketch::ffi as sys;

/// Eagerly initializes CPC's global decompression tables, used during
/// serialization/deserialization.
///
/// Upstream lazily self-initializes these tables on first use, and that
/// lazy path is **not thread-safe**: if two threads race to serialize or
/// deserialize a [`crate::cpc::CpcSketch`] for the first time concurrently,
/// initializing the shared global state is a data race. Call `init()` once,
/// single-threaded, before spawning worker threads that will serialize or
/// deserialize CPC sketches concurrently. Single-threaded callers never
/// need to call this — the lazy self-init is fine there.
pub fn init() {
    sys::cpc_init();
}
