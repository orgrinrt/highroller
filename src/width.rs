//! The integer widths a counter can be, and what each one keeps its count in.
//!
//! Every width up to 64 bits counts in an `AtomicU64`, which is what makes
//! handing out an index one atomic add: the counter passes a narrower width's
//! maximum long before it could wrap itself, so exhaustion is a comparison and
//! wrapping is the narrowing cast. A 128-bit width has no wider atomic to count
//! in, so it keeps a lock instead, and says so.

use core::mem::size_of;
use core::sync::atomic::{AtomicU64, Ordering};

mod sealed {
    pub trait Sealed {}
}

/// An integer a [`Counter`](crate::Counter) can hand out.
///
/// Implemented for `u8`, `u16`, `u32`, `u64`, `usize` and `u128`, and sealed:
/// the store a width counts in is this crate's business, and a width added from
/// outside would have to pick one.
pub trait Width:
    Copy + Eq + Ord + core::fmt::Debug + core::fmt::Display + sealed::Sealed + 'static
{
    /// What a counter of this width keeps its count in.
    #[doc(hidden)]
    type Store: Sync + 'static;

    /// A store holding nothing yet. An associated constant rather than a
    /// function because a trait cannot have a `const fn` on stable, and
    /// this is what lets `Counter::new` be one.
    #[doc(hidden)]
    const EMPTY: Self::Store;

    /// Zero, as this width.
    const ZERO: Self;

    /// The largest value of this width, which is the last one a counter hands
    /// out.
    const MAX: Self;

    /// Whether a counter of this width can tell that it has been exhausted.
    ///
    /// True where the store is wider than the width, because passing the
    /// width's maximum is then a comparison the store cannot have wrapped
    /// past. At `u64`, and at `usize` on a 64-bit target, the store is no
    /// wider than the width and exhaustion is not observable. It is also
    /// not reachable: a thousand million ids a second exhausts a 64-bit index
    /// in about five hundred years.
    ///
    /// Asked of the width rather than of a feature name, because on wasm32 and
    /// i686 a `usize` is 32 bits, the store really is wider, and a check
    /// keyed on the feature would have been compiled out exactly where it
    /// was needed.
    const EXHAUSTION_IS_OBSERVABLE: bool;

    /// Hands out the next value.
    ///
    /// With `refuse` set, an exhausted counter panics rather than repeating a
    /// value, where
    /// [`EXHAUSTION_IS_OBSERVABLE`](Width::EXHAUSTION_IS_OBSERVABLE) allows it
    /// to know.
    #[doc(hidden)]
    fn take(store: &Self::Store, refuse: bool) -> Self;

    /// Puts the counter where the next value handed out is `at`.
    #[doc(hidden)]
    fn place(store: &Self::Store, at: Self);
}

/// The message for an exhausted counter under the refusing policy.
#[cold]
#[inline(never)]
fn refuse(bits: u32) -> ! {
    panic!(
        "highroller: the rolling index is exhausted. All 2^{bits} values of the configured \
         width have been handed out, and the counter is set to stop rather than to reuse \
         one. Either widen the index or let it wrap."
    );
}

macro_rules! counts_in_a_u64 {
    ($($t:ty),+ $(,)?) => {$(
        impl sealed::Sealed for $t {}

        impl Width for $t {
            type Store = AtomicU64;

            // The lint is about a const being read as though it were a shared value. This
            // one is only ever written into a fresh `Counter`, which is what an associated
            // const in a `const fn` constructor is for.
            #[allow(clippy::declare_interior_mutable_const)]
            const EMPTY: AtomicU64 = AtomicU64::new(0);

            const ZERO: $t = 0;
            const MAX: $t = <$t>::MAX;
            const EXHAUSTION_IS_OBSERVABLE: bool = size_of::<$t>() < size_of::<u64>();

            #[inline]
            fn take(store: &AtomicU64, refuse_when_exhausted: bool) -> $t {
                let prev = store.fetch_add(1, Ordering::Relaxed);

                // Both conditions are constants after monomorphisation, so the whole branch
                // folds away where it does not apply and the wrapping path is the add alone.
                if refuse_when_exhausted && Self::EXHAUSTION_IS_OBSERVABLE {
                    // The wide store has not wrapped and will not, so passing the narrow
                    // maximum is exhaustion and every later caller sees it too. That is
                    // what a same-width `fetch_add` cannot offer: there, the wrap to zero is
                    // indistinguishable from a fresh counter, and a thread arriving in that
                    // window would be handed an index already in use.
                    //
                    // Compared as `u128` so that at the widths where the branch is dead the
                    // comparison is still a meaningful one rather than one clippy refuses.
                    if u128::from(prev) > <$t>::MAX as u128 {
                        refuse(<$t>::BITS);
                    }
                }

                // Wrapping is the truncation, and nothing more. Every width's range is a
                // power of two, so reducing the wide count modulo that range is what a
                // narrowing cast already does.
                prev as $t
            }

            #[inline]
            fn place(store: &AtomicU64, at: $t) {
                store.store(at as u64, Ordering::SeqCst);
            }
        }
    )+};
}

