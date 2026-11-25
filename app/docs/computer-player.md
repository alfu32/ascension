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
| 1100 | a gather site for a resource with fewer than half the slots its workers need, paid in that resource (task 78) |
| 950 | the building the next age waits for (a library when there is none) |
| 900 / 860 | worker for a free slot (first eight / later) |
| 880 | first gather site of a resource |
| 880 | military unit while a city is under attack |
| 850 | another city while a resource has no room for another site around the cities (task 78) |
| 760–810 | further gather sites; unattended sites while the cap has room |
| 560 + 2 × `tech.age_priority` | technologies on the way to the next age |
| 300 + 4 × `econ.expand_eagerness` | another city |
| 440 + `mil.army_share_pct` + deficit | military unit |
| 430 + 2 × `mil.army_share_pct` | first military producer of a kind |
| 560–580 | trader, and the building that trains it, once two cities can trade |
| 880 | a site that earns energy by itself while the plans spend more energy than comes in (task 152) |
| 600 | such a site while a city has none (task 152) |
| 300–550 | other technologies, by what they improve |
| 220–520 | other buildings, by what they produce, research or unlock |
| 260 + `mil.defense_share_pct` | towers and forts, `defense_share / 25` per city |
| as above, at least 300 | a lost building put back; a lost producer as the best wish for what it makes (task 152) |

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
  `uses_formations`: every member is sent to the one point naming a formation
  chosen by its row, and the simulation lays the group out (since task 153; see
  "Armies and fleets in formation" below). Members already near the goal fight
  there; reinforcements stream out. A target
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
  the warship trained is the class (task 92) that gives the best exchange
  for its price against the enemy ships and craft seen, or, before any is
  seen, against one of every armed ship and craft the player could train;
- a **transport** loads a builder and some of the main army's troops at an
  own spaceport, flies (and travels) to a habitable planet without an own
  city — free planets first — lands on open ground, founds a city with the
  builder (troops ashore attack the nearest known enemy building), lifts off
  and takes the next planet;
- **orbital craft** (task 58: movement with `climb_ticks`) climb to their
  planet's orbit and defend it; a **launching carrier** (`et_container.launches`)
  waits at home while craft there board it, then joins the escort and
  launches them over its target; craft are trained to fill the carriers.
  Ground batteries are ordinary defensive buildings, and more than that on a
  planet the player sees bombarded (task 75, below).

Headless hosts generate a planet when a ship is first ordered into its orbit,
as the application does (`ai_match_cmd::generate_visited`).

Nothing above names a definition: types are found by component rows
(harvester, gather site, builder, producer, trader, territory, weapons),
technologies by their effects, modifiers and prerequisites.

## At sea (task 97)

Warships are the units that fight and move on water and on no ground (`Catalogue::sea_units`);
they are never enlisted in an army on land, never count as intruders for the home guard, and
the producers that train only them get no rally point on land. The naval manager (`navy.rs`)
leads them as the army `Navy`.

- **When.** The difficulty's rank is at least `sea.min_difficulty_rank` (default 1: the
  easiest level stays ashore) and the player has water near its capital (beside a dock, or
  within the capital's border), and an enemy can be reached from it by water: enemy warships
  remembered on the same sea, or an enemy building with water the navy can sail to within a
  warship's reach of it (a shore target) whose defences a warship the player can train
  outranges at the ground (task 153: before, a shore target alone called for a navy whose
  ships could not shell it, and they waited at the harbour all match). The dock itself is the construction manager's: it
  wishes for every building that trains fighters when the player has none.
- **How many.** `sea.navy_share_pct` (default 20) of the military target, or the enemy fleet
  seen times `mil.attack_ratio_pct` when that is more (at most twice the share: an arms race
  at sea leaves no army to win on land). The land army's target is the rest.
- **Which class.** The warship with the best exchange against what it has seen: the enemy
  warships and the aircraft that strike ships (before any is seen, one of every warship it
  could train), and once the enemy fleet is beaten (a squadron of three, stronger than the
  fleet seen by the attack ratio) the shore targets. The exchange is the production manager's
  (damage dealt times staying power over damage suffered, at the water layer), divided by the
  price twice: ships fight at range in numbers, where a fleet's strength grows with the square
  of its count. A building a ship outranges cannot hurt it. Against line ships this picks the
  escort, against a coast with towers and forts the line ship.
- **Missions.** Enemy warships off the own coast (within a city's border and eight tiles) are
  met by every sound ship. A squadron of three stronger than the enemy fleet by the attack
  ratio hunts it. With the enemy fleet beaten, the ships that outrange the defences around the
  nearest shore target (towers, forts and cities that fire at ships first, then cities, then
  any building) sail to water within their reach of it and shell it, at least two of them;
  the others keep the harbour. Nothing is hunted or shelled during the difficulty's grace
  period. A navy that has lost strength below `mil.retreat_ratio_pct` of its strength when it
  set off pulls back home for a minute (where the difficulty `retreats`).
- **Repair.** A ship below 40 % of its hit points goes home to the harbour. When the player
  can train a unit that heals ships and reaches them on the water (the repair drone), one is
  kept and mends the most damaged ship at home; a ship there stays until 85 %. Before such a
  unit, nothing mends a ship: a damaged ship keeps the harbour. Only such a unit is sent to
  mend: supply carts, which mend vehicles since task 140, cannot reach ships and are left with
  the army.

### Measured (task 97)

The duel map (`rom duel` without `--universe`: the fair one-world map, 160 tiles) is one
continent ringed by sea, so both starts are on the coast in most seeds; no new option was
needed. `sea_duel_report` plays seeds 1–10 of it with two `balanced:moderate` players and
conquest on (player one always the first side; the map is not redrawn for fairness as
`rom duel` does): 183 and 177 warships trained, 83 and 118 sunk, 26 and 19 enemy buildings
shelled from the sea, of which 22 and 12 fell; 7–2, one draw.

| Duel, 100 seeds | Before (0.0.87) | With the navy |
| --- | --- | --- |
| `balanced:moderate` vs `balanced:easiest` | 97–0, 3 draws, mean tick 46910 | 95–0, 5 draws, mean tick 52532 |
| `balanced:moderate` mirror | 51–42, 7 draws, mean tick 64178 | 50–41, 9 draws, mean tick 67173 |

The easiest level stays ashore, so against it the moderate player's navy is land army it
does not have: its wins come about five game minutes later on average.

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

Measured on the i7-6700HQ (`rom duel`, 100 seeds, 8 jobs, 108 000-tick limit):

| Duel | Wins | Draws | Player 1 / 2 wins | Run time |
| --- | --- | --- | --- | --- |
| task 61: moderate – easiest | 27 – 16 | 57 | 36 / 7 | 34.6 min (4 jobs) |
| task 65: moderate – easiest | 13 – 7 | 80 | 9 / 11 | 35.6 min |
| task 65: moderate – moderate | 7 – 7 | 86 | 9 / 5 | 33.3 min |

**The target (90 wins, no draws) is not met.** The starts are now even (98 of
100 maps fair at the first draw; the first player no longer wins 36 of 43), the
stalls the traces showed are gone (no metal, siege-only armies, untouchable
wading citizens, a careless mind spending its age savings), and moderate beats
easiest, but most matches end in a stand-off: with even land both economies run
into the same commerce caps and farm limits, armies stay at 20–60 units, and an
attack on a capital meets the city's guns, towers and dozens of armed citizens
inside its radius; nobody can clear it to capture it. What would end them is
for task 66: an army that grows past the defence before it marches (it is
capped by `mil.army_share_pct` and food), a capture rule that unarmed-for-war
citizens do not hold, or a bigger skill gap between the difficulties than the
existing `ai_difficulty` columns give.

Solo (`rom ai-match --seed 1 --players balanced:<difficulty> --ticks 72000`):

| Difficulty | Age at 72 000 | Cities | Workers | Techs | Gathered |
| --- | --- | --- | --- | --- | --- |
| easiest | 4 | 6 | 83 | 43 | 56 884 |
| moderate | 4 | 6 | 94 | 45 | 74 469 |
| toughest | 6 | 6 | 101 | 66 | 93 192 |

Memory layout version 3 adds the age clock (age and the tick it began);
versions 1 and 2 still read.

## Capture that can be won, and armies that win it (task 67)

The capture rule changed (`docs/database.md`, "Capture and difficulty"): workers no
longer hold a struck-down city, and a city held at 0 hit points by attackers for
`city.capture_hold_ticks` falls even with defenders left. Traced duels then showed why
armies still did not finish a match, one cause after another:

- **Attrition.** Enemy land takes 60 hit points a period from every unit (down to 1)
  except beside its own supply. A siege of a few minutes left the whole wave at 1 hit
  point and any arrow killed it. The main army now takes **supply units**
  (`et_supply`, by component row): one per eight members is wished for at 720 (ahead of
  more soldiers), a gathered army waits up to three minutes for three of them at the
  rally point, and while it advances they walk beside members spread along the body of
  the army (two orders a thought at most).
- **Waves that never left.** A unit sent to a point inside a building has no path; the
  army now aims at the edge of the target's footprint on its own side. A rally point
  in the middle of the town got boxed in by buildings: it now lies just beyond the
  capital's economic radius inside own borders, a 7 × 7 square around it is kept free
  of buildings, and when six units fail to find a path from one place within a minute
  the nearest own building other than a city there is pulled down (`raze`) and, if the
  place is the rally point, the rally point moves to the next direction (never a gather
  site is pulled down: an early version razed a start's only woodcutter's camp). An
  advance with no target (a search for the enemy) is given up after four minutes.
- **Waves that trickled.** Units that join after a wave set off wait at the rally point
  for the next one (during the first minute every member belongs to the wave), and the
  armies take their orders before the wishes while one is on the march (two thirds of
  the order budget), so a wave is not sent off a few units a thought.
- **Decisive waves.** A wave sets off only when it exceeds `mil.attack_ratio_pct` of
  the defence measured at its target: the strongest defence seen around that city
  (units seen lately, every remembered building, armed ones at full strength), and the
  strength of every wave that lost half of itself there; a measure counts half after
  20 game minutes. After 40 minutes of a match the ratio a wave needs falls by two
  points a minute, to 80 % at least: equal armies would otherwise wait for each other
  until the end. A wave keeps fighting while it has two thirds of its strength, and a
  city it holds at 0 hit points is held, not given up. A blunder (`mistake_pct`) lowers
  the ratio to 100 % instead of ignoring it. A captured enemy capital is held by the
  main army until its owner falls; an own city struck down calls every army home.
- **Knowing where to go.** With no enemy city known the main army goes looking even
  when a scout is out; scouts judge danger by enemy units only (walls and towers can be
  walked around), and a place no path leads to counts as looked at. Targets are judged
  reachable when most of the army's kinds can walk there.
- **Spending surplus.** Two technologies besides the way to the next age are wished
  for each thought, more when every earned resource has 1000 or more in store;
  upgrades of fielded military types are valued by how many are fielded; a research
  queues behind training at a busy producer (the building that trains a line also
  researches its upgrades, and never stood idle). With a surplus the army target grows
  to the room the population cap leaves. An army below half its target is wished for
  at 700. A resource with fewer than half the slots its workers need saves for its next
  site.
- **Difficulty that decides.** New `ai_difficulty` columns (`grace_ticks`,
  `first_wave_max`, `wave_growth_per_hour`, `army_vs_enemy_pct`, `economy_pct`; values
  in `database.md`). `easiest` leaves an opponent's town alone for thirty minutes, sends at
  most five units in its first wave (ten more per hour), keeps its army within 40 % of
  the largest enemy army it has seen and keeps half its profile's workers; `easy`
  ten minutes, eight units, 80 %, 80 %. Scouts hold their fire during the grace period:
  they look, and strike no town. The brain does not know which player is a
  person, so this holds against every opponent.
- **Fair duel starts** (`rom duel`): besides land, the best gather site each start can
  place on each node terrain must work at least 60 % of the node tiles of the other's
  (a start whose only woodcutter's camp reached one tree stalled for half an hour).

Measured on the i7-6700HQ (`rom duel`, 100 seeds, 8 jobs, 108 000-tick limit):

| Duel | Wins | Draws | Player 1 / 2 wins | Run time |
| --- | --- | --- | --- | --- |
| 0.0.63: moderate – easiest | 29 – 9 | 62 | 22 / 16 | 26.1 min (4 jobs) |
| task 67: moderate – easiest | 85 – 3 | 12 | 44 / 44 | 4.6 min |
| task 67: moderate – moderate | 37 – 36 | 27 | 33 / 40 | 6.8 min |

**The targets (90 wins and at most 5 draws; at most 10 draws in the mirror) are not
met.** What is left: armies of equal strength cannot break a defended city (cities,
towers and forts fire, citizens fight, the hold takes two minutes), so mirrors end
level; some starts still stall in their first age; a few armies get boxed in faster
than they are freed.

Task 68 adopted in `rom-ai`: warships over an enemy world with nothing in sight move
within the orbit to the nearest enemy building remembered there (ships hold still in
orbit); warships with nothing to do patrol the home orbit (a great circle through the
capital); no orbital strike during the grace period. The colony transport keeps task
68's stop-gaps deliberately: it lands near a home city before troops board (boarding is
refused in orbit) and lifts off once loaded; a builder busy constructing may be taken
when no idle one is near. Traders that fly (the space freighter: a spacecraft with
`et_trader.runs_routes`) are trained when own cities stand on two planets of one star
system, up to half the route limit, and run a route to an own city on another planet
of their system; walking traders keep routes on one planet (before, the best trader by
income was taken for every route, so a freighter would have been trained for a route
it cannot run). A satellite (the ship that sees farthest and neither claims, carries nor
trades, unarmed or bound to its orbit: armed since task 134, it is still the watcher and never
a warship or an army unit) is wished for over every planet with a spaceport. Cities, towers and
forts that fire count at full strength in the measured defence.

## Every strength has an answer; armies that break a city (task 71)

### The owner's rule and `rom fight-matrix`

> "Even if one unit outranges and outbeats the direct target type (like cannons vs
> buildings), its countermeasure should either be effective or be one upgrade away of being
> effective." (owner, 2026-11-16)

```sh
rom fight-matrix --db target/rules.db [--ages 0..13] [--seeds 3] [--jobs 4] [--all]
rom fight-matrix --db target/rules.db --ages 1 --fight unit.catapult,bld.city   # one pairing
```

Everything is read from the rules (`crates/rom-cli/src/fight_matrix_cmd.rs`):

- **The fighters of an age**: armed types the faction can make whose first age is that one
  or earlier, the newest of each upgrade line; units that only fight (no worker, builder,
  trader or carrier) and buildings with a weapon; only what stands on plain ground or flies
  (ships and orbital craft are left out: their fights need water or an orbit).
- **Outranges**: the reach of A's weapons at B's layer, plus B's half footprint (weapons
  measure to a building's edge), is more than B's reach at A's layer plus A's; a B with no
  weapon for A's layer is outranged by any A that can hit it.
- **A fight**: both sides spend three times the dearer side's price (a type's `cost`, or for
  a building upgraded in place the price of what it is made from), at most 24 of a type,
  on a 112 × 64 plain map, the sides 6 tiles beyond both reaches apart; every unit
  attack-moves to the other side's middle (sent on again when idle, as a player would),
  buildings fire on their own; both sides see the whole field (an unarmed observer of the
  farthest-seeing type stands far off each side: a real army brings scouts, and siege sees
  little by itself); no victory rules. It ends when one side has nothing armed left, or
  after five game minutes. Each side's result is the share of its starting value it still
  holds (price × hit points left; a captured city is lost).
