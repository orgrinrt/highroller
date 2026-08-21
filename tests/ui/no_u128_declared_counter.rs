//! A 128-bit index has no wider atomic to count in, so `declare_rolling_idx!` refuses it
//! rather than silently reaching for a lock. The crate's own `u128_index` feature is where
//! that width lives, and it says what it costs.

mod huge {
    highroller::declare_rolling_idx!(u128);
}

fn main() {}
