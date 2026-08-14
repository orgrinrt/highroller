#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", "README.md"))]

use std::sync::Mutex;

#[macro_export]
macro_rules! __rolling_idx_fn {
    (!c, $t:ty, $max_val:expr, $doc:expr, pre $pre:block, inner { $i:expr }) => {

        #[doc = r#"
Returns the current rolling index and then increases it by 1.

The rolling index is ephemeral and runtime-specific, meaning it is reset every time
the application starts.
"#]
        #[doc =  $doc]
        pub fn rolling_idx() -> $t {
            $pre
            let val: $t = {
                let mut this = $crate::_ROLLING_IDX.lock().unwrap();
                if *this == $max_val {
                    #[cfg(feature = "strict")]
                    {
                        $i
                    }
                    // non-strict wraps back to the start of the range instead of panicking
                    #[cfg(not(feature = "strict"))]
                    {
                        *this = 0;
                    }
                }
                let _val = *this;
                *this += 1;
                _val
            };
            val
        }
    };
    (c, $t:ty, $max_val:expr, $doc:expr, pre $pre:block, inner { $i:expr }) => {
        #[doc = r#"
Returns the current rolling index and then increases it by 1.

The rolling index is ephemeral and runtime-specific, meaning it is reset every time
the application starts.
"#]
        #[doc =  $doc]
        // FIXME: we can't access the static _ROLLING_IDX api in const because mutex api isnt const
        pub /*const*/ fn rolling_idx() -> $t {
            $pre
            let val: $t = {
                let mut this = $crate::_ROLLING_IDX.lock().unwrap();
                if *this == $max_val {
                    #[cfg(feature = "strict")]
                    {
                        $i
                    }
                    // non-strict wraps back to the start of the range instead of panicking
                    #[cfg(not(feature = "strict"))]
                    {
                        *this = 0;
                    }
                }
                let _val = *this;
                *this += 1;
                _val
            };
            val
        }
    }
}

