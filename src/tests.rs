//! The rolling index is one global counter, so these tests share state and run under
//! one lock rather than in parallel.
//!
//! The lock is taken by every test, including the ones that panic on purpose, and the
//! poison is cleared rather than propagated. A panicking test has already failed; there
//! is nothing for the next test to learn from the poison, and leaving it set would fail
//! every later test for a reason that has nothing to do with them.

#![allow(clippy::unnecessary_cast)] // the width casts are no-ops at exactly one width

use super::*;
use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());

/// Takes the shared lock and resets the counter, so each test starts from zero.
fn serial() -> MutexGuard<'static, ()> {
    let guard = SERIAL.lock().unwrap_or_else(|poisoned| {
        SERIAL.clear_poison();
        poisoned.into_inner()
    });
    index::reset();
    guard
}

/// How many ids a test may ask for and still expect every one to differ.
///
/// The index wraps when its space is exhausted, so this is capped by the configured
/// width: `u8_index` holds 256 values, and asking for more cannot give unique ones.
#[allow(clippy::unnecessary_cast)] // a no-op at u128_index and load-bearing elsewhere
fn distinct_budget() -> usize {
    const WANTED: usize = 1000;
    let space = _ROLLING_IDX_MAX as u128;
    if space < WANTED as u128 {
        space as usize
    } else {
        WANTED
    }
}

#[test]
fn two_indices_differ() {
    let _g = serial();
    assert_ne!(rolling_idx(), rolling_idx());
}

#[test]
fn indices_start_at_zero_and_step_by_one() {
    let _g = serial();
    let taken: Vec<Idx> = (0..8).map(|_| rolling_idx()).collect();
    let expected: Vec<Idx> = (0..8).collect();
    assert_eq!(
        taken, expected,
        "the index starts at zero and increases by one, and its value is the count of \
         calls before it"
    );
}

#[test]
fn a_run_of_indices_is_free_of_repeats() {
    let _g = serial();
    let budget = distinct_budget();
    let taken: Vec<Idx> = (0..budget).map(|_| rolling_idx()).collect();
    let mut sorted = taken.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), budget, "every index in a run differs from every other");
}

#[test]
fn threads_never_receive_the_same_index() {
    let _g = serial();
    let budget = distinct_budget().min(256);
    // No sleeps: staggering the threads makes a collision less likely to occur, which
    // is the opposite of what a test for collisions wants. They all start at once and
    // contend as hard as the machine allows.
    let taken: Vec<Idx> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..budget).map(|_| s.spawn(rolling_idx)).collect();
        handles.into_iter().map(|h| h.join().expect("no thread panics")).collect()
    });
    let mut sorted = taken.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        budget,
        "{budget} threads asked for an index at once and got {} distinct ones",
        sorted.len()
    );
}

/// Whether the counter is wider than the index, which is what makes exhaustion visible.
///
/// A runtime read of the same thing the crate decides at compile time. The tests below
/// used to be gated on the feature name instead, which on wasm32 and i686 skipped exactly
/// the configuration where the predicate had been wrong.
const EXHAUSTION_IS_OBSERVABLE: bool =
    core::mem::size_of::<Idx>() < core::mem::size_of::<u64>();

#[test]
fn the_exhaustion_check_follows_the_width_and_not_the_feature_name() {
    // `usize_index` is 64 bits on this machine and 32 on wasm32 and i686, so a predicate
    // naming the feature and one measuring the width disagree there. The width is the
    // question, and this is what says so.
    assert_eq!(
        EXHAUSTION_IS_OBSERVABLE,
        core::mem::size_of::<Idx>() < 8,
        "exhaustion is observable exactly when the counter is wider than the index"
    );
}

