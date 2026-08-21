//! Declaring a second rolling index, at a width the caller picks.

/// Declares a rolling index and its counter at the invocation site.
///
/// The crate's own [`rolling_idx`](crate::rolling_idx) is one counter at one width, and the
/// width is a cargo feature. A feature is chosen once for a whole build graph, so it cannot
/// give a program two counters, and it cannot give a library one that its consumer does not
/// also have to agree to. This macro can: invoke it once per counter, in a module per
/// counter, at whatever width each one wants.
///
/// It generates, at the invocation site:
///
/// - `_ROLLING_IDX_MAX`, the largest value this counter hands out,
/// - `rolling_idx()`, which returns the current value and advances,
/// - `reset_rolling_idx()`, which puts it back to zero.
///
/// Those are the same three names the crate exports for its own counter, so a module
/// declaring one reads the same way as the crate does.
///
/// ```
/// mod tickets {
///     highroller::declare_rolling_idx!(u16);
/// }
///
/// mod sessions {
///     highroller::declare_rolling_idx!(u32);
/// }
///
/// // Two counters, two widths, neither aware of the other.
/// assert_eq!(tickets::rolling_idx(), 0);
/// assert_eq!(tickets::rolling_idx(), 1);
/// assert_eq!(sessions::rolling_idx(), 0);
/// assert_eq!(tickets::_ROLLING_IDX_MAX, u16::MAX);
/// ```
///
/// # What it costs
///
/// One relaxed `fetch_add` on a `u64`, and a narrowing cast. The counter is wider than the
/// index on purpose: passing the index's maximum is then a comparison rather than a second
/// decision the add cannot express, and wrapping is what the narrowing cast already does,
/// because every width's range is a power of two.
///
/// An earlier version of this macro kept a `Mutex` per counter. On this machine that is
/// 8.9ns against 2.0ns uncontended, and 583µs against 195µs at eight threads;
/// `benches/rolling.rs` carries the measurement with the mutex kept as an arm.
///
/// # Widths
///
/// `u8`, `u16`, `u32`, `u64` and `usize`. A 128-bit index has no wider atomic to count in,
/// so it is refused here rather than silently given a lock; the crate's own `u128_index`
/// feature is where that width lives, and it says what it costs.
///
/// # Exhaustion
///
/// Wraps to zero, and values start repeating. The crate-level `strict` feature does not
/// reach a counter declared here: it is a property of the crate's own counter, and a macro
/// expanding in someone else's crate cannot read their features. A caller wanting to refuse
/// instead compares against `_ROLLING_IDX_MAX` at the call site.
#[macro_export]
macro_rules! declare_rolling_idx {
    (u128) => {
        ::core::compile_error!(
            "declare_rolling_idx!: a 128-bit index has no wider atomic to count in, so it \
             cannot use the counter this macro generates. The crate's own `u128_index` \
             feature carries that width, with a lock and a note saying what it costs."
        );
    };
    ($t:ty) => {
        /// The largest value this rolling index hands out before it wraps.
        #[allow(non_upper_case_globals, dead_code)]
        pub const _ROLLING_IDX_MAX: $t = <$t>::MAX;

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static __ROLLING_IDX_COUNTER: ::core::sync::atomic::AtomicU64 =
            ::core::sync::atomic::AtomicU64::new(0);

        /// Returns the current rolling index and then advances it by one.
        ///
        /// Ephemeral and specific to one run: it starts at zero every time the process
        /// does, and it is not stored anywhere. Two calls give two different values until
        /// the width is used up, after which it wraps and values repeat.
        #[allow(dead_code)]
        #[inline]
        pub fn rolling_idx() -> $t {
            let previous = __ROLLING_IDX_COUNTER
                .fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
            // The narrowing cast is the wrap. Every width's range is a power of two, so
            // reducing the wide counter modulo that range is what the cast already does.
            previous as $t
        }

        /// Puts this rolling index back to zero.
        ///
        /// **Every value handed out before this call can be handed out again.** For a
        /// caller that knows the previous run of ids is finished with: a test starting each
        /// case from zero, an arena being reused, a phase boundary nothing survives.
        #[allow(dead_code)]
        #[inline]
        pub fn reset_rolling_idx() {
            __ROLLING_IDX_COUNTER.store(0, ::core::sync::atomic::Ordering::SeqCst);
        }
    };
}
