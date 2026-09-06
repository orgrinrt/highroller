//! `Counter` as a value of its own, which is what the crate's global and the
//! macro are built on.
//!
//! Each test owns its counters, so nothing here shares state or takes a lock.
//! The exhaustion cases place the counter at the end of its width rather than
//! counting there, which at 64 bits would not finish.

use std::collections::HashSet;
use std::panic::catch_unwind;

use highroller::{Counter, Refuse, Width, Wrap};

#[test]
fn a_fresh_counter_starts_at_zero_and_steps_by_one() {
    let ids: Counter<u16> = Counter::new();
    let taken: Vec<u16> = (0 .. 8).map(|_| ids.next()).collect();
    assert_eq!(taken, (0 .. 8).collect::<Vec<u16>>());
}

#[test]
fn default_is_a_fresh_counter() {
    let ids: Counter<u32, Refuse> = Counter::default();
    assert_eq!(ids.next(), 0);
}

#[test]
fn two_counters_do_not_see_each_other() {
    let a: Counter<u8> = Counter::new();
    let b: Counter<u8> = Counter::new();
    assert_eq!(a.next(), 0);
    assert_eq!(a.next(), 1);
    assert_eq!(b.next(), 0, "b has its own count");
}

#[test]
fn it_sits_in_a_static() {
    static IDS: Counter<u32, Refuse> = Counter::new();
    assert_eq!(IDS.next(), 0);
    assert_eq!(IDS.next(), 1);
}

#[test]
fn it_sits_inside_a_value() {
    struct Arena {
        ids: Counter<u16>,
    }
    let arena = Arena {
        ids: Counter::new(),
    };
    assert_eq!(arena.ids.next(), 0);
    assert_eq!(arena.ids.next(), 1);
}

#[test]
fn the_policy_and_the_width_are_readable() {
    let wrapping: Counter<u8, Wrap> = Counter::new();
    let refusing: Counter<u8, Refuse> = Counter::new();
    assert!(!wrapping.refuses());
    assert!(refusing.refuses());
    assert_eq!(Counter::<u8, Wrap>::MAX, u8::MAX);
    assert_eq!(Counter::<u64, Refuse>::MAX, u64::MAX);
}

#[test]
fn exhaustion_is_observable_exactly_where_the_store_is_wider_than_the_width() {
    // The width is the question, not the feature name: on wasm32 and i686 a `usize`
    // is 32 bits and the store really is wider there.
    assert!(Counter::<u8>::new().exhaustion_is_observable());
    assert!(Counter::<u16>::new().exhaustion_is_observable());
    assert!(Counter::<u32>::new().exhaustion_is_observable());
    assert!(!Counter::<u64>::new().exhaustion_is_observable());
    assert_eq!(
        Counter::<usize>::new().exhaustion_is_observable(),
        core::mem::size_of::<usize>() < 8
    );
    // Always, at this width: the lock is already held, so a flag saying the width
    // is used up is free.
    assert!(Counter::<u128>::new().exhaustion_is_observable());
    // The method answers off the trait, so the two agree by construction; asserted
    // through a generic function so the comparison is between the two surfaces
    // rather than against a literal the lint folds away.
    fn agrees<W: Width>(counter: &Counter<W>) -> bool {
        counter.exhaustion_is_observable() == W::EXHAUSTION_IS_OBSERVABLE
    }
    assert!(agrees(&Counter::<u8>::new()));
    assert!(agrees(&Counter::<u128>::new()));
}

#[test]
fn set_next_places_the_counter_and_the_sequence_carries_on_from_there() {
    // The hook for a program persisting its last id: place the counter one past it.
    let ids: Counter<u32> = Counter::new();
    ids.set_next(41);
    assert_eq!(ids.next(), 41);
    assert_eq!(ids.next(), 42);
    ids.reset();
    assert_eq!(ids.next(), 0, "a reset is set_next(0)");
}

