# The computer player

How `rom-ai` plays. The design is in `research/05-computer-opponents.md`; this
is what is built. Part 1 (task 60) runs a civilisation: perception, opening
book, economy, construction, research, production and the difficulty limits.
Part 2 (task 61) fights and flies: armies, scouting, siege, formations, the
space manager and the duel harness. The application wiring is task 64.

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
  runner-up technology, unit or wish is taken, and the main army may set off
  for a city before it can take it); `retreats`; `uses_formations`;
  `sees_all`.
- **First gather sites** — while the player has no site for a base resource
  whose first site is paid in that resource (a woodcutter camp costs timber),
  that price is kept back from every other purchase, and the spot the site
  would take is kept free of other buildings in the same thought; the node
  search falls back from its two-tile lattice to every tile when the lattice
  finds nothing. Without these a start could spend its last timber, or give
  its only camp spot to a farm, and never earn timber again.

## Armies (part 2)

Every own fighter (armed, moving, neither worker, builder, trader, carrier,
ship nor orbital craft) belongs to one army; the role is chosen when it is
first seen: the fastest become **scouts** (`scout.units`), then the **home
guard** gets `mil.defense_share_pct` of the fighters, the **raiders**
`mil.raid_share_pct` once ten stand, and the rest is the **main army**. Each
army has a phase — gather, advance, retreat, respond — and remembers its
target, goal, the tick of the last change and its strength when it set off
(all in the saved bytes, layout version 2; version 1 still reads).

- **Strength** is hit points × damage per second of a type, scaled by the
  unit's health; a remembered building counts a quarter (it stands still).
- **Gather.** Idle members walk to the rally point. The main army waits for
  half its share of the army target (at least six, at most sixteen units),
  then needs `mil.attack_ratio_pct` of the known enemy army plus the defence
  around the target, and a target it **can take**: its damage against the
  target's armour (damage matrix) must destroy it before the defence there —
  the target, buildings and units seen lately — has cost it half its hit
  points. Targets are cities, capitals first, valued by price over danger
  and distance; while an enemy territory timer runs, every building that
  holds land is a target and the ratio is waived. With nothing known and no
  scout out, a large main army goes looking itself.
- **Advance.** Attack-move to the target, in formation when the difficulty
  `uses_formations` (the formation row's width to depth and spacing, close
  fighters in front and ranged behind across the line of advance). Members
  already near the goal fight there; reinforcements stream out. A target
  gone, taken or forgotten gives the next one, or home; an advance that has
  not come within reach after four minutes or not taken its target after six
  marks it unreachable for a while.
- **Retreat** (when the difficulty `retreats`): half the army lost against a
  local enemy stronger than `mil.retreat_ratio_pct`, or the target no longer
  within its means; back to the rally point, then gather.
- **Respond.** Armed enemies seen lately inside own territory or a city's
  radius: the home guard attack-moves to the one nearest a city; armies at
  home join when the guard is weaker than the intruders; against more than
  twice everything at home they hold at the city instead. The alarm that
  ends the opening book and moves military wishes to the front now needs a
  hurt city or intruders stronger than the fighters at home.
- **Raiders** go after workers, traders and gather sites whose defence is
  less than half their strength.
- **Scouts** walk to the cell (8 × 8 tiles) nobody has stood in for longest,
  nearest first, on ground they can walk and away from known danger. Every
  own unit marks its cell, fighters the cells around it too.
- **Siege.** A quarter of the main army is trained of the type with the most
  damage per resource against the armour of the city it is after (or the
  most valuable one it knows).
- Memory of enemy units forgets a unit whose last place is in sight and
  empty.

## Space (part 2)

From the first age whose map scale rank is above 0 and in a world with star
systems:

- a building that trains spacecraft is wished for when the player has none;
- **ships** that can claim a presence body (its claim tag) go to the free
  resource bodies in reach (own system; every system on a route once the
  rank allows travel), by income over distance; `space.claim_eagerness`
  decides how many free bodies are worth a ship;