/// The property the wide counter is for.
///
/// A counter as narrow as the index wraps to zero when it is exhausted, and a thread
/// arriving in that window cannot tell a wrapped counter from a fresh one, so it is
/// handed an index already in use. A wider counter passes the narrow maximum without
/// wrapping, so exhaustion is visible to every later caller.
#[test]
#[cfg(feature = "strict")]
fn the_last_value_of_the_width_is_handed_out() {
    let _g = serial();
    // Start at the end rather than counting all the way there, which for a 64-bit
    // index would not finish.
    index::at_last();
    assert_eq!(
        rolling_idx(),
        _ROLLING_IDX_MAX,
        "the whole range is usable. The previous implementation stopped one short and \
         never handed out its own maximum, in either mode."
    );
}

/// Runs `f` and reports whether it panicked, without printing the panic.
///
/// `#[should_panic]` cannot say "panics, where the width allows one to be detected", and
/// which widths those are is not known until `size_of` is evaluated.
#[cfg(feature = "strict")]
fn panicked(f: impl FnOnce() + std::panic::UnwindSafe) -> bool {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(f);
    std::panic::set_hook(previous);
    outcome.is_err()
}

#[test]
#[cfg(feature = "strict")]
fn strict_refuses_once_the_width_is_exhausted() {
    if !EXHAUSTION_IS_OBSERVABLE {
        return;
    }
    let _g = serial();
    index::exhaust();
    assert!(
        panicked(|| {
            let _ = rolling_idx();
        }),
        "an exhausted index refuses rather than reusing a value"
    );
}

#[test]
#[cfg(feature = "strict")]
fn strict_keeps_refusing_after_the_first_refusal() {
    if !EXHAUSTION_IS_OBSERVABLE {
        return;
    }
    let _g = serial();
    index::exhaust();
    // Well past the end: a same-width counter would have wrapped back into the valid
    // range by now and handed out a duplicate instead of refusing.
    assert!(
        panicked(|| {
            let _ = rolling_idx();
        }),
        "and it keeps refusing rather than refusing once"
    );
}

#[test]
#[cfg(not(feature = "strict"))]
fn without_strict_the_index_wraps_and_repeats() {
    if !EXHAUSTION_IS_OBSERVABLE {
        return;
    }
    let _g = serial();
    let first = rolling_idx();
    index::exhaust();
    assert_eq!(
        rolling_idx(),
        first,
        "past its maximum the index returns to the start rather than refusing"
    );
}

#[test]
#[cfg(not(feature = "strict"))]
fn the_last_value_of_the_width_is_handed_out() {
    let _g = serial();
    index::at_last();
    assert_eq!(
        rolling_idx(),
        _ROLLING_IDX_MAX,
        "the whole range is usable. The previous implementation wrapped one early and \
         never handed out its own maximum."
    );
}

// There was a test here asserting `_ROLLING_IDX_MAX == Idx::MAX`, against a definition
// reading `pub const _ROLLING_IDX_MAX: Idx = Idx::MAX;`. A constant compared to the
// literal its own definition sets cannot fail, and the shape has a name in this
// workspace's test gate along with the instruction to delete rather than repair it. What
// ties the maximum to something the code does is the exhaustion tests above, which place
// the counter at it and act on what comes back.

#[cfg(feature = "ruid_type")]
mod ruid {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use crate::{Derived, Provenance, Rolled};

    fn hash_of<P: Provenance>(id: &RUID<P>) -> u64 {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        h.finish()
    }

    /// A derived id built from a known value, which is the only way to get a predictable
    /// one to assert against.
    fn derived(value: u8) -> RUID<Derived> {
        RUID::<Derived>::from(value as Idx)
    }

    #[test]
    fn two_rolled_ids_differ() {
        let _g = serial();
        assert_ne!(RUID::new(), RUID::new());
    }

    #[test]
    fn provenance_is_readable_from_the_value() {
        let _g = serial();
        assert!(RUID::new().is_rolled(), "new() asks the counter");
        assert!(!derived(5).is_rolled(), "from() does not");
    }

    #[test]
    fn a_rolled_id_can_be_demoted_and_keeps_its_value() {
        let _g = serial();
        let rolled = RUID::new();
        let value = rolled.get();
        let derived: RUID<Derived> = rolled.into_derived();
        assert_eq!(derived.get(), value, "demoting forgets the guarantee, not the value");
        assert!(!derived.is_rolled());
    }