- **The measure (fixed)**: a side wins a seed when it holds a larger share than the other;
  it wins the pairing when it wins most of three seeds.
- **A strength**: A outranges B and wins the pairing against B. **Its answer**: a mobile
  fighter C of the same age (a sally) that wins the pairing against A, with no research
  beyond the age's units; failing that, with one research of the age (a technology whose
  age requirement is met, not an age advance, project or handicap, with an owner modifier
  on attack, range, hit points, armour or speed that applies to C).
- The command prints one row per strength and the roster of each age, and fails when a
  strength has no answer; the test `every_strength_has_an_answer` runs the whole matrix.
- **At sea and between ships (task 92).** Sea rows (`sea_matrix.rs`): warships (units that
  cross open water and not the shore, the newest of each class) fight on a field whose halves
  are each side's ground, water beside the plain shore; a row is a strength a warship takes
  part in, answered by any mobile type of the age or, for a warship, a building ashore. Ship
  against ship rows (`orbit_matrix::measure_space_age`): the armed ships and craft of an age in
  one orbit. Tests `every_sea_strength_has_an_answer` and
  `every_ship_against_ship_strength_has_an_answer`; `--no-sea`, `--sea-fight a,b`.

### The matrix (task 71 rules, 3 seeds)

| Age | Strength | Outranges and beats | Answer | Research | Answer's fights (won, value left) |
| --- | --- | --- | --- | --- | --- |
| age.ancient | city | slinger | spearman | - | 3/3 (61‰ left vs 0‰) |
| age.ancient | slinger | scout | spearman | - | 3/3 (488‰ left vs 0‰) |
| age.classical | city | archer | catapult | - | 3/3 (683‰ left vs 0‰) |
| age.classical | tower | hoplite | catapult | - | 3/3 (1000‰ left vs 0‰) |
| age.classical | archer | scout | catapult | - | 3/3 (890‰ left vs 0‰) |
| age.classical | catapult | city | scout | - | 3/3 (536‰ left vs 0‰) |
| age.medieval | city | crossbowman | trebuchet | - | 3/3 (679‰ left vs 0‰) |
| age.medieval | tower | pikeman | trebuchet | - | 3/3 (1000‰ left vs 0‰) |
| age.medieval | fort | pikeman | trebuchet | - | 3/3 (865‰ left vs 0‰) |
| age.medieval | crossbowman | scout | trebuchet | - | 3/3 (777‰ left vs 0‰) |
| age.medieval | trebuchet | city | scout | - | 3/3 (547‰ left vs 0‰) |
| age.gunpowder | city | scout | bombard | - | 3/3 (718‰ left vs 0‰) |
| age.gunpowder | tower | musketeer | bombard | - | 3/3 (1000‰ left vs 0‰) |
| age.gunpowder | fort | musketeer | bombard | - | 3/3 (940‰ left vs 0‰) |
| age.gunpowder | musketeer | scout | bombard | - | 3/3 (915‰ left vs 0‰) |
| age.gunpowder | arquebusier | scout | bombard | - | 3/3 (946‰ left vs 0‰) |
| age.gunpowder | bombard | city | scout | - | 3/3 (544‰ left vs 0‰) |
| age.enlightenment | city | ranger | cannon | - | 3/3 (776‰ left vs 0‰) |
| age.enlightenment | bastion | fusilier | cannon | - | 3/3 (1000‰ left vs 0‰) |
| age.enlightenment | star_fort | fusilier | cannon | - | 3/3 (958‰ left vs 0‰) |
| age.enlightenment | fusilier | ranger | cannon | - | 3/3 (942‰ left vs 0‰) |
| age.enlightenment | light_infantry | ranger | cannon | - | 3/3 (967‰ left vs 0‰) |
| age.enlightenment | cannon | city | ranger | - | 3/3 (596‰ left vs 0‰) |
| age.industrial | city | ranger | field_gun | - | 3/3 (790‰ left vs 0‰) |
| age.industrial | bastion | rifleman | field_gun | - | 3/3 (1000‰ left vs 0‰) |
| age.industrial | star_fort | rifleman | field_gun | - | 3/3 (1000‰ left vs 0‰) |
| age.industrial | rifleman | ranger | field_gun | - | 3/3 (940‰ left vs 0‰) |
| age.industrial | machine_gunner | ranger | field_gun | - | 3/3 (970‰ left vs 0‰) |
| age.industrial | anti_tank_rifleman | machine_gunner | field_gun | - | 3/3 (883‰ left vs 0‰) |
| age.industrial | armoured_car | ranger | field_gun | - | 3/3 (859‰ left vs 0‰) |
| age.industrial | light_tank | ranger | field_gun | - | 3/3 (859‰ left vs 0‰) |
| age.industrial | field_gun | city | ranger | - | 3/3 (513‰ left vs 0‰) |
| age.modern | city | ranger | howitzer | - | 3/3 (811‰ left vs 0‰) |
| age.modern | bunker | infantry | howitzer | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | fortress | infantry | howitzer | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | infantry | ranger | bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | heavy_machine_gunner | ranger | bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | bazooka_team | heavy_machine_gunner | bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | ranger | flak_gun | bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | scout_car | ranger | bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | tank | ranger | bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | howitzer | city | bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | flak_gun | fighter | infantry | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | fighter | bomber | flak_gun | - | 3/3 (1000‰ left vs 0‰) |
| age.modern | bomber | city | fighter | - | 3/3 (1000‰ left vs 0‰) |
| age.information | city | recon_drone | rocket_artillery | - | 3/3 (826‰ left vs 0‰) |
| age.information | bunker | assault_infantry | rocket_artillery | - | 3/3 (1000‰ left vs 0‰) |
| age.information | fortress | assault_infantry | rocket_artillery | - | 3/3 (1000‰ left vs 0‰) |
| age.information | assault_infantry | recon_drone | strategic_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.information | gun_drone | recon_drone | strategic_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.information | missile_team | recon_drone | strategic_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.information | recon_drone | missile_launcher | strategic_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.information | scout_car | recon_drone | strategic_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.information | main_battle_tank | recon_drone | strategic_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.information | rocket_artillery | city | strategic_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.information | missile_launcher | jet_fighter | assault_infantry | - | 3/3 (1000‰ left vs 0‰) |
| age.information | jet_fighter | strategic_bomber | missile_launcher | - | 3/3 (1000‰ left vs 0‰) |
| age.information | strategic_bomber | city | jet_fighter | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | city | recon_drone | rocket_artillery | - | 3/3 (826‰ left vs 0‰) |
| age.automation | defence_turret | combat_robot | rocket_artillery | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | fortress | combat_robot | rocket_artillery | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | combat_robot | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | gun_drone | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | missile_team | combat_robot | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | recon_drone | missile_launcher | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | hovercraft | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | robot_tank | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | rocket_artillery | city | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | missile_launcher | drone_fighter | combat_robot | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | drone_fighter | drone_bomber | missile_launcher | - | 3/3 (1000‰ left vs 0‰) |
| age.automation | drone_bomber | city | drone_fighter | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | city | recon_drone | railgun_battery | - | 3/3 (855‰ left vs 0‰) |
| age.fusion | defence_turret | exo_trooper | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | citadel | exo_trooper | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | exo_trooper | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | pulse_gunner | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | railgun_team | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | recon_drone | laser_battery | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | hovercraft | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | fusion_tank | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | railgun_battery | city | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | laser_battery | interceptor | exo_trooper | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | interceptor | drone_bomber | laser_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.fusion | drone_bomber | city | interceptor | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | city | recon_drone | railgun_battery | - | 3/3 (855‰ left vs 0‰) |
| age.starfaring | defence_turret | marine | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | citadel | marine | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | marine | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | pulse_gunner | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | railgun_team | marine | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | recon_drone | laser_battery | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | grav_skimmer | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | fusion_tank | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | railgun_battery | city | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | laser_battery | interceptor | marine | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | interceptor | drone_bomber | laser_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | drone_bomber | city | interceptor | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | city | recon_drone | railgun_battery | - | 3/3 (855‰ left vs 0‰) |
| age.expansion | defence_turret | marine | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | citadel | marine | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | marine | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | beam_trooper | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | railgun_team | marine | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | recon_drone | laser_battery | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | grav_skimmer | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | grav_tank | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | railgun_battery | city | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | laser_battery | interceptor | marine | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | interceptor | drone_bomber | laser_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | drone_bomber | city | interceptor | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | city | recon_drone | railgun_battery | - | 3/3 (855‰ left vs 0‰) |
| age.abundance | defence_turret | shock_marine | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | citadel | shock_marine | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | shock_marine | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | beam_trooper | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | railgun_team | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | recon_drone | laser_battery | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | grav_skimmer | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | grav_tank | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | railgun_battery | city | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | laser_battery | interceptor | shock_marine | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | interceptor | drone_bomber | laser_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | drone_bomber | city | interceptor | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | city | recon_drone | railgun_battery | - | 3/3 (855‰ left vs 0‰) |
| age.galactic | defence_turret | shock_marine | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | citadel | shock_marine | railgun_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | shock_marine | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | beam_trooper | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | railgun_team | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | recon_drone | laser_battery | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | grav_skimmer | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | grav_tank | recon_drone | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | railgun_battery | city | drone_bomber | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | laser_battery | interceptor | shock_marine | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | interceptor | drone_bomber | laser_battery | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | drone_bomber | city | interceptor | - | 3/3 (1000‰ left vs 0‰) |

