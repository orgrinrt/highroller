//! Two counters in one program, at two widths.
//!
//! The crate's own counter has its width chosen by a cargo feature, and a feature is
//! chosen once for a whole build graph. `declare_rolling_idx!` is how a program gets more
//! than one, or a width its consumer did not agree to.
//!
//! ```text
//! cargo run --example two_counters
//! ```

mod tickets {
    highroller::declare_rolling_idx!(u16);
}

mod sessions {
    highroller::declare_rolling_idx!(u32);
}

fn main() {
    println!("tickets are {} wide", tickets::_ROLLING_IDX_MAX);
    println!("sessions are {} wide", sessions::_ROLLING_IDX_MAX);

    for _ in 0 .. 3 {
        println!("ticket {}", tickets::rolling_idx());
    }

    // Neither counter is aware of the other: the sessions one is still at zero.
    println!("first session {}", sessions::rolling_idx());

    tickets::reset_rolling_idx();
    println!("tickets start again at {}", tickets::rolling_idx());
    println!("sessions carry on at {}", sessions::rolling_idx());
}
