//! Declaring a second rolling index, at a width and a policy the caller picks.

/// Declares a rolling index and its counter at the invocation site.
///
/// The crate's own [`rolling_idx`](crate::rolling_idx) is one counter at one
/// width, and the width is a cargo feature. A feature is chosen once for a
/// whole build graph, so it cannot give a program two counters, and it cannot
/// give a library one that its consumer does not also have to agree to. This
/// macro can: invoke it once per counter, in a module per counter, at whatever
/// width each one wants.
///
/// It generates, at the invocation site:
///
/// - `ROLLING_IDX`, the [`Counter`](crate::Counter) itself, for what the
///   functions below do not cover,
/// - `_ROLLING_IDX_MAX`, the largest value this counter hands out,
/// - `rolling_idx()`, which returns the current value and advances,
/// - `reset_rolling_idx()`, which puts it back to zero.
///
/// Those are the same names the crate exports for its own counter, so a module
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
/// # Exhaustion
///
/// A second argument names the policy, in the words the crate's own feature
/// uses: `strict` refuses with a panic once the width is used up, and `wrap`
/// returns to zero and starts repeating. Left out, it wraps. The crate-level
/// `strict` feature does not reach a counter declared here, because a macro
/// expanding in someone else's crate cannot read their features, so the choice
/// is made at the invocation instead.
///
/// ```
/// mod handles {
///     highroller::declare_rolling_idx!(u8, strict);
/// }
///
/// assert!(handles::ROLLING_IDX.refuses());
/// # fn takes_a_width(_: u8) {}
/// takes_a_width(handles::rolling_idx());
/// ```
///
/// # Widths
///
/// `u8`, `u16`, `u32`, `u64`, `usize` and `u128`. The last has no wider atomic
/// to count in and keeps a lock, which is a cost the other five do not pay;
/// [`Counter`](crate::Counter) says what each costs.
///
/// # What it is
///
/// A `static` [`Counter`](crate::Counter) and three thin wrappers. A counter
/// that should live somewhere other than a module, inside a struct or behind a
/// reference, is that type used directly.
#[macro_export]
macro_rules! declare_rolling_idx {
    ($t:ty) => {
        $crate::declare_rolling_idx!($t, wrap);
    };
    ($t:ty, wrap) => {
        $crate::__declare_rolling_idx!($t, $crate::Wrap);
    };
    ($t:ty, strict) => {
        $crate::__declare_rolling_idx!($t, $crate::Refuse);
    };
}

/// The expansion behind [`declare_rolling_idx!`], with the policy already a
/// type.
#[doc(hidden)]
#[macro_export]
macro_rules! __declare_rolling_idx {
    ($t:ty, $policy:ty) => {
        /// The largest value this rolling index hands out.
        #[allow(non_upper_case_globals, dead_code)]
        pub const _ROLLING_IDX_MAX: $t = <$t>::MAX;

        /// The counter behind `rolling_idx` and `reset_rolling_idx`, for what those do
        /// not cover.
        #[allow(dead_code)]
        pub static ROLLING_IDX: $crate::Counter<$t, $policy> = $crate::Counter::new();

        /// Returns the current rolling index and then advances it by one.
        ///
        /// Ephemeral and specific to one run: it starts at zero every time the process
        /// does, and it is not stored anywhere. Two calls give two different values
        /// until the width is used up, and what happens then is the policy this counter
        /// was declared with.
        #[allow(dead_code)]
        #[inline]
        #[must_use = "an index taken and dropped is an index nobody else can have"]
        pub fn rolling_idx() -> $t {
            ROLLING_IDX.next()
        }

        /// Puts this rolling index back to zero.
        ///
        /// **Every value handed out before this call can be handed out again.** For a
        /// caller that knows the previous run of ids is finished with: a test starting
        /// each case from zero, an arena being reused, a phase boundary nothing
        /// survives.
        #[allow(dead_code)]
        #[inline]
        pub fn reset_rolling_idx() {
            ROLLING_IDX.reset();
        }
    };
}