The fighters of each age:

- age.ancient: city, spearman, slinger, scout
- age.classical: city, tower, hoplite, archer, scout, horseman, cataphract, catapult
- age.medieval: city, tower, fort, pikeman, crossbowman, scout, light_cavalry, knight, trebuchet
- age.gunpowder: city, tower, fort, musketeer, arquebusier, scout, hussar, cuirassier, bombard
- age.enlightenment: city, bastion, star_fort, fusilier, light_infantry, ranger, dragoon, lancer, cannon
- age.industrial: city, bastion, star_fort, rifleman, machine_gunner, anti_tank_rifleman, ranger, armoured_car, light_tank, field_gun, biplane
- age.modern: city, bunker, fortress, infantry, heavy_machine_gunner, bazooka_team, ranger, scout_car, tank, howitzer, flak_gun, fighter, bomber
- age.information: city, bunker, fortress, assault_infantry, gun_drone, missile_team, recon_drone, scout_car, main_battle_tank, rocket_artillery, missile_launcher, jet_fighter, strategic_bomber
- age.automation: city, defence_turret, fortress, combat_robot, gun_drone, missile_team, recon_drone, hovercraft, robot_tank, rocket_artillery, missile_launcher, drone_fighter, drone_bomber
- age.fusion: city, defence_turret, citadel, exo_trooper, pulse_gunner, railgun_team, recon_drone, hovercraft, fusion_tank, railgun_battery, laser_battery, interceptor, drone_bomber
- age.starfaring: city, defence_turret, citadel, marine, pulse_gunner, railgun_team, recon_drone, grav_skimmer, fusion_tank, railgun_battery, laser_battery, interceptor, drone_bomber
- age.expansion: city, defence_turret, citadel, marine, beam_trooper, railgun_team, recon_drone, grav_skimmer, grav_tank, railgun_battery, laser_battery, interceptor, drone_bomber
- age.abundance: city, defence_turret, citadel, shock_marine, beam_trooper, railgun_team, recon_drone, grav_skimmer, grav_tank, railgun_battery, laser_battery, interceptor, drone_bomber
- age.galactic: city, defence_turret, citadel, shock_marine, beam_trooper, railgun_team, recon_drone, grav_skimmer, grav_tank, railgun_battery, laser_battery, interceptor, drone_bomber

Without an answer on the 0.0.67 rules (same command):

| Age | Strength | Outranges and beats | Best try | Research | Its fights |
| --- | --- | --- | --- | --- | --- |
| age.ancient | city | spearman | spearman | - | 0/3 (0‰ left vs 924‰) NO ANSWER |
| age.ancient | tower | spearman | spearman | - | 0/3 (0‰ left vs 981‰) NO ANSWER |
| age.classical | catapult | city | horseman | - | 0/3 (0‰ left vs 946‰) NO ANSWER |
| age.medieval | trebuchet | city | light_cavalry | - | 0/3 (0‰ left vs 952‰) NO ANSWER |
| age.gunpowder | bombard | city | musketeer | - | 0/3 (0‰ left vs 754‰) NO ANSWER |
| age.enlightenment | cannon | city | fusilier | - | 0/3 (0‰ left vs 777‰) NO ANSWER |
| age.industrial | field_gun | city | biplane | - | 0/3 (1000‰ left vs 1000‰) NO ANSWER |

