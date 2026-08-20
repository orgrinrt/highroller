highroller
============
<div style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/highroller.svg)](https://github.com/orgrinrt/highroller/stargazers) 
[![Crates.io Total Downloads](https://img.shields.io/crates/d/highroller)](https://crates.io/crates/highroller)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/highroller.svg)](https://github.com/orgrinrt/highroller/issues) 
[![Current Version](https://img.shields.io/badge/version-0.2.0-orange.svg)](https://github.com/orgrinrt/highroller) 

>A simple, high-level rolling index that is thread-safe and guarantees cheap runtime-unique IDs.

</div>

# Usage

A statically available, thread-safe rolling index, for the cases where a UUID is more
than the situation calls for. Something needs a distinct name for as long as the program
runs, nothing needs that name to mean anything afterwards, and a counter is the whole
answer.

The main function is `rolling_idx()`:

```rust
let id1 = highroller::rolling_idx();
let id2 = highroller::rolling_idx();
println!("Id 1 is: {}", id1);
println!("Id 2 is: {}", id2);
// outputs:
// Id 1 is: 0
// Id 2 is: 1
```

It returns the current value and then increases it, so each call gives a value no earlier
call gave.

> The index is specific to one run. It starts at zero every time the process does, and it
> is not written anywhere.

It is safe to call from any number of threads at once. The test suite spawns up to 256
threads, one per id, with no staggering delays, and asserts that every returned id
differs.

## What it costs

Rolling an index is one atomic add, which measured at 2.0ns on the machine this was
written on.

The counter is deliberately wider than the index. That sounds like a detail and is the
whole design: an index has to wrap or refuse at its own maximum, which is a second
decision that an atomic add cannot express, so a counter of the same width needs a
compare-and-swap loop instead. A wider counter passes the narrow maximum long before it
could wrap itself, so exhaustion becomes a comparison and wrapping becomes the narrowing
cast that was happening anyway.

The alternatives, from `benches/rolling.rs`, which keeps all of them as arms:

| | one thread | 2 threads | 4 threads | 8 threads |
|---|---:|---:|---:|---:|
| a mutex | 8.9 ns | 71 µs | 298 µs | 583 µs |
| a compare-and-swap loop | 2.5 ns | 36 µs | 102 µs | 531 µs |
| `rolling_idx` as it ships | 2.0 ns | 35 µs | 72 µs | 195 µs |

The last row is `highroller::rolling_idx` itself rather than a copy of it written for the
benchmark. That distinction is not pedantry: the stand-in this replaced masked where the
shipped path does not, and its strict variant declared a branch unreachable that a shared
counter carried it into on 93% of calls.

The compare-and-swap row is the one worth looking at. It is the obvious replacement for a
mutex, and under real contention it is barely better than one, because every thread that
loses the race retries and the retries become the work.

`u128_index` is the exception and keeps a lock, because there is no 128-bit atomic to put
a counter in. It is also the width nobody needs: a program exhausting a 64-bit index at
one per nanosecond has been running for five hundred years.

### Feature Flags

| Feature Flag           | Default     | Description                                                                                                                                                             |
|------------------------|-------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `strict`               | *Enabled*   | Panics when the width is exhausted, rather than wrapping and repeating. Also withholds comparison and arithmetic between `RUID` and a bare integer.                     |
| `ruid_type`            | *Disabled*  | Enables `RUID`, a type of its own over the rolling index.                                                                                                              |
| `allow_arithmetics`    | *Disabled*  | Arithmetic operators on `RUID`, by value and by reference.                                                                                                             |
| `const`                | *Disabled*  | Makes `RUID::new()` a `const fn`. The index is taken on first read instead, so `RUID` is not `Copy` and has no `Deref` under this flag.                                 |
| `async`                | *Disabled*  | Kept so that naming it is not an error. It gates nothing: `RUID` is `Send` and `Sync` on its own, and the crate asserts so at compile time.                             |
| size (separate flags)  | `u16_index` | The width of the index: `u8_index`, `u16_index`, `u32_index`, `u64_index`, `u128_index`, `usize_index`                                                                 |

`strict` carries two unrelated meanings, and that is a wart rather than a design. One is
what happens when the width runs out. The other is whether a `RUID` may be compared
against a bare integer. If you want one and not the other, the flag cannot currently say
so.

**Exactly one size flag can be on at a time.** Each one defines the same index type, so
two of them is a duplicate definition rather than a wider index. The default is
`u16_index`, which means picking a different size also means turning the default off:

```toml
[dependencies]
highroller = { version = "0.2", default-features = false, features = ["u32_index", "strict"] }
```

Turning the default off without naming a size leaves no index type at all. Both mistakes
are caught at compile time with a message saying which one happened. The same constraint
is why `cargo build --all-features` cannot work on this crate.

The chosen width is also exported as `highroller::Idx`, so code that stores an id can name
its type without repeating the choice.

### RUID

`RUID` is the rolling index wearing a type of its own, and carrying a record of whether
the counter really produced it. Enable `ruid_type`, and see [Extras](#extras).

## Example

Consider a game where you summon fighters. Each needs a name that is distinct for as long
as the match runs, and nothing more than that, so a UUID per fighter would be paying for
properties nobody uses.

```rust
use highroller::Idx;
use std::sync::{Arc, Mutex};
use std::thread;

// `Idx` is whichever width the size flag selected, so this follows the choice rather
// than repeating it
#[derive(Clone)]
struct Fighter {
  id: Idx,
  power: u32,
}

let fighters_register = Arc::new(Mutex::new(Vec::new()));
let arenas = 4;

// gather twenty fighters for each arena, on its own thread
let mut handlers = Vec::new();
for _ in 0..arenas {
  let fighters_register = Arc::clone(&fighters_register);
  handlers.push(thread::spawn(move || {
    let mut ids = Vec::new();
    for n in 0..20u32 {
      let fighter = Fighter {
        id: highroller::rolling_idx(),
        power: (n * 37 + 11) % 100, // stand-in for a real power stat
      };
      ids.push(fighter.id);
      fighters_register.lock().unwrap().push(fighter);
    }
    ids
  }));
}

// find a champion per arena
let mut champions = Vec::with_capacity(arenas);
for handler in handlers {
  let arena_fighters = handler.join().unwrap();
  let fighters = fighters_register.lock().unwrap();
  let champion = arena_fighters.iter()
    .map(|&id| fighters.iter().find(|fighter| fighter.id == id).unwrap())
    .max_by_key(|fighter| fighter.power)
    .unwrap()
    .clone();
  champions.push(champion);
}

// and then between the arenas
let ultimate_champion = champions.into_iter()
  .max_by_key(|fighter| fighter.power)
  .unwrap();

println!("The ultimate champion is fighter with id: {}", ultimate_champion.id);
```

Every fighter gets a distinct id, from four threads at once, without any of the machinery
a UUID would bring. The index resets when the program does, so anything that has to
survive a restart needs a different tool.

## Running out

The index is as wide as the size flag says, so it has that many values and no more. What
happens at the end is the `strict` flag's business, and the whole range is usable in
either case.

With `strict`, it panics, and keeps panicking. Without it, the index returns to zero and
values start repeating, which is fine when ids only have to be distinct among things alive
at the same time, and is not fine otherwise.

At `u64_index` and `usize_index` the counter is no wider than the index, so exhaustion is
not detectable there. It is also not reachable: a thousand million ids a second exhausts a
64-bit index in about five hundred years.

## Extras

With `ruid_type`, ids are `RUID`s rather than bare integers, which stops one being passed
where another was meant. A `RUID` also says **where its value came from**, in its type.

```rust
# #[cfg(feature = "ruid_type")]
# {
use highroller::{Derived, Idx, Rolled, RUID};

// Rolled: the counter handed this out, so nothing else in this run holds it
let id: RUID<Rolled> = RUID::new();

// Derived: built from a number, so it carries no such promise
// `Idx` follows whichever width the size flag chose, so this example does too
let from_config: RUID<Derived> = RUID::from(7 as Idx);

assert!(id.is_rolled());
assert!(!from_config.is_rolled());

// they compare and hash alike, because two ids naming the same thing are the same id
assert_ne!(id, from_config);
# }
```

`RUID<Rolled>` is the one with the guarantee, and `RUID` on its own means that one. The
only way to obtain it is `RUID::new()`. There is no `From<Idx>` for it, no way to promote
a derived id into one, and arithmetic on one produces a `RUID<Derived>`, because the
result is a number the counter never handed out and might well collide with one it did.

That distinction is the reason the type exists. Before it, `RUID::from(5)` produced
something indistinguishable from a rolled id and `a + b` produced another, so the promise
of uniqueness held only while nobody used the rest of the surface. Three compile-fail
tests hold it now.

A rolled id converts into a derived one whenever you want the value without the claim:

```rust
# #[cfg(feature = "ruid_type")]
# {
use highroller::{Derived, RUID};

let id = RUID::new();
let plain: RUID<Derived> = id.to_derived();
assert_eq!(plain.get(), id.get()); // the value survives; the guarantee does not

// `into_derived` is the same thing when the original is no longer wanted
let consumed: RUID<Derived> = id.into_derived();
assert_eq!(consumed, plain);
# }
```

Beyond that a `RUID` behaves like the integer it holds: ordered, hashable, `Display` and
`Debug`, and `Binary`, `Octal`, `LowerHex` and `UpperHex` with the format flags reaching
through, plus `FromStr`, `Borrow<Idx>` and `AsRef<Idx>` so it can key a map that is looked
up by index.

#### `RUID` under the `const` feature

`RUID::new()` becomes a `const fn`, so a `RUID` can be built where a constant is required:

```rust
# #[cfg(all(feature = "ruid_type", feature = "const"))]
# {
use highroller::RUID;

static ID: RUID = RUID::new();

// no index has been taken yet at this point. The first read takes one and keeps it, so
// every later read agrees with the first.
assert_eq!(ID.get(), ID.get());
# }
```

A `const fn` cannot take an index, so the value starts unassigned and takes one on first
read. That needs somewhere to write the result, which costs two things: `RUID` is not
`Copy` and does not implement `Deref`. Use `get()`, and the reference forms of the
operators (`&a + &b`) where a value is needed twice. The lazy assignment is thread-safe, so
a `RUID` shared between threads resolves to one index for all of them.

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>


## License
>You can check out the full license [here](https://github.com/orgrinrt/highroller/blob/main/LICENSE)

This project is licensed under the terms of the **MPL-2.0** license.
