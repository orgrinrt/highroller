#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", "README.md"))]

use std::sync::Mutex;

use lazy_static::lazy_static;
#[cfg(not(feature = "strict"))]
panic!(
    "This should not be able to be called, flags set incorrectly (inform the \
            maintainer)"
);
#[macro_export]
macro_rules! __rolling_idx_fn {
    (!c, $t:ty, $max_val:expr, pre $pre:block, inner { $i:expr }) => {
        pub fn rolling_idx() -> $t {
            $pre
            let val: $t = {
                let mut this = $crate::_ROLLING_IDX.lock().unwrap();
                if *this == $max_val {
                    $i
                }
                let _val = *this;
                *this += 1;
                _val
            };
            val
        }
    };
    (c, $t:ty, $max_val:expr, pre $pre:block, inner { $i:expr }) => {
        pub const fn rolling_idx() -> $t {
            $pre
            let val: $t = {
                let mut this = $crate::_ROLLING_IDX.lock().unwrap();
                if *this == $max_val {
                    $i
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
        lazy_static! {
            /// The rolling index. This is increased with each call to `rolling_idx`.
            static ref _ROLLING_IDX: Mutex<$t> = Mutex::new(0);
        }

        /// Returns the current rolling index and then increases it by 1.
        ///
        /// The rolling index is ephemeral and runtime-specific,
        /// meaning it is reset every time the application starts.
        ///
        #[cfg(all(feature = "strict", not(feature = "const")))]
        /// NOTE: The feature flag `strict` *is* enabled, so on overflow, this will panic.
        $crate::__rolling_idx_fn!(!c, $t, $max_val,
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
        /// NOTE: The feature flag `strict` is *not* enabled, so on overflow, this will wrap.
        $crate::__rolling_idx_fn!(!c, $t, $max_val,
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

        #[cfg(all(feature = "strict", feature = "const"))]
        /// NOTE: The feature flag `strict` *is* enabled, so on overflow, this will panic.
        $crate::__rolling_idx_fn!(c, $t, $max_val,
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
        /// NOTE: The feature flag `strict` is *not* enabled, so on overflow, this will wrap.
        $crate::__rolling_idx_fn!(c, $t, $max_val,
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

        #[cfg(all(feature = "ruid_type"))]
        pub struct RUID {
            __value: $t,
        }

        #[cfg(all(feature = "ruid_type", not(feature = "const")))]
        impl RUID {
            pub fn new() -> Self {
                RUID {
                    __value: $crate::rolling_idx(),
                }
            }
        }
        #[cfg(all(feature = "ruid_type", feature = "const"))]
        impl RUID {
            pub const fn new() -> Self {
                RUID {
                    __value: $crate::rolling_idx(),
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "async"))]
        unsafe impl Send for RUID {}
        #[cfg(all(feature = "ruid_type", feature = "async"))]
        unsafe impl Sync for RUID {}


        #[cfg(all(feature = "ruid_type"))]
        impl Copy for RUID {}

        #[cfg(all(feature = "ruid_type"))]
        impl Clone for RUID {
            fn clone(&self) -> Self {
                *self
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Deref for RUID {
            fn deref(&self) -> &Self::__value {
                &self.__value
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl PartialEq for RUID {
            fn eq(&self, other: &Self) -> bool {
                self.__value == other.__value
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Eq for RUID {}

        #[cfg(all(feature = "ruid_type"))]
        impl PartialOrd for RUID {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.__value.partial_cmp(&other.__value)
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Ord for RUID {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.__value.cmp(&other.__value)
            }
        }

        #[cfg(all(feature = "ruid_type", not(feature = "strict")))]
        impl PartialEq<$t> for RUID {
            fn eq(&self, other: &$t) -> bool {
                self.__value == *other
            }
        }

        #[cfg(all(feature = "ruid_type", not(feature = "strict")))]
        impl PartialOrd<$t> for RUID {
            fn partial_cmp(&self, other: &$t) -> Option<std::cmp::Ordering> {
                self.__value.partial_cmp(other)
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl Add for RUID {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                RUID {
                    __value: self.__value + other.__value,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl Sub for RUID {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                RUID {
                    __value: self.__value - other.__value,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl Mul for RUID {
            type Output = Self;

            fn mul(self, other: Self) -> Self {
                RUID {
                    __value: self.__value * other.__value,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl Div for RUID {
            type Output = Self;

            fn div(self, other: Self) -> Self {
                RUID {
                    __value: self.__value / other.__value,
                }
            }
        }

        // #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        // impl std::ops::Neg for RUID {
        //     type Output = Self;
        //
        //     fn neg(self) -> Self::Output {
        //         RUID {
        //             __value: -self.__value,
        //         }
        //     }
        // }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::Rem for RUID {
            type Output = Self;

            fn rem(self, other: Self) -> Self {
                RUID {
                    __value: self.__value % other.__value,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::AddAssign for RUID {
            fn add_assign(&mut self, other: Self) {
                self.__value += other.__value;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::SubAssign for RUID {
            fn sub_assign(&mut self, other: Self) {
                self.__value -= other.__value;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::MulAssign for RUID {
            fn mul_assign(&mut self, other: Self) {
                self.__value *= other.__value;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::DivAssign for RUID {
            fn div_assign(&mut self, other: Self) {
                self.__value /= other.__value;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics"))]
        impl std::ops::RemAssign for RUID {
            fn rem_assign(&mut self, other: Self) {
                self.__value %= other.__value;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Add<$t> for RUID {
            type Output = Self;

            fn add(self, other: $t) -> Self {
                RUID {
                    __value: self.__value + other,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Sub<$t> for RUID {
            type Output = Self;

            fn sub(self, other: $t) -> Self {
                RUID {
                    __value: self.__value - other,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Mul<$t> for RUID {
            type Output = Self;

            fn mul(self, other: $t) -> Self {
                RUID {
                    __value: self.__value * other,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Div<$t> for RUID {
            type Output = Self;

            fn div(self, other: $t) -> Self {
                RUID {
                    __value: self.__value / other,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl Rem<$t> for RUID {
            type Output = Self;

            fn rem(self, other: $t) -> Self {
                RUID {
                    __value: self.__value % other,
                }
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl AddAssign<$t> for RUID {
            fn add_assign(&mut self, other: $t) {
                self.__value += other;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl SubAssign<$t> for RUID {
            fn sub_assign(&mut self, other: $t) {
                self.__value -= other;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl MulAssign<$t> for RUID {
            fn mul_assign(&mut self, other: $t) {
                self.__value *= other;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl DivAssign<$t> for RUID {
            fn div_assign(&mut self, other: $t) {
                self.__value /= other;
            }
        }

        #[cfg(all(feature = "ruid_type", feature = "allow_arithmetics", not(feature = "strict")))]
        impl RemAssign<$t> for RUID {
            fn rem_assign(&mut self, other: $t) {
                self.__value %= other;
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Display for RUID {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.__value)
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Debug for RUID {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("RUID")
                    .field("value", &self.__value)
                    .finish()
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl From<$t> for RUID {
            fn from(value: $t) -> Self {
                RUID {
                    __value: value,
                }
            }
        }

        #[cfg(all(feature = "ruid_type"))]
        impl Into<$t> for RUID {
            fn into(self) -> $t {
                self.__value
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

    static RUN_LOCK: Mutex<bool> = Mutex::new(false);

    // NEED INPUT:
    // Re: the smelly poison clearings - My thinking is this:
    // If an earlier test panicked, it would fail that test, so it does not matter that much
    // if I clear the poison when starting another test. The RUN_LOCK should ensure that the tests
    // run sequentially, and while two of the tests do intentional race conditions and other
    // threading problems, the .join() call there should ensure that *IF* any of the threads
    // panicked, it would fail the test *before* we ever cleared the poison.
    // This correct?

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
            let children: Vec<_> = (0..1000)
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
            let children: Vec<_> = (0..1000)
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

            let mut id1 = RUID::new();
            let mut id2 = RUID::new();

            id1.__value = 6; // Use fallback values for testing
            id2.__value = 3;

            let sum = id1 + id2;
            assert_eq!(sum.__value, 9, "Sum does not match");

            let diff = id1 - id2;
            assert_eq!(diff.__value, 3, "Difference does not match");

            let product = id1 * id2;
            assert_eq!(product.__value, 18, "Product does not match");

            let quotient = id1 / id2;
            assert_eq!(quotient.__value, 2, "Quotient does not match");

            let remainder = id1 % id2;
            assert_eq!(remainder.__value, 0, "Remainder does not match");
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

            let mut id1 = RUID::new();
            let i = 2;

            id1.__value = 6; // Use fallback value for testing

            let sum = id1 + i;
            assert_eq!(sum.__value, 8, "Sum does not match");

            let diff = id1 - i;
            assert_eq!(diff.__value, 4, "Difference does not match");

            let product = id1 * i;
            assert_eq!(product.__value, 12, "Product does not match");

            let quotient = id1 / i;
            assert_eq!(quotient.__value, 3, "Quotient does not match");

            let remainder = id1 % i;
            assert_eq!(remainder.__value, 0, "Remainder does not match");
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