macro_rules! declare_rolling_idx {
    ($t:ty, $max_val:expr) => {
        lazy_static::lazy_static! {
            /// The rolling index. This is increased with each call to `rolling_idx`.
            static ref _ROLLING_IDX: Mutex<$t> = Mutex::new(0);
        }

        /// The largest value the rolling index reaches before it wraps or, under `strict`,
        /// panics. This is the width the crate is configured with.
        pub const _ROLLING_IDX_MAX: $t = $max_val;

        #[cfg(all(feature = "strict", not(feature = "const")))]
        $crate::__rolling_idx_fn!(!c, $t, $max_val,
        "NOTE: The feature flag `strict` *is* enabled, so on overflow, this will panic.",
            pre {
                #[cfg(not(feature = "strict"))]
                panic!(
                    "This should not be able to be called, flags set incorrectly (inform the \
                maintainer)"
                );
            },
            inner {
                panic!("Overflow detected")
            }
        );

        #[cfg(all(not(feature = "strict"), not(feature = "const")))]
        $crate::__rolling_idx_fn!(!c, $t, $max_val,
            "NOTE: The feature flag `strict` is *not* enabled, so on overflow, this will wrap.",
            pre {
                #[cfg(feature = "strict")]
                panic!(
                    "This should not be able to be called, flags set incorrectly (inform the \
                maintainer)"
                );
            },
            inner {
                panic!("Overflow detected")
            }
        );

        #[cfg(all(feature = "strict", feature = "const"))]
        $crate::__rolling_idx_fn!(c, $t, $max_val,
            "NOTE: The feature flag `strict` *is* enabled, so on overflow, this will panic.",
            pre {
                #[cfg(not(feature = "strict"))]
                panic!(
                    "This should not be able to be called, flags set incorrectly (inform the \
                maintainer)"
                );
            },
            inner {
                panic!("Overflow detected")
            }
        );

        #[cfg(all(not(feature = "strict"), feature = "const"))]
        $crate::__rolling_idx_fn!(c, $t, $max_val,
            "NOTE: The feature flag `strict` is *not* enabled, so on overflow, this will wrap.",
            pre {
                #[cfg(feature = "strict")]
                panic!(
                    "This should not be able to be called, flags set incorrectly (inform the \
                maintainer)"
                );
            },
            inner {
                panic!("Overflow detected")
            }
        );

        #[cfg(all(feature = "ruid_type"))]
        use std::clone::Clone;
        #[cfg(all(feature = "ruid_type"))]
        use std::cmp::{Eq, Ord, PartialEq, PartialOrd};
        #[cfg(all(feature = "ruid_type"))]
        use std::convert::{From, Into};
        #[cfg(all(feature = "ruid_type"))]
        use std::fmt;
        #[cfg(all(feature = "ruid_type"))]
        use std::fmt::{Debug, Display};
        #[cfg(all(feature = "ruid_type"))]
        use std::marker::Copy;
        #[cfg(all(feature = "ruid_type", feature = "async"))]
        use core::marker::Send;
        #[cfg(all(feature = "ruid_type", feature = "async"))]
        use core::marker::Sync;
        #[cfg(all(feature = "ruid_type"))]
        use std::ops::Deref;

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        use std::ops::{Add, Div, Mul /*, Neg*/, Rem, Sub};
        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        use std::ops::{AddAssign, DivAssign, MulAssign, RemAssign, SubAssign};

        #[cfg(all(feature = "ruid_type", not(feature = "const")))]
        pub struct RUID {
            __value: $t,
        }

        #[cfg(all(feature = "ruid_type", feature = "const"))]
        pub struct RUID {
            __value: ::std::sync::OnceLock<$t>,
        }

        #[cfg(all(feature = "ruid_type", not(feature = "const")))]
        impl RUID {
            pub fn new() -> Self {
                RUID {
                    __value: $crate::rolling_idx(),
                }
            }
            #[inline]
            pub fn get(&self) -> $t {
                self.__value
            }
            #[inline]
            fn set_raw(&mut self, value: $t) {
                self.__value = value;
            }
            #[inline]
            fn from_raw(value: $t) -> Self {
                RUID { __value: value }
            }
        }
        #[cfg(all(feature = "ruid_type", feature = "const"))]
        impl RUID {
            /// Constructs an unassigned `RUID`. It takes its index the first time it is read,
            /// because a `const fn` cannot roll one.
            pub const fn new() -> Self {
                RUID {
                    __value: ::std::sync::OnceLock::new(),
                }
            }
            /// Returns the index, rolling one on the first call and keeping it thereafter.
            ///
            /// Two threads reading an unassigned `RUID` at once agree on the result: one wins the
            /// initialisation and the other sees the winner's value.
            #[inline]
            pub fn get(&self) -> $t {
                *self.__value.get_or_init(|| $crate::rolling_idx())
            }
            #[inline]
            fn set_raw(&mut self, value: $t) {
                // a OnceLock cannot be reassigned, so the cell itself is replaced
                self.__value = ::std::sync::OnceLock::new();
                let _ = self.__value.set(value);
            }
            #[inline]
            fn from_raw(value: $t) -> Self {
                let cell = ::std::sync::OnceLock::new();
                let _ = cell.set(value);
                RUID { __value: cell }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "async"))]
        unsafe impl Send for RUID {}
        // The `const` shape needs no assertion here: it holds a OnceLock, which is already Sync,
        // and which is what makes the lazy assignment safe to share.
        #[cfg(all(feature = "ruid_type", feature = "async", not(feature = "const")))]
        unsafe impl Sync for RUID {}


        #[cfg(all(feature = "ruid_type", not(feature = "const")))]
        impl Copy for RUID {}

        #[cfg(all(feature = "ruid_type", not(feature = "const")))]
        impl Clone for RUID {
            fn clone(&self) -> Self {
                *self
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "const"))]
        impl Clone for RUID {
            fn clone(&self) -> Self {
                // resolves the index first, so the clone and the original agree
                RUID::from_raw(self.get())
            }
        }

        #[cfg(all(feature = "ruid_type", not(feature = "const")))]
        impl Deref for RUID {
            type Target = $t;
            fn deref(&self) -> &$t {
                &self.__value
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl PartialEq for RUID {
            fn eq(&self, other: &Self) -> bool {
                self.get() == other.get()
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Eq for RUID {}

        #[cfg(all(feature = "ruid_type"))]
        impl PartialOrd for RUID {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Ord for RUID {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }

        #[cfg(all(feature = "ruid_type", not(feature = "strict")))]
        impl PartialEq<$t> for RUID {
            fn eq(&self, other: &$t) -> bool {
                self.get() == *other
            }
        }

        #[cfg(all(feature = "ruid_type", not(feature = "strict")))]
        impl PartialOrd<$t> for RUID {
            fn partial_cmp(&self, other: &$t) -> Option<std::cmp::Ordering> {
                self.get().partial_cmp(other)
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl Add for RUID {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                RUID::from_raw(self.get() + other.get())
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl Sub for RUID {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                RUID::from_raw(self.get() - other.get())
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl Mul for RUID {
            type Output = Self;

            fn mul(self, other: Self) -> Self {
                RUID::from_raw(self.get() * other.get())
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl Div for RUID {
            type Output = Self;

            fn div(self, other: Self) -> Self {
                RUID::from_raw(self.get() / other.get())
            }
        }

        // #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        // impl std::ops::Neg for RUID {
        //     type Output = Self;
        //
        //     fn neg(self) -> Self::Output {
        //         RUID {
        //             __value: -self.get(),
        //         }
        //     }
        // }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::Rem for RUID {
            type Output = Self;

            fn rem(self, other: Self) -> Self {
                RUID::from_raw(self.get() % other.get())
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::AddAssign for RUID {
            fn add_assign(&mut self, other: Self) {
                self.set_raw(self.get() + other.get());
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::SubAssign for RUID {
            fn sub_assign(&mut self, other: Self) {
                self.set_raw(self.get() - other.get());
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::MulAssign for RUID {
            fn mul_assign(&mut self, other: Self) {
                self.set_raw(self.get() * other.get());
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::DivAssign for RUID {
            fn div_assign(&mut self, other: Self) {
                self.set_raw(self.get() / other.get());
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::RemAssign for RUID {
            fn rem_assign(&mut self, other: Self) {
                self.set_raw(self.get() % other.get());
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Add<$t> for RUID {
            type Output = Self;

            fn add(self, other: $t) -> Self {
                RUID::from_raw(self.get() + other)
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Sub<$t> for RUID {
            type Output = Self;

            fn sub(self, other: $t) -> Self {
                RUID::from_raw(self.get() - other)
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Mul<$t> for RUID {
            type Output = Self;

            fn mul(self, other: $t) -> Self {
                RUID::from_raw(self.get() * other)
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Div<$t> for RUID {
            type Output = Self;

            fn div(self, other: $t) -> Self {
                RUID::from_raw(self.get() / other)
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Rem<$t> for RUID {
            type Output = Self;

            fn rem(self, other: $t) -> Self {
                RUID::from_raw(self.get() % other)
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl AddAssign<$t> for RUID {
            fn add_assign(&mut self, other: $t) {
                self.set_raw(self.get() + other);
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl SubAssign<$t> for RUID {
            fn sub_assign(&mut self, other: $t) {
                self.set_raw(self.get() - other);
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl MulAssign<$t> for RUID {
            fn mul_assign(&mut self, other: $t) {
                self.set_raw(self.get() * other);
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl DivAssign<$t> for RUID {
            fn div_assign(&mut self, other: $t) {
                self.set_raw(self.get() / other);
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl RemAssign<$t> for RUID {
            fn rem_assign(&mut self, other: $t) {
                self.set_raw(self.get() % other);
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Display for RUID {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.get())
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Debug for RUID {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("RUID")
                    .field("value", &self.get())
                    .finish()
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl From<$t> for RUID {
            fn from(value: $t) -> Self {
                RUID::from_raw(value)
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Into<$t> for RUID {
            fn into(self) -> $t {
                self.get()
            }
        }
    };
}

#[cfg(feature = "u8_index")]
declare_rolling_idx!(u8, u8::MAX);

#[cfg(feature = "u16_index")]
declare_rolling_idx!(u16, u16::MAX);

#[cfg(feature = "u32_index")]
declare_rolling_idx!(u32, u32::MAX);

#[cfg(feature = "u64_index")]
declare_rolling_idx!(u64, u64::MAX);

#[cfg(feature = "u128_index")]
declare_rolling_idx!(u128, u128::MAX);

#[cfg(feature = "usize_index")]
declare_rolling_idx!(usize, usize::MAX);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static RUN_LOCK: Mutex<bool> = Mutex::new(false);

    // NEED INPUT:
    // Re: the smelly poison clearings - My thinking is this:
    // If an earlier test panicked, it would fail that test, so it does not matter that much
    // if I clear the poison when starting another test. The RUN_LOCK should ensure that the tests
    // run sequentially, and while two of the tests do intentional race conditions and other
    // threading problems, the .join() call there should ensure that *IF* any of the threads
    // panicked, it would fail the test *before* we ever cleared the poison.
    // This correct?

    /// How many ids these tests may ask for and still expect every one to differ.
    ///
    /// The rolling index wraps when its space is exhausted, so this is capped by the width the
    /// crate is configured with: `u8_index` holds 256 values, and asking for more than that
    /// cannot give unique ones.
    fn distinct_id_budget() -> usize {
        const WANTED: usize = 1000;
        let space = crate::_ROLLING_IDX_MAX as u128;
        if space < WANTED as u128 {
            space as usize
        } else {
            WANTED
        }
    }

    pub fn reset_rolling_idx() {
        _ROLLING_IDX.clear_poison();
        let mut index = _ROLLING_IDX.lock().unwrap();
        *index = 0;
    }

    #[test]
    fn test_rolling_index_generation() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();

            let id1 = rolling_idx();
            let id2 = rolling_idx();

            assert_ne!(id1, id2, "Newly generated IDs should not be the same");
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }

    #[test]
    fn test_rolling_index_linearity() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();

            let count: usize = 254;
            let mut counts = Vec::new();
            for i in 0..count {
                counts.push(rolling_idx() as usize);
            }

            assert_eq!(
                counts.len(),
                count,
                "[vec len] {} == {} | Something is wrong with the rolling index stepping!",
                counts.len(),
                count
            );
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }

    #[test]
    fn test_rolling_index_generation_multithreaded() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();

            use std::thread;
            use std::time::Duration;

            let sleep_delays = [0, 0, 0, 10, 20, 30, 40, 50, 60, 70, 80, 90]; // in milliseconds
            let children: Vec<_> = (0..distinct_id_budget())
                .map(|i| {
                    let delay = sleep_delays[i % sleep_delays.len()];
                    thread::Builder::new()
                        .name(format!("test_thread_{}", i))
                        .spawn(move || {
                            thread::sleep(Duration::from_millis(delay as u64));
                            rolling_idx()
                        })
                        .unwrap()
                })
                .collect();

            let mut ids = Vec::new();
            for (i, child) in children.into_iter().enumerate() {
                match child.join() {
                    Ok(id) => {
                        assert!(
                            !ids.contains(&id),
                            "Newly generated ID was the same as a previous one"
                        );
                        ids.push(id);
                    },
                    Err(err) => {
                        println!("{:?}", err);
                        eprintln!("Thread {} panicked", i);
                        panic!();
                    },
                }
            }
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }

    #[test]
    #[cfg(feature = "ruid_type")]
    fn test_ruid_generation() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();

            let id1 = RUID::new();
            let id2 = RUID::new();

            assert_ne!(id1, id2, "Newly generated IDs should not be the same");
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }

    #[test]
    #[cfg(feature = "ruid_type")]
    fn test_ruid_generation_multithreaded() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();

            use std::thread;
            use std::time::Duration;

            let sleep_delays = [0, 0, 0, 10, 20, 30, 40, 50, 60, 70, 80, 90]; // in milliseconds
            let children: Vec<_> = (0..distinct_id_budget())
                .map(|i| {
                    let delay = sleep_delays[i % sleep_delays.len()];
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(delay as u64));
                        RUID::new()
                    })
                })
                .collect();

            let mut ids = Vec::new();
            for (i, child) in children.into_iter().enumerate() {
                match child.join() {
                    Ok(id) => {
                        assert!(
                            !ids.contains(&id),
                            "Newly generated ID was the same as a previous one"
                        );
                        ids.push(id);
                    },
                    Err(err) => {
                        println!("{:?}", err);
                        eprintln!("Thread {} panicked", i);
                        panic!();
                    },
                }
            }
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }

    #[test]
    #[should_panic]
    #[cfg(all(feature = "strict", feature = "u8_index"))]
    fn test_u8_overflow_panic() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();
            for _ in 0..300 {
                let _ = rolling_idx();
            }
            RUN_LOCK.clear_poison();
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }

    #[test]
    #[should_panic]
    #[cfg(all(feature = "strict", feature = "u16_index"))]
    fn test_u16_overflow_panic() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();
            for _ in 0..70_000 {
                let _ = rolling_idx();
            }
            RUN_LOCK.clear_poison();
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }

    // #[test]
    // #[should_panic]
    // #[cfg(all(feature = "strict", feature = "u32_index"))]
    // fn test_u32_overflow_panic() {
    //     reset_rolling_idx();
    //     for _ in 0..5_000_000_000 {
    //         let _ = rolling_idx();
    //     }
    // }

    // #[test]
    // #[should_panic]
    // #[cfg(all(feature = "strict", feature = "u64_index"))]
    // fn test_u64_overflow_panic() {
    //     reset_rolling_idx();
    //     for _ in 0..18_000_000_000_000_000_000 {
    //         let _ = rolling_idx();
    //     }
    // }

    #[test]
    #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
    fn test_arithmetic_operations_ruids() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();

            // fixed values, so the arithmetic is checked rather than the rolling index
            let id1 = RUID::from_raw(6);
            let id2 = RUID::from_raw(3);

            let sum = id1.clone() + id2.clone();
            assert_eq!(sum.get(), 9, "Sum does not match");

            let diff = id1.clone() - id2.clone();
            assert_eq!(diff.get(), 3, "Difference does not match");

            let product = id1.clone() * id2.clone();
            assert_eq!(product.get(), 18, "Product does not match");

            let quotient = id1.clone() / id2.clone();
            assert_eq!(quotient.get(), 2, "Quotient does not match");

            let remainder = id1.clone() % id2.clone();
            assert_eq!(remainder.get(), 0, "Remainder does not match");
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }

    #[test]
    #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
    fn test_arithmetic_operations_mixed() {
        let _lock_res = RUN_LOCK.lock();
        let test_fn = || {
            reset_rolling_idx();

            // fixed value, so the arithmetic is checked rather than the rolling index
            let id1 = RUID::from_raw(6);
            let i = 2;

            let sum = id1.clone() + i;
            assert_eq!(sum.get(), 8, "Sum does not match");

            let diff = id1.clone() - i;
            assert_eq!(diff.get(), 4, "Difference does not match");

            let product = id1.clone() * i;
            assert_eq!(product.get(), 12, "Product does not match");

            let quotient = id1.clone() / i;
            assert_eq!(quotient.get(), 3, "Quotient does not match");

            let remainder = id1.clone() % i;
            assert_eq!(remainder.get(), 0, "Remainder does not match");
        };

        match _lock_res {
            Ok(lock_guard) => {
                test_fn();
            },
            Err(poisoned_lock) => {
                RUN_LOCK.clear_poison();
                let lock_guard = poisoned_lock.into_inner();
                test_fn();
            },
        }
    }
}

#[cfg(all(test, feature = "ruid_type", feature = "const"))]
mod const_construction {
    use super::*;

    /// The point of the `const` feature: a `RUID` can be built in a const context. It has no
    /// index at that point, because a `const fn` cannot roll one.
    static PLACEHOLDER: RUID = RUID::new();

    #[test]
    fn const_constructed_ruid_takes_an_index_on_first_read() {
        let first = PLACEHOLDER.get();
        let second = PLACEHOLDER.get();
        assert_eq!(first, second, "the index is rolled once and kept");
    }

    #[test]
    fn separately_constructed_ruids_differ() {
        let a = RUID::new();
        let b = RUID::new();
        assert_ne!(a, b, "each RUID takes its own index");
    }
}