    #[test]
    fn threads_never_receive_the_same_ruid() {
        let _g = serial();
        let budget = distinct_budget().min(256);
        let taken: Vec<RUID<Rolled>> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..budget).map(|_| s.spawn(RUID::new)).collect();
            handles.into_iter().map(|h| h.join().expect("no thread panics")).collect()
        });
        let mut values: Vec<Idx> = taken.iter().map(RUID::get).collect();
        values.sort_unstable();
        values.dedup();
        assert_eq!(values.len(), budget);
    }

    #[test]
    fn reading_a_ruid_twice_gives_the_same_answer() {
        let _g = serial();
        let id = RUID::new();
        assert_eq!(id.get(), id.get(), "a RUID holds an index, it does not take one per read");
    }

    #[test]
    fn a_derived_id_takes_no_index() {
        let _g = serial();
        let before = derived(7);
        assert_eq!(before.get(), 7);
        assert_eq!(
            rolling_idx(),
            0,
            "wrapping a value that was already in hand does not advance the counter"
        );
    }

    #[test]
    fn ordering_follows_the_underlying_index() {
        let _g = serial();
        let first = RUID::new();
        let second = RUID::new();
        assert!(first < second, "ids are ordered the way the indices they hold are");
        assert_eq!(first.cmp(&second), first.get().cmp(&second.get()));
    }

    #[test]
    fn equality_and_ordering_reach_across_provenance() {
        let _g = serial();
        let rolled = RUID::new();
        let same_value = derived(rolled.get() as u8);
        assert_eq!(rolled, same_value, "two ids naming the same thing are the same id");
        assert_eq!(
            hash_of(&rolled),
            hash_of(&same_value),
            "and they hash alike, or a map holds both and answers for neither"
        );
        assert!(rolled <= same_value);
    }

    #[test]
    fn equal_ids_hash_equally() {
        let _g = serial();
        assert_eq!(derived(42), derived(42));
        assert_eq!(hash_of(&derived(42)), hash_of(&derived(42)));
    }

    #[test]
    fn formatting_delegates_to_the_integer() {
        let _g = serial();
        let id = derived(200);
        assert_eq!(format!("{id}"), "200");
        assert_eq!(format!("{id:?}"), "RUID(200)");
        assert_eq!(format!("{id:x}"), "c8");
        assert_eq!(format!("{id:X}"), "C8");
        assert_eq!(format!("{id:o}"), "310");
        assert_eq!(format!("{id:b}"), "11001000");
        // The flags reach the integer rather than being dropped, which is what
        // delegating buys over a hand-written `write!`.
        assert_eq!(format!("{id:#06x}"), "0x00c8");
        assert_eq!(format!("{id:>6}"), "   200");
    }

    #[test]
    fn parsing_gives_a_derived_id() {
        let _g = serial();
        let id: RUID<Derived> = "123".parse().expect("123 is an index");
        assert_eq!(id.get(), 123);
        assert!(!id.is_rolled());
        assert!("not a number".parse::<RUID<Derived>>().is_err());
    }

    #[test]
    fn converting_back_gives_the_index() {
        let _g = serial();
        let raw: Idx = derived(11).into();
        assert_eq!(raw, 11);
    }

    #[test]
    fn default_takes_an_index_rather_than_giving_zero() {
        let _g = serial();
        let a = RUID::default();
        let b = RUID::default();
        assert_ne!(a, b, "a defaulted RUID is a real id, not a zero placeholder");
        assert!(a.is_rolled());
    }

    #[test]
    #[cfg(not(feature = "strict"))]
    fn a_ruid_compares_against_a_bare_index() {
        let _g = serial();
        let id = derived(5);
        assert!(id == 5);
        assert!(id < 6);
    }

    // Both operator families are exercised on purpose. Whether `RUID` is `Copy` depends
    // on the `const` feature, so the by-value form is the natural one for a caller in one
    // configuration and impossible in the other, and only running both proves they agree.
    // Clippy sees one configuration at a time and reads the redundancy as waste.
    #[allow(clippy::clone_on_copy, clippy::op_ref)]
    #[test]
    #[cfg(feature = "allow_arithmetics")]
    fn arithmetic_operates_on_the_underlying_index() {
        let _g = serial();
        let a = derived(10);
        let b = derived(3);

        assert_eq!((&a + &b).get(), 13);
        assert_eq!((&a - &b).get(), 7);
        assert_eq!((&a * &b).get(), 30);
        assert_eq!((&a / &b).get(), 3);
        assert_eq!((&a % &b).get(), 1);

        // The same five against a bare index, a separate set of impls and so a separate
        // chance to have transposed an operator.
        assert_eq!((&a + 3).get(), 13);
        assert_eq!((&a - 3).get(), 7);
        assert_eq!((&a * 3).get(), 30);
        assert_eq!((&a / 3).get(), 3);
        assert_eq!((&a % 3).get(), 1);

        // And by value, which is what a caller writes when the type is Copy.
        assert_eq!((a.clone() + b.clone()).get(), 13);
        assert_eq!((a.clone() - b.clone()).get(), 7);
        assert_eq!((a.clone() * b.clone()).get(), 30);
        assert_eq!((a.clone() / b.clone()).get(), 3);
        assert_eq!((a.clone() % b.clone()).get(), 1);
    }

    #[test]
    #[cfg(feature = "allow_arithmetics")]
    fn arithmetic_on_a_rolled_id_gives_a_derived_one() {
        let _g = serial();
        let rolled = RUID::new();
        let result: RUID<Derived> = &rolled + 1;
        assert!(
            !result.is_rolled(),
            "the counter never handed out this value, so it cannot claim it did"
        );
    }

    #[allow(clippy::clone_on_copy)]
    #[test]
    #[cfg(feature = "allow_arithmetics")]
    fn assigning_arithmetic_matches_its_operator() {
        let _g = serial();
        for (name, apply, expected) in [
            ("add", (|x: &mut RUID<Derived>| *x += 3 as Idx) as fn(&mut RUID<Derived>), 13 as Idx),
            ("sub", |x: &mut RUID<Derived>| *x -= 3 as Idx, 7),
            ("mul", |x: &mut RUID<Derived>| *x *= 3 as Idx, 30),
            ("div", |x: &mut RUID<Derived>| *x /= 3 as Idx, 3),
            ("rem", |x: &mut RUID<Derived>| *x %= 3 as Idx, 1),
        ] {
            let mut id = derived(10);
            apply(&mut id);
            assert_eq!(id.get(), expected, "{name}_assign disagrees with {name}");
        }
    }

    #[test]
    #[cfg(not(feature = "const"))]
    fn a_ruid_derefs_and_borrows_as_its_index() {
        let _g = serial();
        use std::borrow::Borrow;
        let id = derived(4);
        assert_eq!(*id, 4 as Idx);
        let borrowed: &Idx = id.borrow();
        assert_eq!(*borrowed, 4);
        assert_eq!(*id.as_ref(), 4 as Idx);
    }

    /// The point of the `const` feature: a `RUID` can be built where a constant is
    /// required. It holds no index at that point, because a `const fn` cannot take one.
    #[cfg(feature = "const")]
    mod constructed_at_compile_time {
        use super::*;

        static PLACEHOLDER: RUID<Rolled> = RUID::new();

        #[test]
        fn takes_an_index_on_first_read_and_keeps_it() {
            let _g = serial();
            let first = PLACEHOLDER.get();
            let second = PLACEHOLDER.get();
            assert_eq!(first, second, "the index is taken once and kept");
        }

        #[test]
        fn cloning_carries_the_id_rather_than_taking_a_new_one() {
            let _g = serial();
            let original = RUID::new();
            let copy = original.clone();
            assert_eq!(original.get(), copy.get(), "a clone is the same id, not the next one");
        }
    }
}
