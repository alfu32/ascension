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
- **First gather sites** (changed in part 2) — while the player has no site
  for a base resource, the price of the cheapest site it can build is kept
  back from every other purchase (unless a search for it has just failed),
  and the spot the site would take is kept free of other buildings in the
  same thought. Node sites are searched only at spots near tiles of their
  node terrain, found once per city. Without these a start could spend its
  last timber, give its only camp spot to a farm, or — once it had several
  cities — spend its lookup budget far from any mountain and never build a
  mine.

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
  not come within reach after four minutes, or not reached its target after
  six, marks it unreachable for a while. At a neutralised city the army goes
  for whatever enemy still stands in its radius (a worker inside a gather site
  keeps a city from being taken), and sweeps the radius when nothing is known.
- **Focus fire.** Members in a fight turn on the weakest armed enemy seen
  within their reach. Every such order comes out of the order budget, so a
  difficulty with few orders a minute leaves its units to pick their own.
- **Retreat** (when the difficulty `retreats`): half the army lost against a
  local enemy stronger than `mil.retreat_ratio_pct`, or the target no longer
  within its means; back to the rally point, then gather; the target is left
  alone for two minutes.
- **Mistakes.** `mistake_pct` of the thoughts in a minute of ticks send the
  main army at a city before it can take it (on top of the runner-up choices
  of part 1).
- **Respond.** Armed enemies seen lately inside own territory or a city's
  radius, for longer than the difficulty's `reaction_ticks`: the home guard attack-moves to the one nearest a city; armies at
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
four placement previews per thought (each scanned every entity until task 64), at most
60 000 tile lookups in site searches, and no second site search in a thought
once one came back empty-handed. Prices are computed from the rules and the
brain's own counts instead of `World::price`, which also scans every entity.

## Income bonus

`ai_difficulty.income_bonus_pct` is an ordinary modifier carried by a
technology of kind `handicap` that the difficulty names in `handicap_tech_id`
(the seed: `tech.handicap.income_25` on "toughest"). The host grants it at setup
with `World::grant_handicap` before the first tick; the validator keeps the
handicap's `income.*` modifiers equal to the bonus (`docs/database.md`,
"Handicaps"). `rom_ai::handicap_tech` still finds the same technology by its
modifiers, so `rom ai-match` grants it as before.

## In the application (task 64)

Every player other than the person (player one) and nature is played by a
brain (`crates/rom-app/src/computer.rs`). `--ai profile:difficulty,…` chooses
per computer slot (players 2, 3, … in order; a missing slot plays
`balanced:moderate`), `--ai none` leaves them idle; the launcher has the same
choice per slot ("Choose per slot…" beside `--ai`). The brains live on the
simulation thread: before every `World::step` each brain is asked for orders
(it thinks only on its own ticks) and they go through `World::submit` like the
person's, the brain hearing which were refused. With `--demo`, the demo driver
only moves the person's units. Think time shows in the `--stats` overlay and in
`--bench` output (`bench-detail computers=… think_ms_avg=… think_ms_max=…`).

Saves: `computer::save_match` / `load_match` put `Brain::save()` into
`player_ai_state` through `rom_save::save(…, ai_states)` and restore the brains
on load; a loaded match continues with the same orders and hashes (test
`a_save_with_brains_loads_and_continues_alike`).

### Simulation queries a brain can use now

- `World::price`, `World::check_placement`, `World::owned_types`,
  `World::population_used` no longer scan every entity: entities are indexed by
  owner and type, queued constructions live only in the pending orders, and
  overlap uses the per-tile building index. `World::type_counts(player)` gives
  what a player has of each type (sites and ships in transit included).
- `World::city_count`, `city_limit`, `city_covering`, `is_city_type`,
  `active_sources`, `pending_orders` (submitted, applied at the next step) and
  `rally(producer)` are public.
- Orders: `set_rally` (producer, point or entity; its own entity clears it) and
  `disband` (any unit without passengers aboard).

Task 65 adopted these in `rom-ai`: military producers get one `set_rally` each
to the rally point; idle workers with nothing to do are disbanded while the
population is full and the workers exceed their target. Counting still uses the
brain's own view (it already includes queued work).

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

## Strength and difficulty (task 65)

What changed in the brain, and why (each found in traced duels):

- **First mines.** Most stalls were a player with no metal income for the
  whole match: the only spot beside its rock had been built over before mines
  were known. While a node resource (metal, timber) has no gather site yet,
  other buildings keep four tiles off its node terrain around each city
  (`Ground::margin`). A site search cut short by the lookup allowance is tried
  again later instead of being given up for the situation.
- **Mistakes never spend savings.** The runner-up slip is not applied to a
  wish that reserves money; before, a careless mind spent the food it saved for
  the next age thought after thought and never left the first age.
- **The way to the next age gains priority** after four minutes in an age (20
  a minute, at most 400), so a long war cannot keep a player in one age.
- **Unit choice.** Counters are chosen against enemy *units* (buildings made
  every army a siege train that could not clear a city), and prices are
  weighed by how scarce each resource is to the player (`production::dearness`):
  a metal-starved player trains what it can pay for.
