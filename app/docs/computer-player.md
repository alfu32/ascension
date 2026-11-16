# The computer player

How `rom-ai` plays. The design is in `research/05-computer-opponents.md`; this
is what is built. Part 1 (task 60) runs a civilisation: perception, opening
book, economy, construction, research, production and the difficulty limits.
Armies, scouting, space and the application wiring are part 2 (task 61).

## Interface

```rust
let mut brain = rom_ai::Brain::new(rules, PlayerId(2), "balanced", "moderate", match_seed)?;
// every tick, before World::step
let orders = brain.think(&world, world.tick());   // empty unless it is a think tick
for order in orders {
    match world.submit(order.clone()) {
        Ok(_) => brain.accepted(),
        Err(error) => brain.rejected(&order, &error, world.tick()),
    }
}
// or simply: brain.step(&mut world);
```

- A brain reads `&World` through its public queries and returns `Command`s.
  It never holds a `&mut World`; `step` is only the loop above.
- It is told about refused orders, as a person is by the interface. A refused
  subject is left alone for a minute.
- `Brain::save()` gives the columns of `player_ai_state` (profile, difficulty,
  random-generator state, opening position, versioned bytes);
  `Brain::restore` continues exactly as the saved brain would have.
- `rom_ai::handicap_tech(rules, difficulty)` names the technology a host
  grants at setup for a difficulty's income bonus (see "Income bonus").
- Integers only, its own `SimRng` seeded from the match seed and the player
  id, ordered maps everywhere, no clock, no threads.

## One thought

A brain thinks every `ai_difficulty.think_interval_ticks`; players think on
different ticks. One thought:

1. **Perception.** Own entities, queues, population, cities and their radii,
   prices, income rates. Of others only what `World::visible_entities`
   shows (everything with `sees_all`); each sighting is remembered with its
   tick, units fade after two minutes, buildings last as long as the
   simulation's own fog memory. A threat map sums remembered enemy strength
   per 8×8-tile cell. A city that lost hit points or has an armed enemy
   inside its borders raises the alarm; after `reaction_ticks` the opening
   book is abandoned and military wishes move to the front.
2. **Wishes.** Every manager puts forward scored purchases: train a unit,
   research a technology, construct a building.
3. **Serving.** Wishes are served from the highest score down while money and
   the order budget last. A wish worth saving for (the opening's blocking
   step, the way to the next age, the first site of a resource) keeps its
   price aside so cheaper wishes cannot starve it; one such reservation at a
   time. What could not be paid becomes *demand* per resource.
4. **Hands.** Unfinished buildings without a builder get one, idle workers go
   to the resource that lacks hands most, at most one worker per five
   seconds changes resource, idle traders start a route, idle military units
   walk to the rally point beside the capital.

Scores (higher first):

| Score | Wish |
| --- | --- |
| 2000 | current opening step |
| 950 | the building the next age waits for (a library when there is none) |
| 900 / 860 | worker for a free slot (first eight / later) |
| 880 | first gather site of a resource |
| 880 | military unit while a city is under attack |
| 760–810 | further gather sites; unattended sites while the cap has room |
| 560 + 2 × `tech.age_priority` | technologies on the way to the next age |
| 300 + 4 × `econ.expand_eagerness` | another city |
| 440 + `mil.army_share_pct` + deficit | military unit |
| 430 + 2 × `mil.army_share_pct` | first military producer of a kind |
| 560–580 | trader, and the building that trains it, once two cities can trade |
| 300–550 | other technologies, by what they improve |
| 220–520 | other buildings, by what they produce, research or unlock |
| 260 + `mil.defense_share_pct` | towers and forts, `defense_share / 25` per city |

## Managers

- **Opening book** — the profile's `ai_opening_step` rows in order. A step
  holds when the player *has* the count, queued work included. A blocking
  step waits for resources; a step that is impossible for another reason (no
  producer yet, a missing prerequisite, no site) is waited for one minute and
  then skipped. `assign_workers` sets a floor for a resource; `scout` is
  passed over until part 2. The planner runs beside the book with what the
  book leaves.
