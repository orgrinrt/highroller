use std::sync::{Arc, Mutex};
use std::thread;

// the index width is a compile-time choice, so the id type follows whichever size flag is set
#[derive(Clone)]
struct Fighter<I> {
  id: I,
  power: u32,
}

// create a register for fighters
let fighters_register = Arc::new(Mutex::new(Vec::new()));

// create four threads as four different arenas
let arenas = 4;

// gather 20 fighters for each arena
let mut handlers = Vec::new();
for _ in 0..arenas {
  let fighters_register = Arc::clone(&fighters_register);
  handlers.push(thread::spawn(move || {
    let mut ids = Vec::new();
    for n in 0..20u32 {
      let id = highroller::rolling_idx();
      let fighter = Fighter {
        id,
        power: (n * 37 + 11) % 100, // stand-in for a real power stat
      };
      ids.push(fighter.id);
      fighters_register.lock().unwrap().push(fighter);
    }
    ids
  }));
}

// run a simple tournament that finds a champion for each arena
let mut champions = Vec::with_capacity(arenas);
for handler in handlers {
  let arena_fighters = handler.join().unwrap();
  let fighters = fighters_register.lock().unwrap();

  // find the fighter with the highest power in each arena
  let champion = arena_fighters.iter()
    .map(|&id| fighters.iter().find(|fighter| fighter.id == id).unwrap())
    .max_by_key(|fighter| fighter.power)
    .unwrap()
    .clone();

  champions.push(champion);
}

// match the arena champions against each other for the ultimate champion
let ultimate_champion = champions.into_iter()
  .max_by_key(|fighter| fighter.power)
  .unwrap();

// print the winner by id
println!("The ultimate champion is fighter with id: {}", ultimate_champion.id);
