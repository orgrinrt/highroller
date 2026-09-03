//! `declare_rolling_idx!` at the widths it accepts, and at the one it refuses.
//!
//! Each counter here lives in its own module, which is how the macro is meant to be used
//! and also what keeps these tests from sharing state: the crate's own counter is global
//! and its tests take a lock, and none of that applies to a counter declared here.
//!
//! The refusal at `u128` is a `compile_error!`, so it is pinned by `tests/ui/` rather
//! than by anything in this file. A test that could observe it would have to compile.

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

#[test]
fn each_width_reports_its_own_maximum() {
    assert_eq!(narrow::_ROLLING_IDX_MAX, u8::MAX);
    assert_eq!(middling::_ROLLING_IDX_MAX, u16::MAX);
    assert_eq!(wide::_ROLLING_IDX_MAX, u32::MAX);
    assert_eq!(pointer_wide::_ROLLING_IDX_MAX, usize::MAX);
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
    // between two of them is the state this file's per-module shape exists to avoid.
    assert_eq!(independent::rolling_idx(), 0);
    assert_eq!(independent::rolling_idx(), 1);
    let _ = middling::rolling_idx();
    assert_eq!(independent::rolling_idx(), 2);
}

#[test]
fn it_wraps_at_the_index_width_rather_than_the_counter_width() {
    // The counter is a `u64` at every width, so a `u8` index wraps 2^56 times before the
    // counter would. Hand out the whole width and the next call is zero again.
    for expected in 0..=u8::MAX {
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
    // Distinct from the wrap: an earlier version of the crate's own counter stopped one
    // value early, so the width's last value was never seen by anybody.
    pointer_wide::reset_rolling_idx();
    let seen: Vec<usize> = (0..4).map(|_| pointer_wide::rolling_idx()).collect();
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

    // No sleeps and no barrier: the point is that the atomic is sufficient under whatever
    // interleaving the machine happens to produce, and a sleep would only make one
    // interleaving likelier rather than proving anything about the rest.
    let handles: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| (0..64).map(|_| threaded::rolling_idx()).collect::<Vec<_>>())
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
        (0..512u32).collect::<Vec<_>>(),
        "and the 512 handed out are exactly 0 through 511, so none was skipped either"
    );
}
