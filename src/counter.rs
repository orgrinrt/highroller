//! A rolling counter as a value of its own, at a width and a policy the holder
//! picks.
//!
//! The crate's own [`rolling_idx`](crate::rolling_idx) is one of these, at the
//! width and policy the cargo features chose, and
//! [`declare_rolling_idx!`](crate::declare_rolling_idx) expands to another.
//! Both are conveniences over this type, which is what to reach for
//! when a counter should live inside a struct, be handed around, or be more
//! than one.

use core::fmt;
use core::marker::PhantomData;

use crate::width::Width;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Wrap {}
    impl Sealed for super::Refuse {}
}

/// What a counter does once every value of its width has been handed out.
///
/// Sealed, since there are two answers and a third would have to say what else
/// a counter could do with a value it has already given away.
pub trait Exhaustion: sealed::Sealed + 'static {
    /// Whether an exhausted counter panics rather than repeating a value.
    const REFUSES: bool;
}

/// Return to zero and start repeating values.
///
/// Right where ids only have to be distinct among things alive at the same
/// time, and wrong otherwise. This is what the crate's own counter does without
/// the `strict` feature.
#[derive(Debug)]
pub enum Wrap {}

/// Panic, and keep panicking.
///
/// This is what the crate's own counter does under the `strict` feature. It
/// holds where [`Width::EXHAUSTION_IS_OBSERVABLE`] does: at `u64`, and at
/// `usize` on a 64-bit target, the count is no wider than the width and the
/// counter cannot tell, so it wraps there like the other policy. It is also not
/// reachable at those widths in any run a program could have.
#[derive(Debug)]
pub enum Refuse {}

impl Exhaustion for Wrap {
    const REFUSES: bool = false;
}

impl Exhaustion for Refuse {
    const REFUSES: bool = true;
}

/// A rolling counter over one integer width.
///
/// Hands out `W::ZERO` first and every later value in turn, from as many
/// threads as care to ask, and does what `P` says once the width is used up. It
/// is `const`-constructible, so it sits in a `static` without ceremony, and it
/// is `Sync`, so a shared reference is all a thread needs to take a value.
///
/// ```
/// use highroller::{Counter, Refuse, Wrap};
///
/// // A counter of its own, in a static, refusing rather than repeating.
/// static TICKETS: Counter<u16, Refuse> = Counter::new();
///
/// assert_eq!(TICKETS.next(), 0);
/// assert_eq!(TICKETS.next(), 1);
///
/// // Or inside a value, at whatever width the holder wants. `Wrap` is the default.
/// struct Arena {
///     ids: Counter<u8>,
/// }
/// let arena = Arena { ids: Counter::new() };
/// assert_eq!(arena.ids.next(), 0);
/// let _: Counter<u8, Wrap> = Counter::new();
/// ```
///
/// # What it costs
///
/// At every width up to 64 bits, one relaxed `fetch_add` on a `u64` and a
/// narrowing cast. The count is deliberately wider than the width: passing the
/// width's maximum is then a comparison rather than a second decision the add
/// cannot express, and wrapping is what the cast already does, because every
/// width's range is a power of two. At `u128` there is no wider atomic, so that
/// width keeps a lock and costs what a lock costs.
pub struct Counter<W: Width, P: Exhaustion = Wrap> {
    store:  W::Store,
    policy: PhantomData<P>,
}

impl<W: Width, P: Exhaustion> Counter<W, P> {
    /// The largest value this counter hands out.
    pub const MAX: W = W::MAX;

    /// A counter at zero.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            store:  W::EMPTY,
            policy: PhantomData,
        }
    }

    /// Whether this counter panics once its width is used up, rather than
    /// repeating.
    ///
    /// Fixed by the type, so this is for generic code holding a `Counter<W, P>`
    /// that wants to report which kind it has.
    #[inline]
    #[must_use]
    pub const fn refuses(&self) -> bool {
        P::REFUSES
    }

    /// Whether this counter can tell that its width has been used up.
    ///
    /// Where this is false, [`refuses`](Counter::refuses) is a statement of
    /// policy the counter cannot act on: it wraps, because it has no way of
    /// knowing it should not.
    #[inline]
    #[must_use]
    pub const fn exhaustion_is_observable(&self) -> bool {
        W::EXHAUSTION_IS_OBSERVABLE
    }

    /// Hands out the next value.
    ///
    /// Two calls give two different values until the width is exhausted, and
    /// what happens then is `P`'s business.
    #[inline]
    #[must_use = "a value taken and dropped is a value nobody else can have"]
    pub fn next(&self) -> W {
        W::take(&self.store, P::REFUSES)
    }

    /// Puts the counter back to zero.
    ///
    /// Every value handed out before this call can be handed out again. The
    /// counter is unique within a run because it only ever moves forward,
    /// and this is the one thing that breaks that, so it is for a holder
    /// that knows the previous run of values is finished with: a test that
    /// wants each case to start from zero, an arena being reused,
    /// a phase boundary where nothing from the last phase survives.
    ///
    /// It is not synchronised against readers. A thread calling
    /// [`next`](Counter::next) while this runs gets a value from one side
    /// or the other, and which is not defined.
    #[inline]
    pub fn reset(&self) {
        W::place(&self.store, W::ZERO);
    }

    /// Puts the counter where the next value it hands out is `at`.
    ///
    /// A reset to somewhere other than zero, with the same caveats, and the
    /// hook for a program that persists its last id across runs: read it
    /// back, place the counter one past it, and the sequence carries on
    /// where the previous run stopped. Placing it at [`MAX`](Counter::MAX)
    /// leaves exactly one value before exhaustion, which is how the
    /// exhaustion tests reach the end of a 64-bit width in this decade.
    ///
    /// ```
    /// use highroller::Counter;
    ///
    /// let ids: Counter<u32> = Counter::new();
    /// ids.set_next(1_000);
    /// assert_eq!(ids.next(), 1_000);
    /// assert_eq!(ids.next(), 1_001);
    /// ```
    #[inline]
    pub fn set_next(&self, at: W) {
        W::place(&self.store, at);
    }
}

impl<W: Width, P: Exhaustion> Default for Counter<W, P> {
    /// The same as [`new`](Counter::new): a counter at zero.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<W: Width, P: Exhaustion> fmt::Debug for Counter<W, P> {
    /// Names the width and the policy, and not the count: reading it would be a
    /// value nobody took, and printing a counter should not race the
    /// threads using it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Counter")
            .field("width", &core::any::type_name::<W>())
            .field("refuses", &P::REFUSES)
            .finish_non_exhaustive()
    }
}
