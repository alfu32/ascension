# Computer opponents

## What the reference does

- Three ready-made behaviours offered in the player slot: a default, a
  "rush and raid" one and a "boom and build" one; six difficulty levels; a
  separate per-player handicap.
- The opening is a **script**: a numbered list of steps ("have N citizens",
  "place a barracks", "research X"), each either blocking (wait until
  affordable) or skippable, with one parameter sliding between economy-first
  and army-first and a few random variations. The script is abandoned when a
  city is attacked.
- Everything after the opening lives inside the engine.
- Units already look after themselves through stances and auto-behaviours
  (idle workers find work, siege deploys itself, abilities auto-cast). The
  computer player benefits from these exactly as a human does.

## Principles for ours

1. **Same interface as a human.** A computer player reads the game state it is
   allowed to see and writes rows to the order book. There is no second path
   into the simulation, so every rule (costs, queues, population cap, fog of
   war) applies automatically, and anything a computer player can do can be
   replayed and inspected.
2. **Deterministic.** It runs inside the simulation on fixed ticks with its
   own seeded random generator. Same seed, same match — required for replays,
   multiplayer with computer players, and automated tests.
3. **Honest by default.** Difficulty is expressed as *speed, attention and
   judgement*: how often it thinks, how many orders per minute it may issue,
   how fast it reacts, how often it picks the second-best option. An income
   bonus exists only on the top level; ignoring fog of war is available as a
   switch but used by no standard level.
4. **Data-driven personality.** Profiles are rows of weights and an opening
   list (`ai_profile`, `ai_profile_weight`, `ai_opening_step`). The decision
   code is one program for all profiles, nations and ages.
5. **No knowledge of unit names.** It reasons from the rules database: what a
   building produces, what counters what (damage × armour matrix), what a
   tech unlocks. New content needs no new decision code.
6. **Bounded cost.** Each computer player gets a small time slice per tick and
   spreads its work over many ticks. Eight players must not stall the game.

## Structure

```
                 ┌─────────────────────────────────────────────┐
  perception →   │ what I know: own state, explored map, last   │
                 │ seen enemies, threat and territory maps      │
                 └──────────────────────┬──────────────────────┘
                                        ▼
  strategy    (every few seconds)  posture: expand | boom | arm | attack | defend | recover
                                        ▼
  managers    economy · construction · research · production · military · scouting · diplomacy
                                        ▼
  orders      rows in the order book, limited by the orders-per-minute budget
                                        ▼
  units       stances, formations and auto-behaviours shared with human players
```

### Perception

- Own counts by type, income rates against the commerce cap, free gather
  slots, population against the cap, queues.
- **Memory of enemies**: last seen position, type and time for every enemy
  entity observed; entries fade. This is what makes scouting matter.
- **Influence maps** on a coarse grid (one cell ≈ 8×8 tiles), updated a few
  rows per tick: own strength, known enemy strength, territory owner,
  resource value. Used for where to build, where to defend, where to attack
  and which route avoids danger.

### Strategy

A small set of postures chosen by scoring, re-evaluated every few seconds and
immediately on big events (city attacked, age reached, army lost). The
profile weights bias the scores; the scores come from the game state:

| Posture | Chosen when |
| --- | --- |
| expand | city limit allows another city and a good site is known |
| boom | safe, below worker target or commerce cap not reached |
| arm | enemy strength unknown or rising, army share below target |
| attack | own strength ≥ `mil.attack_ratio_pct` of known enemy strength |
| defend | enemy force inside own borders |
| recover | army lost, or income collapsed |

### Managers

- **Economy** — keeps the worker share; fills free gather slots, preferring
  resources below their cap and those the current plan needs; builds gather
  and enhancer buildings; moves workers off a capped resource. In the
  automation ages gather sites run unattended and this manager shrinks to
  placement only.
- **Construction** — picks sites from the placement rules plus the influence
  maps: civil buildings inside city radius away from the front, military
  buildings toward it, towers on approach routes, new cities at the best
  unclaimed site that the borders and city-spacing rules allow.
- **Research** — scores every available tech by what it unlocks or improves
  for the current posture; the age advance is weighted by `tech.age_priority`.
- **Production** — decides army composition. For each enemy type seen,
  it looks up which of its own producible types deal the most damage for
  their cost (damage matrix, range, speed) and builds a mix; without
  information it builds a balanced default. Respects ramping costs by
  spreading across types.
- **Military** — groups units into armies with a role: *main*, *home
  guard*, *raiders*, *escort*. Each army has a small state machine:
  gather → advance → engage → retreat → regroup. Targets are picked by value
  over danger. Retreat when the local strength ratio drops below
  `mil.retreat_ratio_pct`. Uses formations (melee front, ranged and healers
  behind, siege with escort), garrisons wounded units to heal, keeps supply
  units with armies in enemy territory.
- **Scouting** — keeps `scout.units` exploring unexplored and stale areas;
  claims pick-ups; finds rare resources for merchants.
- **Diplomacy** — in games with changing alliances: proposes peace when
  losing on two fronts, alliance against the leader, answers tribute demands
  by comparing strength.
- **Space** (age 10 on) — sends a starship to each reachable resource planet
  by value over distance, escorts claimed ones, picks colony sites on other
  planetary terrains. Because players reach space one by one, a computer
  player that gets there first will grab planets exactly as a human would.

### Opening book

`ai_opening_step` rows: "have at least N of X" for train / construct /
research, plus assign workers, scout, found city. Blocking steps wait for
resources. The book ends when the list is exhausted or the player is
attacked; then the planner continues from the actual state. This gives
competent, varied early games without making the planner clever enough to
discover a build order.

## Difficulty

| Level | Thinks every | Orders / min | Reaction | Mistakes | Formations, abilities, retreat | Bonus |
| --- | --- | --- | --- | --- | --- | --- |
| Easiest | 5 s | 20 | 10 s | 30 % | none | — |
| Easy | 3 s | 40 | 6 s | 20 % | retreat only | — |
| Moderate | 2 s | 80 | 3 s | 10 % | all | — |
| Tough | 1 s | 160 | 1.5 s | 3 % | all | — |
| Tougher | 0.5 s | 300 | 0.5 s | 0 | all | — |
| Toughest | 0.5 s | 300 | 0.5 s | 0 | all | +25 % income |

Values are in `ai_difficulty`; the income bonus is applied as an ordinary
modifier so it shows up in the same places as any other bonus.

## Neutral and scripted owners

- **Nature** (player 0): wildlife wanders; no decisions.
- **Raiders / survival waves**: a profile with no economy manager and a
  spawn schedule — later, with scenarios.

## Why this matters beyond opponents

Two computer players in a match with no window is the main **test harness**
for the whole simulation: it exercises every system for thousands of ticks,
must end with a winner, must produce identical state hashes for the same
seed, and gives win-rate tables for balancing units and nations. The plan
builds this harness early.

## Saved state

`player_ai_state` stores the profile, difficulty, random-generator state,
opening position and a versioned binary snapshot of the memory (enemy
sightings, armies, current posture), so a loaded game continues identically.

## Not planned

Learning systems or anything trained offline. The approach above is
predictable, debuggable and cheap, and its quality is set by data that can
be tuned with the test harness.