- **warships** (the escort, in the brain's armies) contest enemy-held
  bodies, guard own claims and, three or more, strike the orbit of a planet
  where enemy buildings are known, firing on what is seen below;
- a **transport** loads a builder and some of the main army's troops at an
  own spaceport, flies (and travels) to a habitable planet without an own
  city — free planets first — lands on open ground, founds a city with the
  builder (troops ashore attack the nearest known enemy building), lifts off
  and takes the next planet;
- **orbital craft** (task 58: movement with `climb_ticks`) climb to their
  planet's orbit and defend it; a **launching carrier** (`et_container.launches`)
  waits at home while craft there board it, then joins the escort and
  launches them over its target; craft are trained to fill the carriers.
  Ground batteries are ordinary defensive buildings.

Headless hosts generate a planet when a ship is first ordered into its orbit,
as the application does (`ai_match_cmd::generate_visited`).

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

## Simulation gaps found in part 2

Task 61 did not change `rom-sim`; these are for task 64 or later:

- `explore`, `patrol` and `guard` are accepted but fail as unsupported when
  they run; scouting uses `move` to cells the brain picks itself.
- A stance is set through `World::set_stance`, not an order, so a brain (one
  way in) cannot put raiders on `raid` or the home guard on `defensive`.
- Units cannot garrison in a building (`enter` boards carriers only), so
  wounded units cannot heal in a city that heals (`et_container.heals`).
- A player is not told when an accepted order fails while it runs
  (`no_path`, `no_slot`, a site nobody can reach): a brain sends the same
  futile order again. Seen as workers sent thought after thought to a full
  or unreachable gather site, and builders to a site they cannot reach.
- An own unfinished building cannot be cancelled (`raze` is unsupported):
  a site placed where builders cannot reach blocks its type for good.
- Planets are generated by the host when a ship is ordered into their
  orbit; until then an `orbit` order to them holds forever. `rom ai-match`
  and `rom duel` now do it like the application.
- No path or reachability query: an army whose target is across water finds
  out only by not getting there (four minutes, then the target is left alone).
- Formations are only where the brain sends each unit; the simulation has no
  formation movement (`formation.keeps_slowest`).
- Map generation: on a two-player one-world map the two starts can differ
  twofold in land, and a forest or mountain clump can lie where no gather site
  fits. `rom duel` redraws such maps; a match in the application would not.

## Duels

```sh
rom duel --db target/rules.db --profiles balanced:moderate,balanced:easiest --seeds 1..100 \
    [--ticks-limit 108000] [--victories conquest[,territory]] [--universe] [--side 160] \
    [--jobs 4] [--trace 12000]
```

Two computer players of one race (races meet only in space) and the same
faction, picked by the seed, start on one generated map with the victory rules
on (capital elimination, conquest by default). On odd seeds the second
profile takes the first start. A drawn map whose starts are unfair — one holds
less than three quarters of the other's land, or a capital has no room for a
gather site on a node terrain its start sites draw from — is replaced by
another map of the same seed (`maps=` in the output, at most sixteen). Each
match runs until a team wins or the tick limit; output is one line per seed
(players, winner, rule, tick, hash) and the win-rate table; matches run on
`--jobs` threads (default: up to four), each deterministic on its own.

Territory is off by default: on the two-player map the first city of a
central start already holds half the land, so the territory timer decides
the match before an army can.

## Tests

- `crates/rom-ai/tests/managers.rs` — each manager on a small hand-built world.
- `crates/rom-ai/tests/determinism.rs` — the same match twice; a brain
  restored in mid-match, and one restored in mid-war, issues the same orders.
- `crates/rom-ai/tests/military.rs` — taking a known city, waiting for the
  attack ratio, retreating only where the difficulty allows, the home guard,
  scouting, formations.
- `crates/rom-ai/tests/space.rs` — claiming resource bodies and founding a
  colony with a transport in a small universe, craft climbing to orbit and
  boarding a carrier, the space-age brain twice.
- `crates/rom-cli/src/duel_cmd.rs` — a short duel twice with equal hashes.
- `crates/rom-ai/tests/ages.rs` — the second age in the ordinary run; every
  profile to the third age on seeds 1–5 with
  `cargo test --release -p rom-ai --test ages -- --ignored`.