### What changed in the data (details in `docs/database.md`, "Counters and siege")

Siege is a `vehicle` with a 3-tile dead zone (`range_min`), melee does 150 % to
structures, attrition is 30 hit points a period, the tower comes in the classical age, and
woodcutter's camps and mines draw from 7 tiles. Before (0.0.67 rules) the matrix found 137
strengths and 7 without an answer: the city and the tower of the first age (nothing of that
age beat them at equal cost) and siege from the classical to the industrial age (it
outranged and beat every unit of its age, cavalry included; in the industrial age only the
biplane, which cannot hit the ground, was left to try). After: 136 strengths, none without
an answer.

### What changed in the computer players

- **Siege goes with an escort.** A type with a dead zone (`range_min` > 0, read from the
  weapon row) is never chosen as the bulk of an army; it comes only from the siege wish (a
  quarter of the main army, the type with most damage per resource against the target's
  armour), always joins the main army, and a gathered wave waits up to three minutes for
  one siege engine per six units when its target is a building that siege serves best.
  Formations put it behind the line.
- **Defenders answer siege.** Focus fire takes enemy siege first among armed enemies in
  reach; unit choice weighs each candidate by the exchange a fight would give against the
  enemies seen: damage dealt × staying power ÷ damage suffered, where an enemy weapon that
  cannot hit the candidate's layer, or whose dead zone is beyond the candidate's reach,
  deals nothing (so fast melee is valued against siege, as the matrix measures).
- **Lanes, not razing.** Buildings other than mines and woodcutter's camps (whose spot the
  deposit decides) are placed only with a free tile around them; with no such spot they
  wait. Razing to free boxed-in units stays as the last resort.
- **First-age stalls.** Traced: a capital ringed by rock had no spot left for its library
  and farms because the ground kept free near rock for a later mine took it all; the brain
  then issued no order for the whole match (seeds 91 and 92 of the moderate–easiest duel).
  That ground is now a preference: a second search ignores it. A town whose rock lay at the
  edge of its radius never placed a mine (no metal all match: seeds 15, 38, 73, 77): node
  sites draw from 7 tiles instead of 6, and new cities favour node terrain of a resource
  the player has no site for.
- **A defence during the grace period stays a defence.** Answering intruders before the
  difficulty's `grace_ticks`, the responding units guard the own city nearest them instead
  of following them to the other town (an easiest player's home guard chased a person's
  sentries into their town and struck a building at 28:55, inside its 30-minute grace).
- **Patience.** From 30 game minutes (was 40) the ratio a wave needs falls three points a
  minute (was two), to 60 % (was 80 %): siege now breaks walls that are measured at full
  strength.

### Measured (task 71)

On the i7-6700HQ, `rom duel`, 100 seeds, 8 jobs, 108 000-tick limit (the machine was
shared with another task's builds, so run times are rough):

| Duel | Wins | Draws | Player 1 / 2 wins | Run time |
| --- | --- | --- | --- | --- |
| 0.0.67: moderate – easiest | 85 – 3 | 12 | 44 / 44 | 6.2 min |
| 0.0.67: moderate – moderate | 37 – 36 | 27 | 33 / 40 | 20.0 min (shared) |
| task 71: moderate – easiest | 93 – 1 | 6 | 46 / 48 | 3.9 min |
| task 71: moderate – moderate | 48 – 39 | 13 | 46 / 41 | 7.0 min |

Draws of moderate – easiest (seeds 14, 15, 21, 23, 38, 90), traced: in 21 and 90 the
moderate player had one woodcutter's camp of four slots all match (timber +40 a period, one
or two cities, ages 2–3 against the easiest player's 4–6) while metal piled up past 10 000;
in 23 it reached the automation age with 72 units and seven cities, but the easiest
player's single town had no path in (46 `attack_move` orders failed `no_path`; the raiders
stood 48–64 tiles off); in 14 it had the larger economy but lost its waves at the easiest
player's walls, ending an age behind with 3 units; 15 and 38 were metal-starved in earlier
traces of nearly the same code (no mine placed).
Seed 8 is the one
easiest win (moderate ended in the medieval age). The mirror draws all end in the
enlightenment age with both capitals untouched: armies of 2–20 units trade raiders and
responses all match and no wave masses (traced seeds 16, 42, 61).

**The targets are not all met.** Moderate wins 93 (target 90) but draws 6 (target 5); the
mirror has 13 draws (target 10). The runs during the task show how much the result moves
with one change: with woodcutter's camps and mines drawing from 8 tiles instead of 7 the
moderate – easiest duel was 96 – 0 with 4 draws but the mirror had 11–12; with 6 tiles the
mirror had 7 draws but moderate – easiest 7–8. A half-way table and the traced causes are
in the task report; what is left is economy (food and timber cap every army while metal and
knowledge pile up in the thousands) rather than combat.

First attack on a person (`rom first-attack --seeds 1..8`):

| Difficulty | First wave 0.0.67 | First wave task 71 | First building hit 0.0.67 | First building hit task 71 |
| --- | --- | --- | --- | --- |
| easiest | 36:46 / 46:41, 7 units | 42:01 / 45:04, 7 units | 38:22 / 45:29 | 31:34 / 45:15 |
| easy | 44:34 / 45:13, 16 units | 18:40 / 19:44, 10 units | 31:58 / 44:53 | 17:43 / 30:29 |
| moderate | 22:07 / 23:46, 16 units | 21:07 / 23:42, 15 units | 23:37 / 25:50 | 5:05 / 23:37 |
| tough | 22:16 / 25:02, 13 units | 21:25 / 23:44, 14 units | 23:10 / 25:29 | 5:21 / 22:07 |
| tougher | 22:22 / 23:56, 15 units | 20:26 / 22:07, 13 units | 22:36 / 25:01 | 21:13 / 24:40 |
| toughest | 20:39 / 22:14, 16 units | 19:49 / 20:35, 15 units | 20:40 / 23:27 | 16:40 / 22:08 |

(earliest / mean). The hard levels are no gentler. At easiest the mean first hit is the
same (45:15, was 45:29) and every hit now comes after its 30-minute grace; the earliest is
earlier than before (31:34, was 38:22) and one seed in eight saw no attack (was two).

## Economies that do not stall; a gentle easy level (task 78)

Task 71's remaining draws traced to the economy, not to combat. Traced again on 0.0.69 with
the gather sites now printed by `rom duel` (`sites=` in the result line and the trace):

- **One small woodcutter's camp all match** (seeds 21, 90): the start's wood lay hard by the
  town centre and the shore. The opening placed its camp at tick 1 where it reached four trees
  of nine (with a free tile kept round every building, the spot beside the centre was never
  tried), and no second camp ever fitted. Timber stayed at +40 a period; the city that would
  have reached other wood costs 300 timber and was never saved for, because the way to the
  next age and the soldiers held the one reservation, so the town stayed alone until minute
  65–75 with metal past 10 000.
- **No mine all match** (seeds 15, 38): the only rock lay beside the capital and a farm or the
  second woodcutter's camp took the one spot a mine could have had, minutes before mines were
  known (the ground near rock was kept free only as a preference, and node sites ignored it).
- **A camp of one slot** (seed 50 during this task): the opening camp was placed where it
  reached one tree; the next camp, paid in timber, waited behind the way to the next age, which
  held the savings, and timber came in at +10 a period for the whole match.
- **No path into a town** (seed 23): the attack-moves failed 50 tiles out with nothing near
  the units (terrain, not a wall); with the economy fixed the duel is won before that.

What changed in `rom-ai`:

- **Room for every node site.** In each thought that builds, the spot the first site of every
  node resource would take is kept free (`Hold::First`), also for a resource whose age has not
  come (`Hold::Coming`: rock before mines are known), and for a resource whose sites hold fewer
  workers than twice the commerce cap pays for, the spot of its next site (`Hold::Next`). Other
  buildings give up the next-site spots first, then the coming ones, and never the first site's
  spot of a resource the player can build (`Keep` passes in `sites::find_site`; a town on a small
  island still gets its farms). A node site never takes the spot kept for another type, and
  the spots are kept also while a node site is being placed (else the next woodcutter's camp
  took the only spot beside the rock while a mine was looked for). The firmest are kept
  first. Spots are judged on what the player knows, without a placement preview
  (`sites::node_spot`).
- **Camps where the wood is.** A node site may stand hard by another building when that spot
  reaches clearly more of the deposit (three slots or half as many again), unless it would cut a
  way: the free tiles round it must still reach each other within ten tiles, or all but one of
  the regions they lead to must be small closed pockets (`sites::cuts_a_way`; an early version
  without the test shut a town's workers in, one without the pockets left a mine beside a
  five-tile rock unbuilt).
- **Starved sites first.** A site for a resource with fewer than half the slots its workers need,
  paid in that very resource, is wished for at 1100, above the way to the next age at its most
  urgent; it reserves its price.
- **A city where the resource is.** A resource is *stuck* when the last search for its site came
  back empty with as many cities and in the same age as now, its sites are full, the commerce
  cap would pay for more hands and the plans lack it. Then another city is wished for at 850 with
  its price reserved (ahead of soldiers and the next age), the city limit technology is valued as
  by the most eager profile, and the new city goes where the node terrain of the stuck resource
  is (as already for a resource with no site).
