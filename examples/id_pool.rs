//! A pool that hands out identified slots, composing most of the crate at once.
//!
//! Four things together: a counter of its own so the pool does not share the crate's,
//! `RUID`'s provenance so a slot's id cannot be forged, storage the caller lends rather
//! than the pool allocating, and a batch fill so reserving a run of ids costs one call.
//!
//! ```text
//! cargo run --example id_pool --no-default-features --features u32_index,ruid_type,no_alloc
//! ```

use highroller::{fill_rolling_idx, reset_rolling_idx, Derived, Idx, Rolled, RUID};

/// A counter belonging to this pool, at a width the pool chose.
mod pool_ids {
    highroller::declare_rolling_idx!(u32);
}

/// Slots handed out of storage the caller owns.
///
/// The pool allocates nothing. It is lent a region and fills it, which is what lets the
/// same code serve a stack array, a slice out of an arena, and a region from an allocator
/// the caller already holds.
struct Pool<'a> {
    slots: &'a mut [Idx],
    handed_out: usize,
}

impl<'a> Pool<'a> {
    /// Reserves a run of ids into the storage it is lent.
    fn new(storage: &'a mut [Idx]) -> Self {
        let filled = fill_rolling_idx(storage);
        println!("reserved {filled} ids in one call");
        Self { slots: storage, handed_out: 0 }
    }

    /// The next reserved id, if any are left.
    fn take(&mut self) -> Option<Idx> {
        let id = self.slots.get(self.handed_out).copied()?;
        self.handed_out += 1;
        Some(id)
    }

    fn remaining(&self) -> usize {
        self.slots.len() - self.handed_out
    }
}

fn main() {
    reset_rolling_idx();

    // The caller's memory, on the stack. The pool never asks where it came from.
    let mut backing = [0 as Idx; 8];
    let mut pool = Pool::new(&mut backing);

    while let Some(id) = pool.take() {
        if pool.remaining() % 3 == 0 {
            println!("slot {id}, {} left", pool.remaining());
        }
    }
    println!("pool drained, {} left", pool.remaining());

    // The pool's own counter is untouched by any of that: it is a different counter.
    println!("pool's own first id: {}", pool_ids::rolling_idx());

    // And an id the crate's counter issued carries its provenance in its type, so a
    // computed one cannot be passed where an issued one is required.
    let issued: RUID<Rolled> = RUID::new();
    let computed: RUID<Derived> = issued.into();
    println!("issued {issued} is rolled: {}", issued.is_rolled());
    println!("the same value as derived {computed} is rolled: {}", computed.is_rolled());
}