counts_in_a_u64!(u8, u16, u32, u64, usize);

impl sealed::Sealed for u128 {}

impl Width for u128 {
    type Store = lock::Locked;

    #[allow(clippy::declare_interior_mutable_const)]
    const EMPTY: lock::Locked = lock::Locked::new();
    /// Always, at this width: the lock is already held, so a flag saying the
    /// width has been used up is free, and the refusing policy means the
    /// same thing here as it does at the narrow widths rather than quietly
    /// meaning nothing.
    const EXHAUSTION_IS_OBSERVABLE: bool = true;
    const MAX: u128 = u128::MAX;
    const ZERO: u128 = 0;

    #[inline]
    fn take(store: &lock::Locked, refuse_when_exhausted: bool) -> u128 {
        store.with(|state| {
            if refuse_when_exhausted && state.exhausted {
                refuse(u128::BITS);
            }
            let value = state.next;
            if value == u128::MAX {
                // The last value is handed out like any other, and the next call is what
                // wraps or refuses. An earlier version stopped one short here and never
                // handed out its own maximum.
                state.next = 0;
                state.exhausted = true;
            } else {
                state.next = value + 1;
            }
            value
        })
    }

    #[inline]
    fn place(store: &lock::Locked, at: u128) {
        store.with(|state| {
            state.next = at;
            state.exhausted = false;
        });
    }
}

/// The lock a 128-bit counter keeps, since there is no atomic to put one in.
///
/// Under `std` it is a `Mutex`, which parks a waiting thread. Under `no_std` it
/// is a spin lock, which is what `core` can offer: the critical section is a
/// comparison and an add, so a spinner waits nanoseconds, and a thread
/// preempted inside it costs the others a time slice, which is the price of a
/// 128-bit index without an operating system.
mod lock {
    /// The count, and whether the width has been used up.
    pub struct State {
        pub next:      u128,
        pub exhausted: bool,
    }

    const FRESH: State = State {
        next:      0,
        exhausted: false,
    };

    #[cfg(not(feature = "no_std"))]
    pub struct Locked(std::sync::Mutex<State>);

    #[cfg(not(feature = "no_std"))]
    impl Locked {
        pub const fn new() -> Self {
            Self(std::sync::Mutex::new(FRESH))
        }

        #[inline]
        pub fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
            // A poisoned lock holds a consistent state: the only panic inside the critical
            // section is the refusal, which fires before anything is written.
            let mut state = self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            f(&mut state)
        }
    }

    #[cfg(feature = "no_std")]
    pub struct Locked {
        busy:  core::sync::atomic::AtomicBool,
        state: core::cell::UnsafeCell<State>,
    }

    // SAFETY: the only access to `state` is through `with`, which holds `busy` for
    // the duration, so no two threads reach it at once.
    #[cfg(feature = "no_std")]
    unsafe impl Sync for Locked {}

    // The one panic that can happen while the lock is held is the refusal, which
    // fires before anything is written, and the lock is released on unwind. So
    // a lock observed after a panic holds the state it held before, which is
    // what a `Mutex` promises with its poisoning and what this promises without
    // it.
    #[cfg(feature = "no_std")]
    impl core::panic::RefUnwindSafe for Locked {}

    #[cfg(feature = "no_std")]
    impl Locked {
        pub const fn new() -> Self {
            Self {
                busy:  core::sync::atomic::AtomicBool::new(false),
                state: core::cell::UnsafeCell::new(FRESH),
            }
        }

        #[inline]
        pub fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
            use core::sync::atomic::Ordering;

            while self
                .busy
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                core::hint::spin_loop();
            }

            // The refusal panics while the lock is held. Releasing on the way out, panic
            // or not, is what keeps the next caller from spinning forever on a lock whose
            // holder is gone.
            struct Release<'a>(&'a core::sync::atomic::AtomicBool);
            impl Drop for Release<'_> {
                fn drop(&mut self) {
                    self.0.store(false, Ordering::Release);
                }
            }
            let _release = Release(&self.busy);

            // SAFETY: `busy` was taken above and is held until `_release` drops, so this is
            // the only reference to the state for as long as it lives.
            f(unsafe { &mut *self.state.get() })
        }
    }
}
