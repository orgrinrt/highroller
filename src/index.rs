//! The crate's own rolling index: one [`Counter`] at the width and policy the
//! features chose.

use crate::counter::Counter;
use crate::Idx;

/// The policy the crate's own counter follows once its width is used up.
///
/// [`Refuse`](crate::Refuse) under the `strict` feature, [`Wrap`](crate::Wrap)
/// without it.
#[cfg(feature = "strict")]
pub type Policy = crate::counter::Refuse;
/// The policy the crate's own counter follows once its width is used up.
///
/// [`Refuse`](crate::Refuse) under the `strict` feature, [`Wrap`](crate::Wrap)
/// without it.
#[cfg(not(feature = "strict"))]
pub type Policy = crate::counter::Wrap;

/// The largest value the rolling index can hand out.
///
/// Under `strict` this is where it panics instead. Without `strict` it is where
/// it wraps back to zero.
#[allow(non_upper_case_globals)]
pub const _ROLLING_IDX_MAX: Idx = Idx::MAX;

/// The crate's own counter, which [`rolling_idx`] and [`reset_rolling_idx`] are
/// the short forms of.
///
/// It is public so that what the short forms do not cover is still reachable:
/// placing the counter to carry on from a persisted id is
/// [`Counter::set_next`], and there is no short form for it.
pub static ROLLING_IDX: Counter<Idx, Policy> = Counter::new();

/// Returns the current rolling index and then increases it by one.
///
/// The index is ephemeral and specific to one run of the program: it starts at
/// zero every time the process does, and it is not stored anywhere.
///
/// Two calls in the same run give two different values until the width is
/// exhausted. What happens then is the `strict` feature's business: with it,
/// this panics; without it, the index wraps to zero and values start repeating.
///
/// This is [`ROLLING_IDX`]`.next()`, and costs what that costs: one atomic add
/// at every width but `u128`, which keeps a lock.
#[inline]
#[must_use = "an index taken and dropped is an index nobody else can have"]
pub fn rolling_idx() -> Idx {
    ROLLING_IDX.next()
}

/// Puts the rolling index back to zero.
///
/// **Every value handed out before this call can be handed out again.** The
/// index is unique within a run because it only ever moves forward, and this is
/// the one thing that breaks that, so it is for a program that knows the
/// previous run of ids is finished with: a test that wants each case to start
/// from zero, an arena being reused, a phase boundary where nothing from the
/// last phase survives.
///
/// It is not synchronised against readers. A thread calling [`rolling_idx`]
/// while this runs gets a value from one side or the other, and which is not
/// defined.
///
/// ```
/// use highroller::{reset_rolling_idx, rolling_idx};
///
/// let first = rolling_idx();
/// let second = rolling_idx();
/// assert_ne!(first, second);
///
/// reset_rolling_idx();
/// assert_eq!(rolling_idx(), 0, "the index starts again");
/// ```
#[inline]
pub fn reset_rolling_idx() {
    ROLLING_IDX.reset();
}

/// Takes a run of indices into storage the caller supplies.
///
/// The counter is the crate's, but the memory is not: [`Lend`] is notko's
/// contract for storage handed over by whoever obtained it, so this fills a
/// stack array, a slice out of an arena, or a region from an allocator the
/// caller already holds, and never asks where it came from.
///
/// Fills the whole of what it is lent and returns how many that was, which is
/// the storage's capacity. A caller wanting fewer lends a smaller slice.
///
/// `?Sized`, so a bare `&mut [Idx]` out of an arena is lent directly rather
/// than by lending a reference to one. Without it the `Lend for [T]` impl is
/// unreachable here, which is exactly the shape an arena hands out.
///
/// ```
/// # #[cfg(feature = "no_alloc")] {
/// use highroller::{fill_rolling_idx, reset_rolling_idx, Idx};
///
/// reset_rolling_idx();
/// let mut ids = [0 as Idx; 4];
/// let taken = fill_rolling_idx(&mut ids);
///
/// assert_eq!(taken, 4);
/// assert_eq!(ids, [0, 1, 2, 3]);
/// # }
/// ```
///
/// Under `strict` an exhausted index panics here exactly as it does in
/// [`rolling_idx`], because this takes the same values by the same route.
///
/// [`Lend`]: notko::lend::Lend
#[cfg(feature = "no_alloc")]
#[inline]
pub fn fill_rolling_idx<L>(storage: &mut L) -> usize
where
    L: notko::lend::Lend<Idx> + ?Sized,
{
    let mut fill = notko::lend::Fill::new(storage);
    // The id is taken only once there is somewhere to put it. Writing this as
    // `while fill.push(rolling_idx()).is_ok() {}` reads correctly and is not: the
    // argument is evaluated before the call, so the refusal that ends the loop
    // discards an id the counter has already handed out, and every fill leaves
    // a gap of one. The contents are right either way, which is why it took a
    // test that looked at the counter afterwards rather than at the storage.
    while fill.len() < fill.capacity() {
        let _ = fill.push(ROLLING_IDX.next());
    }
    fill.len()
}
