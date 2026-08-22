//! Taking a run of indices into storage the caller supplies.
//!
//! The whole file is behind `no_alloc`, which is where `fill_rolling_idx` exists. A doctest
//! on it cannot stand in for this: doctests run under the default features, so a doctest
//! gated on a non-default feature is compiled away and reports nothing.

#![cfg(feature = "no_alloc")]

use highroller::{fill_rolling_idx, reset_rolling_idx, rolling_idx, Idx};

#[test]
fn a_lend_is_filled_to_its_capacity() {
    reset_rolling_idx();
    let mut ids = [0 as Idx; 4];
    let taken = fill_rolling_idx(&mut ids);

    assert_eq!(taken, 4, "the whole lend is filled");
    assert_eq!(ids, [0, 1, 2, 3]);
}

#[test]
fn the_caller_chooses_how_many_by_choosing_the_size() {
    reset_rolling_idx();
    let mut ids = [0 as Idx; 1];
    assert_eq!(fill_rolling_idx(&mut ids), 1);
    assert_eq!(ids, [0]);
}

#[test]
fn an_empty_lend_takes_nothing_and_advances_nothing() {
    // The counter is only touched for a value that is kept. A `push` that refuses must not
    // have consumed an id, or the ids either side of an empty fill would have a gap.
    reset_rolling_idx();
    let mut nothing: [Idx; 0] = [];
    assert_eq!(fill_rolling_idx(&mut nothing), 0);
    assert_eq!(rolling_idx(), 0, "no id was consumed by the empty fill");
}

#[test]
fn a_slice_out_of_a_larger_region_is_lent_like_anything_else() {
    // The point of taking `Lend` rather than an array: an arena hands out a slice, and the
    // filler does not ask where it came from.
    reset_rolling_idx();
    let mut arena = [0 as Idx; 16];
    let region: &mut [Idx] = &mut arena[4 .. 8];
    assert_eq!(fill_rolling_idx(region), 4);

    assert_eq!(&arena[4 .. 8], &[0, 1, 2, 3], "the region was filled");
    assert_eq!(&arena[0 .. 4], &[0; 4], "and nothing either side of it was");
    assert_eq!(&arena[8 .. 12], &[0; 4]);
}

#[test]
fn filling_continues_the_same_sequence_as_taking_one_at_a_time() {
    // `fill_rolling_idx` is the same counter by the same route, so a fill and a single call
    // interleave without a gap or a repeat.
    reset_rolling_idx();
    assert_eq!(rolling_idx(), 0);

    let mut ids = [0 as Idx; 3];
    assert_eq!(fill_rolling_idx(&mut ids), 3);
    assert_eq!(ids, [1, 2, 3]);

    assert_eq!(rolling_idx(), 4, "the fill left the counter where it should be");
}
