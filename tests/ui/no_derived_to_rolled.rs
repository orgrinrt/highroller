//! And a derived id cannot be promoted into a rolled one.
//!
//! The conversion exists in the other direction only, because a rolled id genuinely is a
//! derived one and nothing this crate could check makes the reverse true.

use highroller::{Derived, Idx, Rolled, RUID};

fn main() {
    let derived: RUID<Derived> = RUID::from(5 as Idx);
    let _promoted: RUID<Rolled> = derived.into();
}