#[test]
fn the_last_value_of_every_width_is_handed_out() {
    // An earlier counter stopped one short and never handed out its own maximum.
    macro_rules! last_value {
        ($($t:ty),+) => {$(
            let ids: Counter<$t> = Counter::new();
            ids.set_next(<$t>::MAX);
            assert_eq!(ids.next(), <$t>::MAX, "{} hands out its maximum", stringify!($t));
        )+};
    }
    last_value!(u8, u16, u32, u64, usize, u128);
}

#[test]
fn a_wrapping_counter_returns_to_zero_past_its_width() {
    macro_rules! wraps {
        ($($t:ty),+) => {$(
            let ids: Counter<$t, Wrap> = Counter::new();
            ids.set_next(<$t>::MAX);
            let _ = ids.next();
            assert_eq!(ids.next(), 0, "{} wraps to zero", stringify!($t));
            assert_eq!(ids.next(), 1, "and carries on");
        )+};
    }
    wraps!(u8, u16, u32, u64, usize, u128);
}

#[test]
fn a_refusing_counter_panics_past_its_width_where_it_can_tell() {
    macro_rules! refuses {
        ($($t:ty),+) => {$(
            let ids: Counter<$t, Refuse> = Counter::new();
            ids.set_next(<$t>::MAX);
            assert_eq!(ids.next(), <$t>::MAX, "{}: the whole width is usable", stringify!($t));
            if ids.exhaustion_is_observable() {
                assert!(catch_unwind(|| ids.next()).is_err(), "{} refuses", stringify!($t));
                assert!(
                    catch_unwind(|| ids.next()).is_err(),
                    "{} keeps refusing rather than refusing once", stringify!($t)
                );
                ids.reset();
                assert_eq!(ids.next(), 0, "{}: a reset is the way back", stringify!($t));
            } else {
                // No wider store to count in, so the policy cannot be acted on and the
                // counter wraps like the other one. Said in the documentation, and held
                // here so it does not become an undocumented surprise.
                assert_eq!(ids.next(), 0, "{} cannot tell, and wraps", stringify!($t));
            }
        )+};
    }
    refuses!(u8, u16, u32, u64, usize, u128);
}

#[test]
fn the_refusal_is_not_made_of_a_default_hook_message() {
    // The panic names the crate and says what to do, so a consumer seeing it in a
    // log knows which counter and which way out.
    let ids: Counter<u8, Refuse> = Counter::new();
    ids.set_next(u8::MAX);
    let _ = ids.next();
    let message = catch_unwind(|| ids.next())
        .expect_err("an exhausted counter refuses")
        .downcast::<String>()
        .expect("the panic carries a formatted message");
    assert!(message.contains("highroller"), "{message}");
    assert!(message.contains("2^8"), "{message}");
    assert!(message.contains("wrap"), "{message}");
}

#[test]
fn threads_taking_from_one_counter_never_receive_the_same_value() {
    macro_rules! contended {
        ($($t:ty),+) => {$(
            let ids: Counter<$t> = Counter::new();
            let taken: Vec<$t> = std::thread::scope(|s| {
                let handles: Vec<_> = (0 .. 8)
                    .map(|_| s.spawn(|| (0 .. 32).map(|_| ids.next()).collect::<Vec<$t>>()))
                    .collect();
                handles.into_iter().flat_map(|h| h.join().expect("no thread panics")).collect()
            });
            let distinct: HashSet<$t> = taken.iter().copied().collect();
            assert_eq!(distinct.len(), 256, "{}: no two threads received the same value", stringify!($t));
            let mut sorted = taken;
            sorted.sort_unstable();
            assert_eq!(
                sorted,
                (0 .. 256).map(|v| v as $t).collect::<Vec<$t>>(),
                "{}: and none was skipped", stringify!($t)
            );
        )+};
    }
    // Every store, including the locked one at 128 bits.
    contended!(u16, u64, u128);
}

#[test]
fn debug_names_the_width_and_the_policy_and_not_the_count() {
    let ids: Counter<u16, Refuse> = Counter::new();
    let printed = format!("{ids:?}");
    assert!(printed.contains("u16"), "{printed}");
    assert!(printed.contains("refuses: true"), "{printed}");
    assert!(printed.starts_with("Counter {"), "{printed}");
}
