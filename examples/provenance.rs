//! What `RUID`'s type parameter is for: telling an id the counter produced apart from one
//! somebody computed.
//!
//! ```text
//! cargo run --example provenance --features ruid_type,allow_arithmetics
//! ```

use highroller::{Derived, Rolled, RUID};

fn main() {
    // A `RUID<Rolled>` is reachable only by asking the counter. There is no
    // `From<Idx> for RUID<Rolled>` and the field is private, which together are what make
    // the parameter mean anything.
    let issued: RUID<Rolled> = RUID::new();
    println!("issued {issued}, rolled: {}", issued.is_rolled());

    // Anything computed from one is `Derived`, whatever it was computed from. The counter
    // never handed out this value, and the type says so.
    let offset: RUID<Derived> = issued + 10;
    println!("offset {offset}, rolled: {}", offset.is_rolled());

    // A value that came in from outside is `Derived` too, by the same rule.
    let from_elsewhere = RUID::<Derived>::from(42);
    println!("from outside {from_elsewhere}, rolled: {}", from_elsewhere.is_rolled());

    // The assigning operators exist only on `Derived`. Applying one to a rolled id would
    // change it in place into a value the counter never produced, which is the hole the
    // whole arrangement closes, so the type refuses instead. Uncommenting this line is a
    // compile error, and `tests/ui/` pins that it stays one:
    //
    //     let mut rolled = RUID::<Rolled>::new();
    //     rolled += 1;

    let mut derived = from_elsewhere;
    derived += 1;
    println!("derived and then advanced: {derived}");
}