- **Economy** — the worker target is `econ.worker_share_pct` of the
  population cap. It is handed out one worker at a time to the resource with
  the highest weight per worker: every resource starts equal, unpaid demand
  raises its weight, a full store lowers it. A resource never gets more
  workers than its commerce cap can use, or than its sites hold when no
  further site fits. Sites are built until the slots match; workers are
  trained while free slots (and a spare builder) lack hands. Part of the
  target is held back for worker types whose resource nobody gathers yet
  (scholars before the first university), so the population cap is not full
  of the wrong hands when their time comes — units cannot be disbanded.
- **Construction** — sites come from a spiral of tiles around each city,
  filtered by what the player knows (explored, buildable, required terrain,
  own territory, no known building or its one-tile lane, per-city limits)
  and confirmed with `World::check_placement`. Gather sites that take slots
  from a node terrain go where most of it is in reach. A new city goes
  inside the borders at least the city spacing away, where resources and
  open ground are richest. A failed search is not repeated until a city is
  finished, three more buildings stand, the age advances or a minute passes.
- **Research** — the next age's technology and, through prerequisites,
  everything it waits for come first; one other technology is offered, the
  best by its modifiers (income, population cap when crowded, city limit by
  eagerness, commerce cap when capped), the types it upgrades and what it
  unlocks.
- **Production** — while the army is below `mil.army_share_pct` of the
  population cap, the unit with the best damage per second (damage matrix ×
  armour of the enemy types remembered, or every armour class alike when
  none is known) times staying power per resource is trained; ramping prices
  spread the choice over types. Only units that can walk on the capital's
  ground are trained. Traders are trained up to the route limit once two own
  cities stand a route apart.
- **Difficulty** — think interval; an order budget (`orders_per_minute`,
  with a quarter of a minute in reserve); reaction delay; `mistake_pct` (the
  runner-up technology, unit or wish is taken); `sees_all`.

Nothing above names a definition: types are found by component rows
(harvester, gather site, builder, producer, trader, territory, weapons),
technologies by their effects, modifiers and prerequisites.

## Bounded cost

A thought costs about 1–2 ms of processor time on the i7-6700HQ; the worst
measured in the match tables is 17 ms. Three limits keep it there: at most
four placement previews per thought (each makes the simulation scan every entity), at most
60 000 tile lookups in site searches, and no second site search in a thought
once one came back empty-handed. Prices are computed from the rules and the
brain's own counts instead of `World::price`, which also scans every entity.

## Income bonus

`ai_difficulty.income_bonus_pct` is meant to be an ordinary modifier. A
modifier needs a source in the player's active sources (researched
technologies, age, faction), so the only way without a simulation change is a
technology nobody can research that a host grants at setup.
`rom_ai::handicap_tech` finds it by data — no production row, only
`income.*` percentage modifiers equal to the bonus — and `rom ai-match`
grants it. The seed does not contain such a technology yet: `rom check-rules`
reports an unresearchable technology as unreachable. Until that is decided,
"toughest" plays without its bonus. A test builds the technology into a copy
of the rules and checks the income.

## Headless matches

```sh
rom ai-match --db target/rules.db --seed 1 --players balanced:moderate,builder:tough \
    --ticks 72000 [--universe] [--trace 6000]
```

Players are `profile[:difficulty]` (`moderate` when left out). Without
`--universe` everyone shares one generated map; with it each race starts on
its home planet in a generated universe, as in the application. Every player
gets a city, five builders and 500 of each base resource. Victory rules are
off. Output: one line per player (age, cities, buildings, units, workers,
military, technologies, resources gathered, orders, rejected orders and the
longest run of them, the tick of each age), the rejection reasons, the state
hash; processor time of thinking goes to standard error. `--trace N` prints
every player's economy each N ticks.

## Tests

- `crates/rom-ai/tests/managers.rs` — each manager on a small hand-built world.
- `crates/rom-ai/tests/determinism.rs` — the same match twice; a brain
  restored in mid-match issues the same orders.
- `crates/rom-ai/tests/ages.rs` — the second age in the ordinary run; every
  profile to the third age on seeds 1–5 with
  `cargo test --release -p rom-ai --test ages -- --ignored`.