- **Breaking a way in.** When members of an advancing army failed with `no_path` three times in a
  minute (the simulation's order failures), every member that can hit it strikes the remembered
  enemy building nearest the army's front within eight tiles of the straight way to the goal; one
  that cannot be reached either fails too and is left alone, and the next is tried (`army::breach`).
  The march itself already strikes buildings in its way (`attack_move`); in the traced duels the
  remaining `no_path` failures were terrain.
- **Market trade** of surplus is not possible: the rules have no order that sells a resource
  (trade routes bring wealth only). Surplus goes into research and the army as before.

Easy is gentle again (data only, `docs/database.md`): `grace_ticks` 54000 (45 minutes) for easy
and 57600 (48 minutes) for easiest: with economies that no longer stall the low levels' armies
are ready sooner (easiest's first wave came at 36 minutes with its old 30-minute grace).

### Measured (task 78)

On the i7-6700HQ, `rom duel`, 100 seeds, 8 jobs, 108 000-tick limit. The machine was shared
with other tasks' builds and tests (load 15–25), so run times are rough.

| Duel | Wins | Draws | Player 1 / 2 wins | Run time |
| --- | --- | --- | --- | --- |
| 0.0.69: moderate – easiest | 93 – 1 | 6 | 46 / 48 | 6.8 min (6 jobs) |
| 0.0.69: moderate – moderate | 48 – 39 | 13 | 46 / 41 | 9.3 min (6 jobs) |
| task 78: moderate – easiest | 96 – 0 | 4 | 48 / 48 | 7.3 min |
| task 78: moderate – moderate | 53 – 42 | 5 | 46 / 49 (read from which side kept its sites) | 14.0 min |

Single gather sites at the end (`sites=`; 0.0.69 did not print them): the moderate player ended
7 of the 100 moderate – easiest matches with one woodcutter's camp or one mine (seeds 4, 5, 27,
29, 31, 90, 91; in 4, 5, 27 and 31 it won anyway, with a site of 5–10 slots), and 2 of the
mirror's winners with one mine (41, 77). Each is a start whose only wood or rock in reach
holds one site. The card's measure (no single site of a spent resource) is therefore not met on
those maps. Draws of moderate – easiest, traced (`--trace 12000`):

- **seed 14**: the moderate player leads (age 5 against 4, 51 soldiers, 10 000 timber and
  13 000 metal in store) but food is its cap (four cities, twenty farms, +180 a period) and its
  armies spend the last half hour answering the easiest player's raiders inside its borders.
- **seed 29**: the only rock in its land is five tiles beside the capital, in a pocket closed by
  the town centre; a mine there would shut in what the town trains, so none is built and metal
  stays at 9. An army of 85 without metal units answers raids and kills 18.
- **seed 90**: the start's only wood is seven tiles beside the capital (one camp of seven
  slots, timber +87); 18 attack-moves failed with `no_path` (terrain: the army stands with no
  building in its way).
- **seed 91**: an island capital with no rock in reach (metal +0), so three soldiers at the end.

Of the causes task 71 traced, the single small camp of a town whose wood lay beside its centre
(21), the missing mine (15, 38) and the stalled first ages no longer draw; seed 90 still has
room for only one camp. Determinism: the four draws traced again (other jobs, stepped in trace
chunks) and seeds 1–3 of the mirror run alone gave the hashes of the 100-seed runs.

First attack on a person (`rom first-attack --seeds 1..8`, earliest / mean):

| Difficulty | First wave 0.0.69 | First wave task 78 | First building hit 0.0.69 | First building hit task 78 |
| --- | --- | --- | --- | --- |
| easiest | 42:01 / 45:04, 7 units | 48:01 / 51:29, 5 units | 31:34 / 45:15 | 48:27 / 52:05 |
| easy | 18:40 / 19:44, 10 units | 45:01 / 45:38, 8 units | 17:43 / 30:29 | 45:11 / 46:05 |
| moderate | 21:07 / 23:42, 15 units | 17:51 / 22:27, 15 units | 5:05 / 23:37 | 18:03 / 23:18 |
| tough | 21:25 / 23:44, 14 units | 21:26 / 23:52, 13 units | 5:21 / 22:07 | 23:08 / 26:29 |
| tougher | 20:26 / 22:07, 13 units | 14:34 / 21:31, 14 units | 21:13 / 24:40 | 22:18 / 23:55 |
| toughest | 19:49 / 20:35, 15 units | 18:54 / 21:39, 15 units | 16:40 / 22:08 | 18:25 / 22:23 |

Easy is back where 0.0.67 had it (44:34 / 45:13) and easiest comes last. The hard levels are
within a minute of 0.0.69: their first waves are as early or earlier except toughest's mean
(a minute later) and tough's (eight seconds); their first building hit comes later where 0.0.69
had a raider strike a building in the first six minutes.

