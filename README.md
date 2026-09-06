# `highroller`

<div align="center" style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/highroller.svg)](https://github.com/orgrinrt/highroller/stargazers)
[![Crates.io](https://img.shields.io/crates/v/highroller)](https://crates.io/crates/highroller)
[![docs.rs](https://img.shields.io/docsrs/highroller)](https://docs.rs/highroller)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/highroller.svg)](https://github.com/orgrinrt/highroller/issues)
![License](https://img.shields.io/github/license/orgrinrt/highroller?color=%23009689)

> A simple, thread-safe rolling index handing out cheap runtime-unique ids. Ships with a typed `RUID` over it, a macro for declaring more counters, and a `no_std` build.

</div>

A rolling index is a counter that gives out the next number every time it's asked, and this
crate keeps one as a static, so any thread anywhere in the program gets an id nobody else in
that run has been handed, for the price of one atomic add. The ids hold for one run of the
process and no longer, since nothing is stored anywhere and the counter starts from zero
again with the program, which is enough for most of the things a uuid tends to get reached
for (entities in a game, handles in a registry, request ids in a log) and quite a bit
cheaper than one.

The counter underneath is 64 bits wide whatever width the index has, which is what lets a
single relaxed `fetch_add` be the whole of it. Running past the index's maximum then shows
up as a comparison instead of a second decision the add can't make on its own, and wrapping
back to zero is the narrowing cast that happens anyway, as every width's range is a power of
two. Do note that the width is a cargo feature, `u16_index` by default, so the index is a
`u16` unless the manifest says otherwise, and `highroller::Idx` names whichever one got
picked.

On top of that there's `RUID`, behind the `ruid_type` feature, which is the same index as a
type of its own, carrying in its type parameter whether the counter actually produced the
value or somebody computed it. And `declare_rolling_idx!` for the cases where one counter
isn't enough, or a library wants a counter at a width its consumer never agreed to. The
crate's own code is `core` only at every width but `u128`, and allocates nothing in any
configuration, so the `no_std` feature is the attribute and little else.

## Usage

```bash
cargo add highroller
```

Only one width may be on at a time, as each one defines the same `Idx` and two of them is a
duplicate definition, so picking another means turning the default off as well. Both
mistakes, two widths and none, refuse at compile time with a message naming the flags,
which is also why `--all-features` can't build this crate:

```bash
cargo add highroller --no-default-features --features u32_index,strict
```

The whole of it is `rolling_idx()`, which returns the current value and moves the index
forward by one:

```rust
use highroller::{reset_rolling_idx, rolling_idx, Idx, _ROLLING_IDX_MAX};

let first: Idx = rolling_idx();
let second = rolling_idx();
assert_ne!(first, second);

// the last value this width hands out, so u16::MAX on the default build
assert_eq!(_ROLLING_IDX_MAX, Idx::MAX);

// back to zero, after which every value given out so far can be handed out again
reset_rolling_idx();
assert_eq!(rolling_idx(), 0);
```

The reset is for a program that knows the previous run of ids is done with, a test starting
each case from zero or an arena being reused, and it isn't synchronised against readers, so
a thread calling `rolling_idx()` while it runs gets a value from one side or the other of
it.

What happens at the end of the width is the `strict` feature's business. With it, on by
default, the index panics once its maximum has been passed and keeps panicking on every
call after, and without it the index goes back to zero and values start repeating, which
is fine where ids only have to differ among things alive at the same time and not fine
otherwise. Running out is only visible where the index is narrower than the counter, so at
`u64_index`, and at `usize_index` on a 64-bit target, `strict` has nothing to observe,
though no program gets there either, as a thousand million ids a second takes about five
hundred years to use up a 64-bit index. `u128_index` is the one width with no atomic to sit
in, so it keeps a lock instead and notices exhaustion on its own.

A second counter, or one at a different width, comes from `declare_rolling_idx!`, which
writes the same three names into whichever module it's invoked in:

```rust
mod tickets {
    highroller::declare_rolling_idx!(u16);
}

mod sessions {
    highroller::declare_rolling_idx!(u32);
}

assert_eq!(tickets::rolling_idx(), 0);
assert_eq!(tickets::rolling_idx(), 1);
assert_eq!(sessions::rolling_idx(), 0); // its own counter, untouched by the tickets
assert_eq!(tickets::_ROLLING_IDX_MAX, u16::MAX);
```

It costs the same as the crate's own counter and is built the same way, although a
declared one always wraps, since the `strict` flag belongs to this crate's features and a
macro expanding in some other crate can't read them, so refusing there is a comparison
against `_ROLLING_IDX_MAX` at the call site. `u128` is refused by the macro outright, for
the lack of a wider atomic to count in.

## Example

Consider a game where fighters get summoned into arenas, from several threads at once, and
each needs a name that stays distinct for as long as the match runs and nothing beyond
that. A uuid would be paying for properties nobody reads here, so the ids come off the
rolling index instead, and `Idx` keeps the struct's field at whatever width the build
picked:

```rust
use highroller::Idx;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
struct Fighter {
    id: Idx,
    power: u32,
}

let register = Arc::new(Mutex::new(Vec::new()));
let arenas = 4;

// twenty fighters per arena, each arena on a thread of its own
let mut handles = Vec::new();
for _ in 0 .. arenas {
    let register = Arc::clone(&register);
    handles.push(thread::spawn(move || {
        let mut ids = Vec::new();
        for n in 0 .. 20u32 {
            let fighter = Fighter {
                id: highroller::rolling_idx(), // distinct across all four threads
                power: (n * 37 + 11) % 100,   // stands in for a real stat
            };
            ids.push(fighter.id);
            register.lock().unwrap().push(fighter);
        }
        ids
    }));
}

// a champion per arena, looked up by the id the arena kept
let mut champions = Vec::with_capacity(arenas);
for handle in handles {
    let arena = handle.join().unwrap();
    let fighters = register.lock().unwrap();
    let champion = arena
        .iter()
        .map(|&id| fighters.iter().find(|f| f.id == id).unwrap())
        .max_by_key(|f| f.power)
        .unwrap()
        .clone();
    champions.push(champion);
}

// and one between the arenas
let winner = champions.into_iter().max_by_key(|f| f.power).unwrap();
println!("the champion is fighter {}", winner.id);
```

Eighty fighters, four threads, no two ids alike, and nothing taken but one atomic add per
fighter. The index goes back to zero with the next run of the program, so anything that has
to survive a restart wants a different tool than this.

## Motivation

At times what's needed is a plain guaranteed-unique identifier for something, and a uuid
brings along a cost and a dependency the case likely never asked for, when it's simple and
not very extensive. A static counter that increments itself after every fetch, with the
thread-safety measures added in, covers a surprising amount of that ground and is very
cheap to run.

The cheapness is the point of the design, so it's measured. `benches/rolling.rs` keeps the
alternatives as arms beside the shipped function, a global mutex (what a first version of
such a counter tends to be) and a compare-and-swap loop (the obvious thing to replace the
mutex with). Uncontended on the machine this was written on, the mutex measured 8.9 ns, the
loop 2.5 ns and `rolling_idx` 2.0 ns, and with eight threads each rolling two thousand ids,
583 µs, 531 µs and 195 µs for the batch, thread spawning included. The loop is the
interesting row, being barely better than the lock under
contention, as every thread losing the race retries and the retries become the work. Take
the numbers as one machine's though, and rerun the bench where it matters.

What it costs otherwise is a global. The counter is a static, so two crates in one build
share it, which is what `declare_rolling_idx!` is for, and the width is chosen for the
whole build graph by whichever manifest names it.

## Extras

### Status

Early days still, so the api hasn't fully settled and the next release can move things.
Every release is tagged and the log between two tags is what actually changed, and we'll
try to keep the notes on that worth reading. The floor is rust 1.70, as that is where
`OnceLock` arrived, and the manifest's `rust-version` says so.

### Cargo features

| Feature | Default | Effect |
|---|---|---|
| `u16_index` | on | The width of the index. Exactly one of `u8_index`, `u16_index`, `u32_index`, `u64_index`, `u128_index` and `usize_index` is on, and `Idx` is that integer. |
| `strict` | on | Panics when the width is used up instead of wrapping. Also withholds `==` and `<` between a `RUID` and a bare `Idx`. |
| `ruid_type` | off | `RUID`, the index as a type of its own with its provenance in the type. |
| `allow_arithmetics` | off | The arithmetic operators on `RUID`, by value and by reference. |
| `const` | off | Makes `RUID::new()` a `const fn`. The index is taken on first read, so `RUID` is not `Copy` and has no `Deref`, `Borrow` or `AsRef`. Needs std. |
| `no_std` | off | The `#![no_std]` attribute. Excludes `const` and `u128_index`, and says so at compile time. |
| `no_alloc` | off | Implies `no_std` and adds `fill_rolling_idx`, which takes a run of ids into storage the caller lends. Pulls `notko` in for the lending contract. |
| `async` | off | Gates nothing. `RUID` is `Send` and `Sync` on its own and the crate asserts so at compile time; the flag is kept so naming it isn't an error. |

`strict` carries two meanings, what happens when the width runs out and whether an id is
opaque against an integer, and one flag can't currently say one and not the other.

### RUID

With `ruid_type`, ids can be `RUID`s, which stops one being passed where another was
meant, and a `RUID` says in its type where the value came from. `RUID<Rolled>` is one the
counter handed out, and a bare `RUID` means that one. `RUID<Derived>` is anything else:
built from an integer, parsed, or the result of arithmetic. The two compare, hash and print
alike, as two ids naming the same thing are the same id, and a rolled one converts into a
derived one freely, but nothing goes the other way.

```rust
# #[cfg(feature = "ruid_type")]
# {
use highroller::{Derived, Idx, Rolled, RUID};

// the counter handed this out, so nothing else in this run holds it
let id: RUID<Rolled> = RUID::new();

// built from a number, so it carries no such promise
let from_config: RUID<Derived> = RUID::from(7 as Idx);

assert!(id.is_rolled());
assert!(!from_config.is_rolled());
assert_ne!(id, from_config);

// the value survives the conversion, the guarantee doesn't
let plain: RUID<Derived> = id.to_derived();
assert_eq!(plain.get(), id.get());
# }
```

`RUID::new()` is the only way to a `RUID<Rolled>`. There's no `From<Idx>` for it and no
way to promote a derived id, and with `allow_arithmetics` on, every operator produces a
`RUID<Derived>` whatever it was given, since the result is a number the counter never
issued and might well collide with one it did. The assigning forms (`+=` and the rest)
exist on `Derived` only, for the same reason, and the compile-fail suite under `tests/ui/`
pins all three refusals.

Beyond that a `RUID` behaves like the integer inside it: ordered, hashable, `Display` and
`Debug`, `Binary`, `Octal`, `LowerHex` and `UpperHex` with the format flags reaching
through, `FromStr` into a `Derived`, and `Deref`, `Borrow<Idx>` and `AsRef<Idx>` so it can
key a map that gets looked up by index. `Default` on a `RUID<Rolled>` takes a fresh index,
so a `#[derive(Default)]` type holding one still gets an id instead of a zero.

Under the `const` feature `RUID::new()` becomes a `const fn`, so a `RUID` can sit in a
`static` or an associated constant:

```rust
# #[cfg(all(feature = "ruid_type", feature = "const"))]
# {
use highroller::RUID;

static ID: RUID = RUID::new();

// no index has been taken yet at this point. The first read takes one and keeps it,
// so every later read agrees with the first, from any thread
assert_eq!(ID.get(), ID.get());
# }
```

A `const fn` can't ask the counter for anything, so the value starts unassigned and the
first read takes it, which needs somewhere to write the result and that is where `Copy`,
`Deref`, `Borrow` and `AsRef` go. `get()` reads it, a clone carries the id already taken
and never a fresh one, and the reference forms of the operators (`&a + &b`) are there for
a value that's needed twice.

### Limitations

The ids are ephemeral by design, so anything that has to hold across restarts or between
processes wants a store of its own, or a uuid after all.

`no_std` and `const` are exclusive, since `const` keeps the value in a `std::sync::OnceLock`
and `core` has no shareable equivalent, and so are `no_std` and `u128_index`, the lock
being a `std::sync::Mutex`. Both refuse at compile time with the way out named. `u128` is
also the width where the id stops being cheap, and probably the one nobody needs.

`no_alloc` is the one feature with a dependency, `notko`, whose `Lend` contract is what
`fill_rolling_idx` is written against. Nothing in the crate allocates with or without it,
so the flag declares an absence and adds the batch fill, and that's all it does.

## Support

Feel free to contribute! If unsure about wasting work, the best practice is to throw in an issue describing what you'd do, and only then commit to writing a big PR, because chances are, it might not be something that belongs here. However, forks are always a valid choice and we'd encourage everyone to experiment and have their own takes on this. When doing this, do mind the license(s) though!

A new refusal wants a case in the compile-fail suite beside it, and a new example wants a line in `tests/examples.rs`, since that is what keeps the examples running and not only compiling.

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/highroller/blob/dev/LICENSE)
