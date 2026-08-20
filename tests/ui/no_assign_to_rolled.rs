//! Arithmetic cannot be assigned into a rolled id.
//!
//! Doing so would change it in place into a value the counter never handed out, while it
//! went on claiming otherwise. The assigning operators exist only for `RUID<Derived>`.

use highroller::{Idx, RUID};

fn main() {
    let mut rolled = RUID::new();
    rolled += 1 as Idx;
}
