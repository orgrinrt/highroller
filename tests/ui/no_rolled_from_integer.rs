//! A rolled id cannot be made out of a number.
//!
//! This is the whole guarantee. If it ever compiles, `RUID<Rolled>` means nothing.

use highroller::{Idx, Rolled, RUID};

fn main() {
    let _fabricated: RUID<Rolled> = RUID::from(5 as Idx);
}
