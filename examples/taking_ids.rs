//! The smallest thing the crate does: ask for an id, get one nobody else got.
//!
//! ```text
//! cargo run --example taking_ids
//! ```

use highroller::{reset_rolling_idx, rolling_idx, _ROLLING_IDX_MAX};

fn main() {
    // The maximum rather than the count: the count is the maximum plus one, which at the
    // widest index is one past what that type can hold.
    println!("this build's index runs to {_ROLLING_IDX_MAX}");

    // Ephemeral and specific to one run. It starts at zero every time the process does,
    // and it is not stored anywhere.
    let first = rolling_idx();
    let second = rolling_idx();
    println!("took {first}, then {second}");
    assert_ne!(first, second);

    // Putting it back is a deliberate act, and it means values repeat from here.
    reset_rolling_idx();
    println!("after a reset, the next is {}", rolling_idx());
}