- **Cities** are wished for at 450 + 4 × `econ.expand_eagerness` (saved for
  from eagerness 50), and `city_limit` technologies count three times when every
  allowed city stands: food comes from farms, and farms from cities.
- **Difficulty in the economy**, from the existing `ai_difficulty` columns:
  demand and worker moves are paced at twice `reaction_ticks` (at least 100
  ticks); a thought that slips (`mistake_pct`) notices only resources without
  any gather site; workers are `worker_share × (200 − mistake_pct) / 200` and
  soldiers `army_share × (100 + 2 × mistake_pct) / 100` of the profile's shares
  (a careless mind over-arms: soldiers that fall in skirmishes instead of the
  growth and research that win wars — measured, an under-armed easiest beat
  moderate). Thinking speed, order budget and mistakes in research and wishes
  stay as before.
- **Struck-down cities.** The army's focus fire picks only targets its units
  can hit; workers leave sites of an own city that was struck down.

Memory layout version 3 adds the age clock (age and the tick it began);
versions 1 and 2 still read.

## Simulation gaps found in part 2, and what task 65 did with them

- **Closed: `explore`, `patrol`, `guard`.** `explore` walks to the nearest tile
  its player has never explored that its movement class can reach over the
  terrain, picks the next once that one is seen, and is done when none is left
  within 96 tiles (a map whose terrain is revealed at setup has none: the
  scouts' own staleness walk stays for that). `patrol` walks between where it
  began and its point and back, fighting like `attack_move`. `guard` stays
  beside an own or allied entity (at the footprint edge of a building) and
  fights off enemies it can see within 8 tiles of it; the enemy is kept in
  `order.engaged`. The home guard of a brain guards the capital in its gather
  phase, so a slow mind's guard fights without orders.
- **Closed: stance as an order.** `stance` (subject = a combat stance) applies
  when it leaves `pending` and keeps the queue. Raiders take `raid`, the home
  guard `defensive`.
- **Closed: failure report.** `World::order_failures(player)` lists the latest
  64 accepted orders that failed while running (tick, order, executor, kind,
  subject, targets, reason); saved, not hashed. A brain reads what failed since
  its last thought: a gather site that had no slot or no path is not offered
  for a minute, a building whose site could not be reached is not wished for,
  an attack target with no path is left alone for two minutes.
- **Closed: cancel an own unfinished building.** `raze` on an own building: an
  unfinished one goes and `economy.cancel_refund_pct` of its base price, for the
  work not yet done, comes back; a finished one that cannot be captured is
  pulled down; a finished city is refused.
- **Closed: reachability.** `World::reachable(type, from, to)` answers over the
  terrain alone (buildings come and go) from per-class landmass labels made when
  first asked for (derived from the map, not state). The army's target choice
  skips targets its units could never walk to.
- **Closed: formation movement at the slowest speed.** `move`, `attack_move`
  and `patrol` may name a formation as subject; the orders one player gives in
  one tick with one kind and formation walk at the slowest member's speed
  (`order.pace`) when the formation `keeps_slowest`. A brain that
  `uses_formations` names its line formation when the army advances.
- **Closed: start fairness in map generation** (see "Fair starts" below).
- **Left out: garrisoning to heal** (optional in the card): `enter` still boards
  carriers only.
- **Left out: planets generated by the simulation** (optional): hosts still
  expand a planet when a ship is first ordered into its orbit.

Two more simulation changes came out of the duels:

- A walker pushed onto ground its class cannot enter (a citizen standing on a
  coast tile) counted as standing on that ground's layer, so no land weapon
  could hit it, and it kept a struck-down city contested for the rest of the
  match. It now counts as on the ground layer. `World::can_target(attacker,
  target)` tells whether any weapon of an attacker can aim at a target, and the
  brain's focus fire only picks targets its units can hit.
- Idle workers no longer go to work by themselves in gather sites of a city the
  enemy has struck down (hit points 0); the brain also calls its workers out of
  such sites (`release`). Workers there kept the city contested.

## Fair starts (task 65)

`rom_mapgen::generate_fair` (flat maps; `rom ai-match` and `rom duel` use it) and
`generate_sphere` place the starts of players who share a planet so that each
holds an even share of the land: of 48 sets of starts (the farthest-point
construction on even attempts, a random candidate at least three quarters as
far on odd ones), the set with every start on one landmass, the most even
shares of it (tiles nearer to one start than to the others; shares within 95 %
of each other count as even) and then the widest spread is taken. On a sphere
the same is weighed with the land tiles nearest each start. A per-player
feature (forest, mountain clumps) is rolled once for the first player and every
other start gets the same clumps of the same sizes in the same distance band.
One player per planet draws exactly as before, so universes whose home
planets each hold one race keep their hashes. Plain `generate` keeps the old
flat maps: the application's default map and the screenshot scripts written for
it (task 30, task 32) depend on them, and `rom-app` was outside this task;
switching the application to `generate_fair` means re-authoring those scripts.
`GeneratedMap::start_land_shares` measures the shares (test
`fair_maps_give_two_players_even_land_and_the_same_clumps`: of 30 two-player
seeds at least 27 fair maps hold shares within 85 % of each other, more than
plain `generate`). `rom duel` still checks each map and
redraws an unfair one, which now happens rarely (`maps=` in the output).

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
