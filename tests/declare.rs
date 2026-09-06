//! `declare_rolling_idx!` at the widths it accepts, and at the one it refuses.
//!
//! Each counter here lives in its own module, which is how the macro is meant
//! to be used and also what keeps these tests from sharing state: the crate's
//! own counter is global and its tests take a lock, and none of that applies to
//! a counter declared here.
//!
//! A counter declared here wraps unless the invocation says `strict`, and both
//! policies are exercised below at the end of a width, placed there rather than
//! counted to.

mod narrow {
    highroller::declare_rolling_idx!(u8);
}

mod middling {
    highroller::declare_rolling_idx!(u16);
}

mod wide {
    highroller::declare_rolling_idx!(u32);
}

mod pointer_wide {
    highroller::declare_rolling_idx!(usize);
}

mod independent {
    highroller::declare_rolling_idx!(u32);
}

mod resettable {
    highroller::declare_rolling_idx!(u32);
}

mod threaded {
    highroller::declare_rolling_idx!(u32);
}

mod huge {
    highroller::declare_rolling_idx!(u128);
}

mod refusing {
    highroller::declare_rolling_idx!(u8, strict);
}

mod wrapping_by_name {
    highroller::declare_rolling_idx!(u8, wrap);
}

#[test]
fn each_width_reports_its_own_maximum() {
    assert_eq!(narrow::_ROLLING_IDX_MAX, u8::MAX);
    assert_eq!(middling::_ROLLING_IDX_MAX, u16::MAX);
    assert_eq!(wide::_ROLLING_IDX_MAX, u32::MAX);
    assert_eq!(pointer_wide::_ROLLING_IDX_MAX, usize::MAX);
    assert_eq!(huge::_ROLLING_IDX_MAX, u128::MAX);
}

#[test]
fn a_128_bit_counter_is_declared_like_any_other() {
    // This width used to be refused by the macro with a `compile_error!`, because
    // the generated counter was a `u64` and had nowhere to keep a wider count.
    // The counter is a type now, and the type carries a lock at this width, so
    // the width is ordinary here.
    assert_eq!(huge::rolling_idx(), 0);
    assert_eq!(huge::rolling_idx(), 1);
    huge::ROLLING_IDX.set_next(u128::MAX);
    assert_eq!(
        huge::rolling_idx(),
        u128::MAX,
        "the last value is handed out"
    );
    assert_eq!(
        huge::rolling_idx(),
        0,
        "and then it wraps, since nothing said strict"
    );
}

#[test]
fn the_policy_is_readable_off_the_declared_counter() {
    assert!(refusing::ROLLING_IDX.refuses());
    assert!(!wrapping_by_name::ROLLING_IDX.refuses());
    assert!(
        !narrow::ROLLING_IDX.refuses(),
        "left out, the policy is wrap"
    );
}

#[test]
fn a_strict_declared_counter_refuses_past_its_width() {
    refusing::ROLLING_IDX.set_next(u8::MAX);
    assert_eq!(
        refusing::rolling_idx(),
        u8::MAX,
        "the whole width is usable"
    );
    let outcome = std::panic::catch_unwind(refusing::rolling_idx);
    assert!(outcome.is_err(), "the value after the last one is refused");
    let again = std::panic::catch_unwind(refusing::rolling_idx);
    assert!(
        again.is_err(),
        "and it keeps refusing rather than refusing once"
    );
    // A reset is the way back, and it means what it says.
    refusing::reset_rolling_idx();
    assert_eq!(refusing::rolling_idx(), 0);
}

#[test]
fn wrap_named_explicitly_is_the_same_as_leaving_it_out() {
    wrapping_by_name::ROLLING_IDX.set_next(u8::MAX);
    assert_eq!(wrapping_by_name::rolling_idx(), u8::MAX);
    assert_eq!(wrapping_by_name::rolling_idx(), 0);
}

#[test]
fn a_declared_counter_starts_at_zero_and_advances_by_one() {
    assert_eq!(middling::rolling_idx(), 0);
    assert_eq!(middling::rolling_idx(), 1);
    assert_eq!(middling::rolling_idx(), 2);
}

#[test]
fn two_counters_at_two_widths_do_not_see_each_other() {
    // A module of its own, because these tests run in parallel and a counter shared
    // between two of them is the state this file's per-module shape exists to
    // avoid.
    assert_eq!(independent::rolling_idx(), 0);
    assert_eq!(independent::rolling_idx(), 1);
    let _ = middling::rolling_idx();
    assert_eq!(independent::rolling_idx(), 2);
}

#[test]
fn it_wraps_at_the_index_width_rather_than_the_counter_width() {
    // The counter is a `u64` at every width, so a `u8` index wraps 2^56 times
    // before the counter would. Hand out the whole width and the next call is
    // zero again.
    for expected in 0 ..= u8::MAX {
        assert_eq!(
            narrow::rolling_idx(),
            expected,
            "every value of the width is handed out, in order"
        );
    }
    assert_eq!(
        narrow::rolling_idx(),
        0,
        "past its maximum a declared index returns to the start, since strict is the \
         crate's own counter's property and does not reach here"
    );
    assert_eq!(narrow::rolling_idx(), 1, "and keeps going from there");
}

#[test]
fn the_maximum_is_handed_out_rather_than_stopped_one_short() {
    // Distinct from the wrap: an earlier version of the crate's own counter stopped
    // one value early, so the width's last value was never seen by anybody.
    pointer_wide::reset_rolling_idx();
    let seen: Vec<usize> = (0 .. 4).map(|_| pointer_wide::rolling_idx()).collect();
    assert_eq!(seen, vec![0, 1, 2, 3]);
}

#[test]
fn a_reset_puts_it_back_and_values_repeat() {
    let first = resettable::rolling_idx();
    let second = resettable::rolling_idx();
    assert_ne!(first, second);
    resettable::reset_rolling_idx();
    assert_eq!(
        resettable::rolling_idx(),
        0,
        "a reset means every value handed out before it can be handed out again"
    );
}

#[test]
fn eight_threads_taking_sixty_four_each_get_no_repeat() {
    use std::collections::HashSet;

    threaded::reset_rolling_idx();

    // No sleeps and no barrier: the point is that the atomic is sufficient under
    // whatever interleaving the machine happens to produce, and a sleep would
    // only make one interleaving likelier rather than proving anything about
    // the rest.
    let handles: Vec<_> = (0 .. 8)
        .map(|_| {
            std::thread::spawn(|| {
                (0 .. 64)
                    .map(|_| threaded::rolling_idx())
                    .collect::<Vec<_>>()
            })
        })
        .collect();

    let mut all = Vec::new();
    for h in handles {
        all.extend(h.join().expect("no thread panicked"));
    }

    assert_eq!(all.len(), 512);
    let unique: HashSet<_> = all.iter().copied().collect();
    assert_eq!(unique.len(), 512, "no two threads received the same index");

    let mut sorted = all;
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        (0 .. 512u32).collect::<Vec<_>>(),
        "and the 512 handed out are exactly 0 through 511, so none was skipped either"
    );
}