`rom fight-matrix` (the owner's counter rule, unchanged data): 136 strengths, none without an
answer (2.8 min, 2 jobs).

## Automated colonies and their governors (task 73)

An automated colony (abundance age, `docs/database.md` "Automated colonies") keeps the sites its
type lists in `et_colony_site` standing in its area. The keeping is done by **the governor**,
`rom_ai::governor`, which is not part of a brain:

- **Every owner, people included.** Hosts call `rom_ai::run_governors(&mut world)` before every
  `World::step` (the application's simulation thread, `rom ai-match`, `rom duel` and
  `rom first-attack`); every `GOVERNOR_INTERVAL` ticks (40, two seconds) it asks each player's
  colonies, in id order, for at most one order each and submits them. Refused orders are tried
  again on a later round. It keeps no memory, so nothing of it is saved; what it reads is the
  world (the colony's buildings, its queue, pending orders, prices, stores, the switch).
- **Orders only, in the owner's name.** The colony is a builder that does not move: the governor
  orders it to `construct` a site (`queue` off, one at a time) or to `repair` an unfinished one
  nobody builds. The colony pays at submission like anyone; it builds at `build_pct` 200.
- **Which site next.** The kinds below their caps, furthest below as a share of the cap first
  (ties in the colony type's order): the first one the owner can build now (unlocked, paid for)
  and that finds room is placed. So farms, camps and labs come up in turn, and a kind that
  cannot be paid for does not hold up the others.
- **Where.** Every tile of the colony's economic radius, nearest first, snapped as the
  simulation snaps it; a spot must pass `World::check_placement`, be served by the colony itself
  (`city_covering`, so a nearer city's area is left to that city) and keep a free tile round the
  footprint. A site that draws from a node terrain (a woodcutter's camp, forest) goes to the spot
  with the most of it within its `node_radius`, weighed over up to 48 spots spread over the area;
  any other takes the nearest spot.
- **Rebuilds.** Counts are of the line in the colony's area (unfinished included), so a site
  destroyed is simply missing on the next round and placed again.
- **The switch and the panel.** `rom_ai::colony_status(world, colony)` gives each kind's count
  against its cap and what the governor waits for (`Waiting`: paused, the colony itself
  unfinished or struck down, building a site, a kind locked, the resources the most needed kind
  lacks, placing). The application shows it in the colony's card ("Farm 5/5 · Camp 3/5 · Lab
  4/5", "Governor on: building Camp" or "needs 200 Metal") and its Order tab has **Pause
  governor** / **Resume governor** (orders `pause_governor`, `resume_governor`).

**Brains.** A brain treats a colony type (an `et_colony` row) apart from cities: it wishes one
colony per town of its own (score 520, not reserved, so only when it pays) at a site found like
a city's (`find_city_site`: inside its borders, a spacing from every city); colonies are not
among the brain's towns (`View::cities`), so its own farms and workers stay out of their areas;
builders without movement are never sent to build; and in every thought it resumes any of its
colonies whose governor is paused. Nothing names a definition: colonies are found by the
component, their sites by their rows.

### Measured (task 73)

`rom duel`, 100 seeds, 8 jobs, 108 000-tick limit, on the i7-6700HQ (other sessions' builds ran
alongside, so run times are rough). No duel reaches the abundance age, so colonies play no part;
what could change them is the survey ship's own line (the corvette no longer needs "Upgrade to
Corvettes") and the definition ids after it moving down by one.

| Duel | Wins | Draws | Draw seeds | Run time |
| --- | --- | --- | --- | --- |
| task 78 (0.0.72): moderate – easiest | 96 – 0 | 4 | 14, 29, 90, 91 | 7.3 min |
| task 73: moderate – easiest | 96 – 0 | 4 | 14, 29, 90, 91 (the causes traced in task 78) | 6.5 min; 4.8 min with the sites' prices |
| task 78 (0.0.72): moderate – moderate | 53 – 42 | 5 | | 14.0 min |
| task 73: moderate – moderate | 53 – 42 | 5 | 36, 41, 43, 55, 68 | 6.8 min; 6.0 min with the sites' prices |

Seeds 1–3 of the mirror run again alone gave the same hashes. The automated sites' prices (added
after the owner's review) changed neither table: no duel reaches the automation age. `rom fight-matrix`: 136 strengths,
`unanswered=0` (27 s).

## Bombardment and batteries (task 75)

**Using bombardment.** The strike fleet (the escort, three warships or more) goes for the orbit
of a planet in reach where enemy buildings are known. Over it a ship fires on the batteries seen
there first (a type with a weapon at the `orbit` layer that is not itself orbital), then the
cities (a territory row with `is_city`), then what is nearest; a ship that fired by itself at
something less important is turned to the better mark, and a ship that only holds its orbit
counts as free to fire. With the frigate's area volleys this levels the cities of other
planets, which the computer players cannot reach on the ground. **Escorting it:** the strike sets
off only with two more warships for every enemy battery known on that planet, and the escort
asks for that many (at most twelve), so a planet defended by batteries is held off until the
fleet can take them; the carrier still launches its craft over the target.

**Answering it.** A planet is bombarded for a player when it sees an enemy with a weapon that
strikes the ground from orbit (`orbit_only`) holding the orbit over it, or a blast of an
enemy's area weapon on its ground (`World::blasts()`), and has a city there. The construction
manager then wishes for two ground batteries (by the weapon's layer, not by name) per city on
such a planet, at score 780 and reserved: above soldiers and most buildings, below the age path.
The battery is placed like other buildings, around the player's cities in order.

Nothing names a definition: the rank comes from weapons and territory rows.

### Measured (task 75)

`rom fight-matrix` across the orbit (`crates/rom-cli/src/orbit_matrix.rs`; fights on a planet of
plain ground in a star system of its own, at equal cost as on the ground, 3 seeds, the ships
entering the orbit beyond both reaches and ordered at the nearest enemy they can aim at; the
volleys of one ship against a town as in `database.md`, "Orbital bombardment"):

| Age | Across the orbit | Outranges and beats | Answer | Research | Answer's fights (won, value left) |
| --- | --- | --- | --- | --- | --- |
| age.starfaring | corvette | city | ground_battery | - | 3/3 (999‰ left vs 0‰) |
| age.starfaring | orbital_bomber | city | ground_battery | - | 3/3 (966‰ left vs 0‰) |
| age.starfaring | laser_battery | orbital_fighter | marine | - | 3/3 (1000‰ left vs 0‰) |
| age.starfaring | ground_battery | corvette | marine | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | space_frigate | city | ground_battery | - | 3/3 (703‰ left vs 0‰) |
| age.expansion | orbital_bomber | city | ground_battery | - | 3/3 (966‰ left vs 0‰) |
| age.expansion | laser_battery | orbital_fighter | marine | - | 3/3 (1000‰ left vs 0‰) |
| age.expansion | ground_battery | space_frigate | marine | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | space_cruiser | city | ground_battery | - | 3/3 (373‰ left vs 0‰) |
| age.abundance | orbital_bomber | city | ground_battery | - | 3/3 (966‰ left vs 0‰) |
| age.abundance | laser_battery | orbital_fighter | shock_marine | - | 3/3 (1000‰ left vs 0‰) |
| age.abundance | ground_battery | space_cruiser | shock_marine | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | battleship | city | ground_battery | - | 3/3 (192‰ left vs 0‰) |
| age.galactic | orbital_bomber | city | ground_battery | - | 3/3 (966‰ left vs 0‰) |
| age.galactic | laser_battery | orbital_fighter | shock_marine | - | 3/3 (1000‰ left vs 0‰) |
| age.galactic | ground_battery | battleship | shock_marine | - | 3/3 (1000‰ left vs 0‰) |

Every ship that bombards beats the city of its age at equal cost and is answered by the ground
battery without research; the battery outranges and beats the ships of its age and is answered
by the age's infantry, which it cannot fire at (troops landed out of its reach). The test
`every_orbit_strength_has_an_answer` fails when one of these has no answer, and
`a_frigate_or_cruiser_levels_a_town_in_three_to_five_volleys` holds the owner's target.

`rom duel`, 100 seeds, 4 jobs, 108 000-tick limit (a test suite and the other duel ran alongside,
so run times are rough). No duel leaves its planet, so neither bombardment nor batteries play a
part; the duels are the same match for match as 0.0.75 (seeds 1–3 of the mirror run again alone
gave the same hashes). A first version gave the battery an armour class of its own and moved
the first duel to 94–0 with 6 draws: computer players weigh units against enemies not yet seen
by an average over every armour class, so the battery now wears an existing one (`hull`).

| Duel | Wins | Draws | Draw seeds | Run time |
| --- | --- | --- | --- | --- |
| 0.0.75: moderate – easiest | 96 – 0 | 4 | 14, 29, 90, 91 | |
| task 75: moderate – easiest | 96 – 0 | 4 | 14, 29, 90, 91 | 11.9 min |
| 0.0.75: moderate – moderate | 53 – 42 | 5 | 36, 41, 43, 55, 68 | |
| task 75: moderate – moderate | 53 – 42 | 5 | 36, 41, 43, 55, 68 | 19.5 min |

`rom fight-matrix`: 152 strengths (136 on the ground, the same rows as 0.0.75, and 16 across the
orbit), `unanswered=0` (46 s).

## At the market (task 107)

Once its wishes are served, a brain with a complete market and the research that enables trading
(`World::can_trade`) trades through the ordinary `buy` and `sell` orders, at the prices any player
sees (`World::market_quote`); nothing in it names a resource (`crates/rom-ai/src/market.rs`):

- **Selling surplus.** Each tradeable resource (id order) its plans did not lack this thought is
  sold, one lot, while more than `market.sell_above` (1500) would remain and a lot still pays at
  least `market.sell_min_pct` (70) of the base price.
- **Buying shortfalls.** Then the resources this thought's wishes lacked, largest shortfall first:
  one lot each while it costs at most `market.buy_max_pct` (140) of the base price and the
  currency left stays above `market.wealth_reserve` (300) plus what the wishes lacked of it.
- At most `market.lots_per_thought` (1) lots a thought and one per resource; the purse of the
  thought follows each trade. The weights are `ai_weight_key` rows, so a profile may override
  them.

### Measured (task 107)

`rom duel --seeds 1..100 --jobs 2` (conquest, flat two-player maps), before and after trading:

| Profiles | 0.0.100 | Task 107 |
| --- | --- | --- |
| `balanced:moderate,balanced:easiest` | 94–1, 5 draws, mean tick 56 192 | 97–0, 3 draws, mean tick 53 258 |
| `balanced:moderate,balanced:moderate` | 48–42, 10 draws, mean tick 70 179 | 48–47, 5 draws, mean tick 60 390 |

Selling surplus pays for research and units sooner: matches end earlier and fewer are drawn.
The playthrough does not trade, so its hashes stay; `rom fight-matrix` is unchanged
(`strengths=255 unanswered=0`).

## Drone operators (task 120)

The drone operator (information age, `docs/database.md` "Drone operators") outranges every foot
soldier and vehicle of its age through its cloud (8 tiles against 4 to 6) and beats them:
`rom fight-matrix --ages 7 --fight unit.drone_operator,<b>` (3 seeds, value left in ‰):

| Against | Operators | The other side |
| --- | --- | --- |
| assault infantry | 1000, 1000, 1000 | 0, 0, 0 |
| main battle tank | 868, 824, 824 | 0, 0, 0 |
| missile launcher (anti-air) | 1000, 1000, 1000 | 0, 0, 0 |
| rocket artillery | 0, 0, 0 | 723, 815, 861 |
| strategic bomber | 0, 0, 0 | 1000, 1000, 1000 |

Its answers are what reaches the operator itself: artillery from beyond its reach (rocket
artillery, 13 tiles) and aircraft, which its drones cannot strike (strategic bomber in its age,
drone bomber after). Anti-air thins the cloud (a missile launcher's hit brings down three
drones) but cannot touch the operator, and its own crew falls to one drone each, so it holds the
cloud off without winning. The matrix gains seven rows, one per age from the information age on
(`drone_operator | city | strategic_bomber` or `drone_bomber`): `strengths=270 unanswered=0`
(263 before).

Computer players choose it through the ordinary army choice. `rom-ai` weighs a cloud weapon at
the pace its operator issues drones (`et_drone_cloud.rebuild_ticks`, 120, when longer than the
weapon's 10-tick reload) in `production::value`, `dps_against` and `view::strength`: a burst of
sixteen drones is not a sustained rate. `rom duel --profiles balanced:moderate,balanced:easiest --seeds 1..20 --jobs 2`: 20–0, mean
tick 54 449, as before: no duel reaches the information age.

## The drone grid (task 128)

Task 128 put the drones in a 4 × 4 grid (`docs/database.md`, "The drone grid"): one drone per
enemy, a drone sent every 5 ticks instead of 10 (four a second, one or two in the air at once,
a 16-drone grid spent in four seconds), every drone a target of its own that one anti-air hit
brings down (a missile launcher's hit brought down three of task 120's drones), the operator a
target for everything that hits the ground. One drone still kills assault infantry and ten the
main battle tank (counted in `rom-sim`'s `drone_cloud` tests). `rom fight-matrix --ages 7
--fight unit.drone_operator,<b>` (3 seeds, value left in ‰):

| Against | Operators | The other side |
| --- | --- | --- |
| assault infantry | 1000, 1000, 957 | 0, 0, 0 |
| main battle tank | 912, 912, 912 | 0, 0, 0 |
| missile launcher (anti-air) | 1000, 1000, 1000 | 0, 0, 0 |
| rocket artillery | 0, 0, 0 | 677, 769, 815 |
| strategic bomber | 0, 0, 0 | 1000, 1000, 1000 |
| gunship helicopter | 0, 0, 0 | 1000, 1000, 1000 |

The answers are unchanged: artillery from beyond the operator's reach and aircraft, which its
drones cannot strike. The counter rule holds with no re-tuning beyond the strike pace:
`rom fight-matrix` `strengths=270 unanswered=0`, the same seven `drone_operator` rows.
`rom duel --profiles balanced:moderate,balanced:easiest --seeds 1..20 --jobs 2`: 20–0, mean tick
54 449, as before: no duel reaches the information age.

## Capitals revealed by research (task 130)

Nothing in `rom-ai` changed. The research manager offers the new technologies (Orbital Survey,
Deep Space Survey, Long-range Imaging, Total Network) as side research by their worth, like
every technology without modifiers; the construction manager builds the spaceport from the
information age and the Deep Space Tracking Station as other buildings that research something;
the Orbital Survey waits for a satellite, which the space manager trains from the starfaring age
(its wishes start there). Space probes are not trained. The army already doubles the value of
an enemy capital it knows by sight or by the survey (`World::capital` on a remembered city);
`World::known_capitals` is there for a brain that wants a capital known only by research as a
goal (its planet or system to head for), which is left for later: it would send armies and
fleets across systems toward a city nobody has seen.

## Militia (task 141)

Task 141 changed nothing in `rom-ai`; task 152 (below) calls up militia as sketched here. It could, with
the orders a person has (`call_up`, `stand_down`), but doing it well is more than a few lines:

- *When*: the alarm (`attacked`) already says a city is hurt or intruders outnumber the army at
  home; the call-up would take the workers inside that city's borders, after the difficulty's
  reaction time.
- *Who leads them*: a militia is a fighter by its data (armed, neither builds nor gathers), so
  `production::is_military` and `army::fighters` already count it and the army manager would
  take it into its armies and march it away; it must instead be held at the city it defends,
  where it can stand down (a stand-down is refused outside the borders).
- *Afterwards*: once the alarm is over every militia inside the borders stands down; the
  economy manager must not train workers to replace the called-up ones meanwhile (they are
  not workers while they are militia), or it overshoots when they come back.

Until then the computer player owns no militia, so its armies, economy and the duel are as
before; the rules change only shifts later definition ids. A person's militia it meets are
soldiers to it: it sees their weapons and strength like any unit's.
Measured with the militia in the rules: `rom fight-matrix` `strengths=270 unanswered=0` (the
militia has no production row, so it is not in the rosters); `rom duel --profiles
balanced:moderate,balanced:easiest --seeds 1..20 --jobs 2`: 20–0, mean tick 53 613, as before.

## Rebuilding, energy and militia (task 152)

Three gaps found while merging tasks 140, 141 and 146 (`rebuild.rs`, `economy.rs`, `militia.rs`).

**Rebuilding what was lost.** Every thought the brain compares its finished buildings with those
of the thought before (memory: `standing`, by entity id, type and place). One missing now was
destroyed or taken (one it pulled down itself, `unbox`, is forgotten first); its line (the first
type of its upgrade line) is remembered with how many the line held before the loss and where the
lost one stood (`lost`). Cities and automated colonies are left to their own wishes. While the
line holds fewer than that (unfinished ones count; at most what its per-city or per-player limit
allows):

- the line is wished for, after every other manager has wished, at the construction manager's
  score for one more of it (`construction::score`), or 300 when that manager would want no more;
  a producer whose products are wished for now (the product itself, or for a building that
  trains military units another military unit of the same element, for one that trains
  spacecraft another spacecraft) ranks as the best of those wishes. A line the construction
  manager wishes for already keeps that one wish, raised the same way; gather sites are left to
  the economy, which places them by the slots and income it lacks.
- Any building of the line, whoever wishes for it, goes first where a lost one stood, when that
  ground is still free (`sites::at_spot`: no known building in the way, room in the city's
  per-city limit, the simulation's placement preview), else to the nearest legal place as
  always. The spots are kept for the line as loosely as the next site of a resource
  (`Hold::Next`): other buildings take them only when nothing else fits.

The spaceport of the task 140 test is rebuilt by the space manager's and the construction
manager's existing wishes; what was missing was the place. In `the_warship_class_follows_the_enemy_ships_seen` the
loosening of task 140 is reverted: it counts finished warships again, and that holds (it held on
main at the start of this task too: the bombardment no longer wrecks the spaceport before the
first cruiser in that scenario).

**Energy.** A base resource that no unit gathers but that sites needing no workers earn
(`et_gather_site` with `slots_max` 0 and a `self_income`: the nuclear reactor from the Automation
age, the fusion plant it becomes in the Fusion age) now counts in the economy, with no workers
given to it. Once every manager has wished, the economy wants income of it proportional to what
the plans spend of it: the price in it of this thought's wishes plus what earlier wishes lacked
(the smoothed demand), brought in over 10 income periods (`SPEND_PERIODS`, five game minutes); the
sites wanted are the working ones plus enough for the shortfall at the type's own income, and at
least one per city, as far as the type's limit allows (one per city for the reactor). While the
plans spend more than comes in the site is wished for at 880 and saved for (as the first site of
a resource); otherwise, while a city lacks one, at 600 (with the technologies, below the gather
sites that feed the workers). The upgrade to the fusion plant is the simulation's; the line counts
as one. Before this, the price of the first reactor was kept back from every purchase from the
Automation age on (`Plan::with_first_sites`) and never spent.

**Militia.** A city is *raided* while armed enemy units seen lately stand in its town (its
economic radius and four tiles beyond, where its workers and sites are) or it lost hit points
since the last thought (`raids`: when it began, when the last sign was). Not its whole border: a
city's border reaches 40 tiles, and a first version that called up against intruders anywhere
inside it sent workers after raiders far from their town and lost them (duel seed 9: workers
fell from 38 to 24, and the match was drawn). After the difficulty's `reaction_ticks` from the first sign,
while intruders are there, workers that can be called up (`World::militia_change`) inside the
city's economic radius are called up, nearest to the intruders first, until the defence of the
town (the fighters in it, its militia, and its armed buildings, the city's own guns included,
that have an intruder within their reach) is 150 % of the intruders' strength (`CALL_UP_PCT`),
but never more than half of the workers inside that radius (`MAX_SHARE_PCT`, militia already out
included): the rest keep the economy going. Each militia is the city's (`militia`, by entity id): never enlisted in an
army (`army::fighters` and the army's population leave militia out), it attack-moves to the
intruder nearest the city while the raid lasts, and walks back to the city when it strayed. When
no sign of an attack has been seen for 600 ticks (`STAND_DOWN_TICKS`, half a game minute) it
stands down where it is inside the own territory (one outside walks home first), and the economy
sends the worker back to work. Workers called up count as workers for the economy (`eco.militia`
in the worker total and in the hands of the worker types that can be called up), so none is
trained to replace them. The sim's under-attack alerts (task 118) are not read: they are neither
saved nor hashed, so a brain restored from a save would not see them.

Memory layout version 5 adds the buildings standing and lost, the militia and the raids; versions
1 to 4 still read.

### Measured (task 152)

`rom duel --seeds 1..20 --jobs 2` (conquest, flat two-player maps, 108 000-tick limit):

| Profiles | Main before (0.0.132) | Task 152 | Task 152 without militia (trial) |
| --- | --- | --- | --- |
| `balanced:moderate,balanced:easiest` | 20–0, mean tick 52 405 | 20–0, mean tick 57 678 | 19–0, 1 draw (seed 14), mean tick 51 818 |
| `balanced:moderate,balanced:moderate` | | 11–9, no draws, mean tick 61 709 | |

No duel reaches the Automation age, so energy plays no part in them; what moves them is the
militia and, less, the rebuilding. The losing side now calls up its workers when a wave reaches
its town: against the trial without militia, conquests in seeds 2, 6, 10, 16 and 17 come 7 to
14 game minutes later, in seeds 5, 8 and 11 earlier. Seed 14, the slowest, is won at tick
103 472, close to the limit; without militia it is drawn. A first version that called up against
intruders anywhere inside the 40-tile border drew seeds 9 and 14 (18–0): in seed 9 the moderate
player had 10 workers under arms against raiders far from its towns, and its workers fell from
38 to 24 while its age stalled.

`rom fight-matrix`: `strengths=270 unanswered=0`, as before (it fights without brains).
`rom playthrough --seed 1` (the simulation's own bot, no brain): `hash=572e8c1fd9800f8c` and
ascension `ascended_tick=59544 hash=f787a8ee44aec6a6` on the main this task started from, unchanged.

## Armies and fleets in formation, and a firmer win (task 153)

**Formations by their rows** (`formations.rs`). Where the difficulty `uses_formations`, an order
that moves a group names a formation chosen from the `formation` rows by shape and numbers, never
by key, and every member is sent to one point: the simulation lays the group out (task 143),
ranks facing from the group's centre to the point, ranged and support members behind when the
row's `ranged_behind` says so, the group kept to its slowest member. Only rows that lay out
straight ranks are chosen (line, double line, the blocks); rows that put ranged members behind
and keep to the slowest are preferred when there are any.

- **Attack** (the main army's advance and assault, ships contesting a body or striking an
  orbit): the row with the most members in its front rank whose front is no wider than the
  frontage and no deeper than wide; a group that mixes close fighters with ranged or support
  members needs two ranks at least. Ties go to the spacing nearest the members' own that is not
  tighter (`spacing_pct` 100 or more, the smallest), then the id. With the seeded rows a wave of
  up to 13 walking soldiers forms one line, up to 26 a double line, more a square block; a
  mixed wave a double line or a block.
- **Frontage**: 12 tiles (twice `SLACK`, so every place stands within reach of the goal and a
  member waiting at its place is not sent on), narrowed to the open ground at the goal: from the
  goal across the way in from the members' centre, tile by tile to either side until ground the
  most common member type cannot stand on (terrain only; buildings are the simulation's lattice
  fallback). When nothing wider than deep fits, the narrowest row: a column through a gap.
  Nothing else measures a narrow way: the path search does not report its width.
- **Hold** (the main army holding a captured enemy capital, warships holding an own claimed
  body): the row as wide as deep for the group, the square.
- **Fleets**: warships on `orbit` and `travel` take the same choice with no frontage limit: line
  abreast to strike or contest, the square to hold. Claiming ships and transports go alone.
- **Not in formation**: the answer to intruders (a formation walks at its slowest member's
  pace, and an answer that comes late loses the workers it is for: a version that answered in
  formation drew duel seed 9), scouts, raiders, supply wagons, militia, the navy at sea.

A wave sent off over several thoughts (the order budget) is laid out batch by batch about the
same point; a member that arrives alone goes to the point itself.

**Duel seed 14** (moderate won at tick 103 472 on main), traced with `rom duel --trace` and
temporary prints of the order budget:

- **The order budget was spent on orders that failed.** From tick 21 000 the moderate player had
  three orders a thought and used them all: two to five a thought were `trade` orders to a city
  no trader could reach and `guard` orders to a capital its home guard could not reach (both
  `no_path`, given again every thought because nothing remembered the failure), the rest market
  sales. The navy, ordered last, got none for 43 000 ticks: sixteen warships waited at the
  harbour beside a shore target. Now a city traders find no way to is not a route's end for a
  minute (`heed_failures`, `memory.unreachable`, `economy::route_for`), and a capital (or city)
  its guard found no way to is not guarded for two minutes: the guard waits at the rally point.
- **The main army waited for a full wave against nobody.** With the warships holding a third of
  the army's population, the land army stayed at 14 of the 16 a wave waits for from tick 86 000,
  while the easiest player had no soldier left. Fixed by the two causes below rather than by a
  smaller wave (that would also change the easiest level).

Seed 14 alone, with only the budget fixed: won at tick 62 320. Two more weaknesses showed in
the seeds that grew slower while formations went in (seed 9 at 100 443, then drawn):

- **A gathering army drawn into an enemy town.** The easiest player's temple stood seventeen
  tiles from the moderate player's rally point; the gathered main army went after it by itself
  (automatic attacks), piecemeal, and died there before it ever set off. Members of an army that
  gathers (or retreats) and went after something by themselves farther than twelve tiles from
  the rally point are now called back (`army::regroup`, `CHASE_SLACK`).
- **A navy that could not fight.** A shore target warranted a navy even when no warship the
  player could train outranged its defences at the ground (triremes cannot shell the coast at
  all), so a fifth of the military target (9 to 14 ships in seed 9) waited at the harbour all
  match. A shore target now calls for a navy only when a trainable warship outranges the
  defences there (`navy::shore_defence`); enemy warships seen call for one as before.

### Measured (task 153)

`rom duel --seeds 1..20 --jobs 2` (conquest, flat two-player maps, 108 000-tick limit):

| Profiles | Main before (0.0.134) | Budget fix only (trial) | Task 153 |
| --- | --- | --- | --- |
| `balanced:moderate,balanced:easiest` | 20–0, mean tick 57 678, slowest 103 472 (seed 14) | 20–0, mean tick 55 410, slowest 75 589 (seed 15) | 20–0, mean tick 47 463, slowest 62 598 (seed 13); seed 14 at 47 189 |
| `balanced:moderate,balanced:moderate` | 11–9, no draws | | 9–10, one draw (seed 4), mean tick 54 795 |

Seed 4 of the mirror is not a stall: both players fought a war of attrition from tick 42 000
(each fell to 30 to 60 population, its army to a few soldiers while its navy kept its ships),
the first rebuilt and at the limit was taking the other's last city (population 135 against 13).
The easiest level's own changes are the budget fix (it wasted orders the same way) and the
recall; it does not use formations or keep a navy. Against it the moderate player now wins eight
and a half game minutes sooner on average. `rom first-attack --seeds 1..8` gives the easiest level
the same numbers as main (first wave 48:01 / 52:20 with 5 units, first building hit 48:30 / 51:46,
two seeds without an attack in an hour); moderate's first wave comes at 18:33 / 23:16 (was 21:59 /
24:24), its first hit at 22:52 / 26:15 (was 17:28 / 26:00).

`rom fight-matrix`: `strengths=270 unanswered=0` (it fights without brains). `rom playthrough
--seed 1` (the simulation's own bot): `hash=d20c41789bf78ef0`, ascension `ascended_tick=59544
hash=388f8f410f80705f`, as on main.

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
  one tick with one formation walk at the slowest member's speed
  (`order.pace`) when the formation `keeps_slowest`. Since task 153 a brain that
  `uses_formations` sends the whole advance to one point and lets the
  simulation lay it out (task 143); before, it gave each member its own point
  of a line it worked out itself.
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
faction, picked by the seed, start on one fair generated map (`generate_fair`) with the victory rules
on (capital elimination, conquest by default). On odd seeds the second
profile takes the first start. A drawn map whose starts are unfair — one holds
less than three quarters of the other's land, or a capital has no room for a
gather site on a node terrain its start sites draw from — is replaced by
another map of the same seed (`maps=` in the output, at most sixteen). Each
match runs until a team wins or the tick limit; output is one line per seed
(players, winner, rule, tick, the age each side ended in, hash, and each side's gather sites
at the end) and the win-rate table; matches run on `--jobs` threads (default: up to four), each
deterministic on its own. The sites read `F5:5/4 T2:14/12 M1:7/7 K1:7/7`: per resource
(abbreviation) the finished gather sites, their slots and the workers in them; `--trace`
prints the same for each player at every trace line.

Territory is off by default: on the two-player map the first city of a
central start already holds half the land, so the territory timer decides
the match before an army can.

## First attack on a person (task 67)

```sh
rom first-attack --db target/rules.db --seeds 1..8 [--difficulties easiest,easy,...] \
    [--profile balanced] [--ticks 72000] [--defenders 8] [--jobs 8]
```

Player one stands in for a person: no brain, its starting city and builders and a fixed
defence (`--defenders` soldiers of the cheapest first-age fighter of its faction, in a
ring beside the city, placed before the first tick). Player two is a computer of the
difficulty under test on the duel's fair map. Each run stops when a building of the
stand-in first loses hit points, or at the tick limit; the table gives, per difficulty,
when the main army or the raiders first set off for one of the stand-in's buildings and
with how many units, when a building was first hit and how many of the computer's
fighters stood near the stand-in's capital then.

## Tests

- `crates/rom-ai/tests/managers.rs` — each manager on a small hand-built world.
- `crates/rom-ai/tests/determinism.rs` — the same match twice; a brain
  restored in mid-match, and one restored in mid-war, issues the same orders.
- `crates/rom-ai/tests/military.rs` — taking a known city, waiting for the
  attack ratio, retreating only where the difficulty allows, the home guard,
  scouting, formations.
- `crates/rom-ai/tests/space.rs` — claiming resource bodies and founding a
  colony with a transport in a small universe, craft climbing to orbit and
  boarding a carrier, the space-age brain twice; a bombarded player builds
  ground batteries, a strike fleet fires on the batteries first (task 75).
- `crates/rom-cli/src/duel_cmd.rs` — a short duel twice with equal hashes.
- `crates/rom-ai/tests/governor.rs` — a governor keeps fifteen sites in its colony's area and
  rebuilds them, twice alike; paused it waits, without means it names the price; a brain in the
  abundance age founds a colony and turns a paused governor back on (task 73).
- `crates/rom-ai/tests/navy.rs` — on a strait both sides train warships and keep them out of
  the armies on land; against enemy line ships the class chosen sinks them at equal cost; the
  same sea match twice gives equal hashes; `sea_duel_report` (ignored) plays the 10-seed sea
  duel of task 97.
- Task 152: `space.rs` `a_lost_spaceport_is_rebuilt` (a destroyed spaceport goes up again where it
  stood, twice alike); `managers.rs` `an_automation_age_start_builds_energy_sites` (a reactor and
  energy income, twice alike); `military.rs` `a_raided_city_calls_up_militia_and_stands_them_down`
  (at most half the workers, never in an army, all stood down once quiet, twice alike).
- Task 153: `military.rs` `an_advance_marches_in_formation_with_the_ranged_behind` (one point,
  a formation with `ranged_behind`, the ranged members' places behind the close fighters', twice
  alike) and `a_walled_in_capital_does_not_eat_the_order_budget`; `space.rs`
  `a_fleet_travels_in_formation` (line abreast, a place of its own for every ship, twice alike);
  `formations.rs` unit tests of the choice.
- `crates/rom-ai/tests/ages.rs` — the second age in the ordinary run; every
  profile to the third age on seeds 1–5 with
  `cargo test --release -p rom-ai --test ages -- --ignored`.

## Fortifications and minefields (task 147)

Part 147a added walls, gates and minefields to the rules and the simulation only
(`docs/database.md`, "Fortifications and minefields"). The computer player has no code for them
yet: its construction manager scores the first building of a line it has none of at 220 when
nothing better waits, so it may lay one wall segment, one gate and one mine in a match (each kind
is one upgrade line, so never more while they stand). No data column keeps a constructible
building out of its catalogue without keeping it from people too; part 147b teaches it to lay
lines (never walling itself in) and minefields, and to break or go round an enemy's.

Measured (20 seeds, balanced:moderate against balanced:easiest): 18–0 with 2 draws (seeds 5 and
14), mean tick 52052. The same rules without the construct rows of the seven new types give 20–0,
mean tick 52405, as before the task: the draws come from the players' one palisade and gate (their
first wave in seed 5 is lost after the match has drifted), not from the simulation.
