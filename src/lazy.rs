//! A cell that takes its value on first read, for `RUID` under the `const`
//! feature.
//!
//! A `const fn` cannot ask the counter for an index, so a `RUID` built in one
//! starts empty and the first read fills it. Under `std` that is a `OnceLock`,
//! which parks a thread that arrives while another is filling the cell. Under
//! `no_std` it is a state byte and a slot: the fill is one atomic add on the
//! counter, so a thread that arrives during it spins for nanoseconds, and
//! `core` has nothing better to offer than that.

use crate::Idx;

/// An index, or the promise of one.
#[cfg(not(feature = "no_std"))]
pub(crate) struct Lazy(std::sync::OnceLock<Idx>);

#[cfg(not(feature = "no_std"))]
impl Lazy {
    /// Empty, to be filled on first read.
    #[inline]
    pub(crate) const fn new() -> Self {
        Self(std::sync::OnceLock::new())
    }

    /// Already holding `value`, so no read ever takes one.
    #[inline]
    pub(crate) fn with(value: Idx) -> Self {
        let cell = std::sync::OnceLock::new();
        let _ = cell.set(value);
        Self(cell)
    }

    /// The value, taking one with `take` on the first call and keeping it
    /// after.
    #[inline]
    pub(crate) fn get_or_init(&self, take: impl FnOnce() -> Idx) -> Idx {
        *self.0.get_or_init(take)
    }
}

#[cfg(feature = "no_std")]
pub(crate) struct Lazy {
    state: core::sync::atomic::AtomicU8,
    value: core::cell::UnsafeCell<core::mem::MaybeUninit<Idx>>,
}

#[cfg(feature = "no_std")]
const EMPTY: u8 = 0;
#[cfg(feature = "no_std")]
const FILLING: u8 = 1;
#[cfg(feature = "no_std")]
const FULL: u8 = 2;

// SAFETY: the slot is written exactly once, by the thread that moved the state
// from `EMPTY` to `FILLING`, and read only after `FULL` is observed with
// acquire ordering, which orders the read after that write. `Idx` is a plain
// integer, so sharing the value itself is fine.
#[cfg(feature = "no_std")]
unsafe impl Sync for Lazy {}

// A panic while filling puts the state back to `EMPTY` before it unwinds, so a
// cell observed after one is exactly a cell nobody has read yet. That is the
// property the trait names, and the one `OnceLock` has for the same reason.
#[cfg(feature = "no_std")]
impl core::panic::RefUnwindSafe for Lazy {}

#[cfg(feature = "no_std")]
impl Lazy {
    /// Empty, to be filled on first read.
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            state: core::sync::atomic::AtomicU8::new(EMPTY),
            value: core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit()),
        }
    }

    /// Already holding `value`, so no read ever takes one.
    #[inline]
    pub(crate) fn with(value: Idx) -> Self {
        Self {
            state: core::sync::atomic::AtomicU8::new(FULL),
            value: core::cell::UnsafeCell::new(core::mem::MaybeUninit::new(value)),
        }
    }

    /// The value, taking one with `take` on the first call and keeping it
    /// after.
    ///
    /// Two threads reading an empty cell at once agree on the answer: one moves
    /// the state to `FILLING` and takes, the other spins until it sees
    /// `FULL` and reads what the winner wrote.
    #[inline]
    pub(crate) fn get_or_init(&self, take: impl FnOnce() -> Idx) -> Idx {
        use core::sync::atomic::Ordering;

        loop {
            match self.state.load(Ordering::Acquire) {
                // SAFETY: `FULL` is stored with release ordering after the slot is written,
                // and was loaded here with acquire, so the slot holds an initialised value.
                FULL => return unsafe { (*self.value.get()).assume_init() },
                EMPTY
                    if self
                        .state
                        .compare_exchange(EMPTY, FILLING, Ordering::Acquire, Ordering::Relaxed)
                        .is_ok() =>
                {
                    // `take` is the counter, and under the refusing policy the counter
                    // panics when it is exhausted. Left at `FILLING`, every later reader
                    // would spin forever on a fill that is never coming, so the state goes
                    // back to `EMPTY` on the way out unless the fill completed. A `OnceLock`
                    // does the same, which is what makes a failed `get_or_init` retryable.
                    struct Abandon<'a>(&'a core::sync::atomic::AtomicU8);
                    impl Drop for Abandon<'_> {
                        fn drop(&mut self) {
                            self.0.store(EMPTY, Ordering::Release);
                        }
                    }
                    let abandon = Abandon(&self.state);
                    let value = take();
                    core::mem::forget(abandon);
                    // SAFETY: this thread alone moved the state to `FILLING`, so nothing
                    // else writes the slot, and nothing reads it until `FULL` is stored.
                    unsafe { (*self.value.get()).write(value) };
                    self.state.store(FULL, Ordering::Release);
                    return value;
                },
                // Someone else is filling it, or won the exchange just now. The fill is a
                // single atomic add on the counter, so this is a very short wait.
                _ => core::hint::spin_loop(),
            }
        }
    }
}
