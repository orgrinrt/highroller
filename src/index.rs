//! The rolling index itself: one counter, and the two policies for running out.

use crate::Idx;

/// The largest value the rolling index can hand out.
///
/// Under `strict` this is where it panics instead. Without `strict` it is where it
/// wraps back to zero.
#[allow(non_upper_case_globals)]
pub const _ROLLING_IDX_MAX: Idx = Idx::MAX;

// The counter is deliberately wider than the index, and that is what makes a single
// `fetch_add` sufficient.
//
// Handing out an index is a read and an increment, which is an atomic add, except
// that the index also has to wrap or refuse at its own maximum. Doing that on a
// counter of the same width needs a compare-and-swap loop, because the wrap is a
// second decision the add cannot express. A wider counter removes the decision: it
// passes the narrow maximum long before it could wrap itself, so exhaustion is a
// comparison, and wrapping is a mask, because every index width's range is a power
// of two.
//
// Measured on this machine, uncontended: a mutex 8.9ns, a compare-and-swap loop
// 2.5ns, this 2.0ns. At eight threads: 583us, 531us, 195us. The compare-and-swap
// loop is the interesting one, because it is the obvious replacement for a mutex
// and it is barely better than one under contention: every conflicting thread
// retries, so the retries are the work. `benches/rolling.rs` carries all of it,
// with the mutex kept as an arm, and the last row measures this function rather
// than a copy of it written for the benchmark.
#[cfg(not(feature = "u128_index"))]
mod counter {
    use super::Idx;
    #[cfg(any(feature = "strict", test))]
    use super::_ROLLING_IDX_MAX;
    use core::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Whether the counter is genuinely wider than the index.
    ///
    /// Asked of the width, because the width is the question. An earlier version asked
    /// whether the feature was named `usize_index`, which is the same thing only on a
    /// 64-bit target: on wasm32 and i686 a `usize` is 32 bits, the counter really is
    /// wider, and naming the feature compiled the check out anyway. `strict` then meant
    /// nothing there, and worse, the cast below truncated and handed out an index already
    /// in use. The mutex this replaced detected exhaustion at every width.
    ///
    /// Where it is false the check would be a comparison against the counter's own
    /// maximum, whose answer is fixed at compile time; clippy denies that, correctly. It
    /// is also unreachable in practice: a thousand million ids a second exhausts a 64-bit
    /// index in about five hundred years.
    #[cfg(feature = "strict")]
    const EXHAUSTION_IS_OBSERVABLE: bool =
        core::mem::size_of::<Idx>() < core::mem::size_of::<u64>();

    #[inline]
    pub(super) fn next() -> Idx {
        let prev = COUNTER.fetch_add(1, Ordering::Relaxed);

        #[cfg(feature = "strict")]
        if EXHAUSTION_IS_OBSERVABLE {
            // The wide counter has not wrapped and will not, so passing the narrow
            // maximum is exhaustion and every later caller sees it too. That is the
            // property a same-width `fetch_add` cannot offer: there, the wrap to zero
            // is indistinguishable from a fresh counter, and a thread arriving in that
            // window would be handed an index already in use.
            if u128::from(prev) > _ROLLING_IDX_MAX as u128 {
                panic!(
                    "highroller: the rolling index is exhausted. All {} values of the \
                     configured width have been handed out, and the `strict` feature \
                     asks to stop rather than to reuse one. Either widen the index or \
                     turn `strict` off to wrap.",
                    _ROLLING_IDX_MAX as u128 + 1
                );
            }
        }

        // Wrapping is the truncation, and nothing more. Every width's range is a power
        // of two, so reducing the wide counter modulo that range is what a narrowing
        // cast already does. An `& WRAP` was here first and was dead: masking to 255
        // and then casting to `u8` is casting to `u8`. A mutation test found it by
        // widening the mask and watching nothing go red.
        prev as Idx
    }

    #[cfg(test)]
    pub(crate) fn reset() {
        COUNTER.store(0, Ordering::SeqCst);
    }

    /// Places the counter on the last value the width can hand out.
    ///
    /// Reaching that point by counting would take longer than the tests have, and
    /// arithmetic on the width's own maximum overflows at the widest widths, so these
    /// hooks name the state rather than the number.
    #[cfg(test)]
    pub(crate) fn at_last() {
        COUNTER.store(_ROLLING_IDX_MAX as u64, Ordering::SeqCst);
    }

    /// Places the counter past the last value, which is the exhausted state.
    #[cfg(test)]
    pub(crate) fn exhaust() {
        COUNTER.store((_ROLLING_IDX_MAX as u64).saturating_add(1), Ordering::SeqCst);
    }
}

// A 128-bit index has no atomic to sit in, so it keeps a lock. Nothing else does.
//
// This is the width where the promise of a cheap id is not kept, and saying so is
// better than implying the cost is uniform. It is also the width nobody needs: a
// program that exhausts a `u64` index at one per nanosecond has been running for
// five hundred years.
#[cfg(feature = "u128_index")]
mod counter {
    use super::{Idx, _ROLLING_IDX_MAX};
    use std::sync::Mutex;

    /// The counter, and whether the width has been used up.
    ///
    /// The flag is what a wider counter provides at every other width. Here it is free,
    /// because the lock is already held, so `strict` means the same thing at this width
    /// as at the narrow ones rather than quietly meaning nothing.
    static COUNTER: Mutex<(Idx, bool)> = Mutex::new((0, false));

    #[inline]
    pub(super) fn next() -> Idx {
        let mut state = COUNTER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        #[cfg(feature = "strict")]
        if state.1 {
            panic!(
                "highroller: the rolling index is exhausted. All values of the \
                 configured width have been handed out, and the `strict` feature asks \
                 to stop rather than to reuse one."
            );
        }

        let value = state.0;
        if value == _ROLLING_IDX_MAX {
            // The last value is handed out like any other, and the next call is what
            // wraps or refuses. Every width behaves this way; an earlier version stopped
            // one short here and never handed out its own maximum.
            state.0 = 0;
            state.1 = true;
        } else {
            state.0 = value + 1;
        }
        value
    }

    /// Places the counter, so a test can reach exhaustion without counting to it.
    #[cfg(test)]
    pub(crate) fn reset() {
        *COUNTER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = (0, false);
    }

    /// Places the counter on the last value the width can hand out.
    #[cfg(test)]
    pub(crate) fn at_last() {
        *COUNTER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = (_ROLLING_IDX_MAX, false);
    }

    /// Places the counter past the last value, which is the exhausted state.
    #[cfg(test)]
    pub(crate) fn exhaust() {
        *COUNTER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = (0, true);
    }
}

/// Returns the current rolling index and then increases it by one.
///
/// The index is ephemeral and specific to one run of the program: it starts at zero
/// every time the process does, and it is not stored anywhere.
///
/// Two calls in the same run give two different values until the width is exhausted.
/// What happens then is the `strict` feature's business: with it, this panics; without
/// it, the index wraps to zero and values start repeating.
#[inline]
pub fn rolling_idx() -> Idx {
    counter::next()
}

#[cfg(test)]
pub(crate) use counter::{at_last, exhaust, reset};
