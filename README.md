highroller
============
<div style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/highroller.svg)](https://github.com/orgrinrt/highroller/stargazers) 
[![Crates.io Total Downloads](https://img.shields.io/crates/d/highroller)](https://crates.io/crates/highroller)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/highroller.svg)](https://github.com/orgrinrt/highroller/issues) 
[![Current Version](https://img.shields.io/badge/version-0.1.0-orange.svg)](https://github.com/orgrinrt/highroller) 

>A simple, high-level rolling index that is thread-safe and guarantees cheap runtime-unique IDs.

</div>

# Usage

This Rust crate provides a statically available, thread-safe rolling index. Intended for 
simple use cases where UUIDs would be overkill and a cheap alternative is preferable.

The main function provided is `rolling_idx()`. Simplistically:

```rust
let id1 = highroller::rolling_idx();
let id2 = highroller::rolling_idx();
println!("Id 1 is: {}", id1);
println!("Id 2 is: {}", id2);
// outputs:
// Id 1 is: 0
// Id 2 is: 1
```

The function `rolling_idx()` returns the current index value. After retrieving, it increments the index by 1. This way, you get a unique, ever-increasing rolling index each time you call this function.

> Please note that the rolling index is runtime-specific and is reset every time your application starts. 
 
The rolling index is also thread-safe, meaning you can access it from multiple threads simultaneously without 
encountering issues related to concurrent data access*.

<small>*Pending more robust testing, should hold true. </small>

### Feature Flags

`highroller` provides several feature flags for flexibility.

| Feature Flag          | Default    | Description                                                                                                                                                             |
|-----------------------|------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `strict`              | *Enabled*  | Panics on overflow and disables `RUID` - numeric type comparisons and <br/>arithmetics. *(When disabled, overflow will wrap instead)*                                   |
| `ruid_type`           | *Disabled* | Enables Rolling Unique ID (`RUID`) type, a wrapper over the rolling index                                                                                               |
| `allow_arithmetics`   | *Disabled* | Optional support for arithmetic operations on `RUID` *(note that without `strict` enabled, you should know what you are doing, since it can cause ambiguous behaviour)* |
| size (separate flags) | `u16_index` | Choose the size of the rolling index: `u8_index`, `u16_index`, `u32_index`, `u64_index`, `u128_index`, `usize_index`                                                    |

If you need a particular rolling index size, or if you want to implement more explicit typing with `RUID`, enable the features according to your use case. The `strict` feature will help you catch overflows, where as the `allow_arithmetics` flag expands `RUID` functionality to support arithmetic operations.

### RUID
"Rolling Unique ID" (RUID) is essentially a wrapper over the rolling index, with optional support for arithmetic 
operations, and complete equivalence relation methods and display methods. You can use the `ruid_type` feature flag 
to enable `RUID` and use it in your program. [Read more about `RUID` at the "Extras" section](#extras).


## Example

Consider a basic game where you summon digital fighters. Each summoned fighter needs to have a unique identifier. 
Creating a complex UUID for each fighter could eat up valuable resources and cause performance issues in your game.

That's where you can take advantage of `highroller` to assign unique identifiers. It is simple and efficient:

```rust
use std::sync::{Arc, Mutex};
use std::thread;
use rand::Rng;

struct Fighter {
  id: u8,
  power: u32,
};

// create a register for fighters
let fighters_register = Arc::new(Mutex::new(Vec::new()));

// create four threads as four different arenas
let arenas = 4;

// first, gather a randomized set of 20 fighters (power randomized) for each arena
let mut handlers = Vec::new();
for _ in 0..arenas {
  let fighters_register = Arc::clone(&fighters_register);
  handlers.push(thread::spawn(move || {
    let mut rng = rand::thread_rng();
    let mut ids = Vec::new();
    for _ in 0..20 {
      let fighter = Fighter {
        id: highroller::rolling_idx(),
        power: rng.gen_range(1, 100),
      };
      ids.push(fighter.id);
      fighters_register.lock().unwrap().push(fighter);
    }
    ids
  }));
}

// do a simple tournament that reveals a champion for each arena
let mut champions: Vec<Fighter> = Vec::with_capacity(arenas);
for handler in handlers {
  let arena_fighters = handler.join().unwrap();
  let fighters = fighters_register.lock().unwrap();
  
  // Find the fighter with the highest power in each arena
  let champion = arena_fighters.iter()
    .map(|&id| fighters.iter().find(|fighter| fighter.id == id).unwrap())
    .max_by_key(|fighter| fighter.power)
    .unwrap()
    .clone();
  
  champions.push(champion);
}

// now match the top against each other in a playoff that is based 
// on the power we randomized earlier
let ultimate_champion = champions.into_iter()
  .max_by_key(|fighter| fighter.power)
  .unwrap();

// print the winner by id
println!("The ultimate champion is fighter with id: {}", ultimate_champion.id);
```


In this example, each fighter we create gets a unique identifier from `highroller::rolling_idx()`. Since the rolling 
index is incremental, each fighter gets a unique ID. This happens without any complex UUID or similar overhead, and 
is practical to use.

Please remember that the index resets every time your application restarts. If you need persistence across 
application restarts, you will have to implement additional strategies.

## The problem

At times, you need a very simple guaranteed unique identifier for something. Using UUIDs can be overkill
and bring forward resource costs you likely don't need to afford, if your use case is very simple and not
very extensive.

In comes a static rolling counter.

The concept is simple: Have a statically available rolling value, that automatically increments
itself after each fetch. Add in some thread-safety measures and you have a very easy-to-use and
practical guaranteedly unique identifier.

Mainly useful for simple identification needs, it avoids a lot of complexities, and can be
very cheap to run.

## Extras

With the `ruid_type` feature flag, you're able to leverage `RUID`s, or Rolling Unique Identifiers, as custom integer 
types. You can convert a `RUID` to an standard integer or vice versa, compare two `RUID`s, manipulate `RUID`s with arithmetic operations if `allow_arithmetics` flag is on, and print `RUID`s as they implement the `fmt::Display` trait.

Example usage of `RUID`:

```rust
use highroller::RUID;

let id1 = RUID::new();
let id2 = RUID::new();

assert_ne!(id1, id2);
```

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>


## License
>You can check out the full license [here](https://github.com/orgrinrt/highroller/blob/master/LICENSE)

This project is licensed under the terms of the **MIT** license.
