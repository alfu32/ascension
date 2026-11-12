# Reference data model

Study of the rule files of the primary reference game ("Reference A", a
history-spanning RTS). Two secondary references ("B": sci-fi RTS with medics,
"C": medieval RTS with priests) contribute one unit role each and were not
studied in depth. Game names are intentionally not used in this repository.

Everything below was read from the local install's data files; nothing is
copied into this repository beyond structural facts.

## 1. Rule tables found

| Table | Rows | What it is |
| --- | --- | --- |
| units | 364 rows / 300 names | one flat record of 54 columns per unit |
| buildings | 129 rows / 64 names | 50 columns; house/office variants repeat |
| technologies | 85 | ages, 4 research tracks, building upgrades, governments |
| resources | 6 base + 44 rare | base have exchange rates; rare have 2 yields + 1 special bonus |
| abilities | 55 | includes pure UI commands (declare war, pack, raze) |
| formations | 10 | line, column, wedge, square, echelon L/R, envelop, refused, sparse, mob |
| terrain types | 9 | each lists what it yields |
| nations | 24 | name pools + art set; powers live elsewhere |
| constants | 723 | global tunables **and** every nation/wonder/rare bonus |
| tech bonuses | 122 | text descriptions keyed to a tech |
| balance matrix | 291 × 291 | attacker-vs-defender damage % per unit pair |
| map styles | 23 | terrain clump rules per map type |
| game setup options | 41 lists | start age, pop limit, rush rules, victory types … |
| player options | 330 commands | every bindable action |
| script functions | 626 in 56 groups | scenario scripting API |

## 2. How the reference models things

- **Unit record** — combat (attack, hits, armour, min–max range, recharge,
  splash radius/%, accuracy and its falloff per tile, projectile speed),
  movement (speed, turn rate, domain land/sea/air), economy (cost, ramp cost,
  job time, pop), tech (3 prerequisites, upgrade-from / upgrade-to, unique
  substitute for another unit), shape (spacing, block radius, figures per
  unit, crew), UI (button grid x/y, icon).
- **Flags as letters** — two masks: class letters shared with buildings
  (vehicle class, weapon class, operational type) and lower-case behaviour
  flags (garrisonable, cloaked, fires while moving, destroyed on attack …).
- **Where made** — each unit and tech names the building that produces it.
- **Costs** — text like `8m/8g`, multiplied by a global factor of 10.
- **Ramping cost** — each extra unit of a type (or of a production group such
  as "all barracks units") costs more and takes longer, with caps by category.
- **Time** — 15 simulation frames per second; all durations in frames.
- **Distance** — tiles, with finer sub-units for speed (1/192 tile) and
  spacing (1/16 tile).
- **Building record** — footprint, hit points, garrison capacity, optional
  weapon, line of sight, plunder value, placement flags (land/sea, inside or
  outside city radius, one per city), wonder points.
- **Resource economy** — citizens fill *slots* at gather buildings; no
  carrying back and forth. Slots scale with nearby trees/mountains. Income is
  a rate per period, limited by a per-player **commerce cap** (knowledge is
  exempt). Enhancer buildings multiply a city's output.
- **Territory** — cities, forts and temples push national borders. Buildings
  go inside borders; civil buildings inside a city's economic radius. Enemy
  units inside your borders take attrition.
- **Cities** — captured, not destroyed, then assimilated after a timer;
  number limited by civic research; city level grows with building count.
- **Four research tracks** — military (pop cap), civic (city limit, borders),
  commerce (commerce cap, caravans), science (cheaper/faster research, line
  of sight). Ages need a number of track techs first.
- **Special units** — scholar, caravan, merchant (claims a rare resource),
  general (auras + abilities), spy, supply wagon, scout.
- **Stances** — combat (aggressive, defensive, stand ground, raid, raze, hold
  fire), worker (gather, build & gather, build, idle), ability use
  (auto/manual), deploy (auto/manual).

## 3. Orders found (complete list)

move, follow, attack, attack-move, explore-move, flee, patrol, guard, forward,
halt, garrison/heal, eject, transport/disembark, auto-explore, bombard ground,
repair/build, alarm (citizens hide) / all clear, to-arms (citizen ↔ militia),
pack / deploy, set rally point (also onto a unit or into a building),
infinite queue, air missions (strike, patrol, transfer, return, scramble,
repeat), buy/sell at market, disband, raze, face left/right/about.

## 4. Lessons taken into our schema

| Reference does | Problem | We do |
| --- | --- | --- |
| ~450 of 723 constants are named one-off bonuses, each with engine code | every new nation/wonder needs a programmer | one `modifier` table |
| Letter-flag masks | unreadable, fixed alphabet | `tag` + component tables |
| 291 × 291 unit balance matrix | grows quadratically | `damage_class × armor_class` matrix + tag modifiers |
| Separate unit and building tables with duplicated columns | flying/moving buildings impossible | one `entity_type` + components |
| Prerequisites as 3 fixed columns | cannot express "A or B" | `prerequisite` with OR-groups |
| Nation availability as a 24-bit mask | fixed nation count | `availability` rows |
| Costs as strings ×10 | parsing, hidden factor | `cost` rows, plain integers |
| Unique units duplicated per art variant | 364 rows for 300 units | `replaces_id` + `entity_visual` variants |
| Land/sea/air domain enum | no room for space, orbit, underwater | `layer` + `movement_class` |

Kept as-is because it works well: slot-based gathering, commerce cap, ramping
costs, territory and attrition, city capture and assimilation, four research
tracks, rally points and queues, stances, formations.

## 5. Things the brief did not mention but the reference has

Worth a decision each; the schema already has room for all of them.

- Ramping costs and the commerce cap (core pacing mechanics).
- Attrition inside enemy borders and supply units that negate it.
- City capture → assimilation timer → capital elimination timer.
- Garrison: units inside buildings heal and add firepower.
- One-time resource bonus when a gather building is completed.
- Ruins / pick-ups that grant a one-off bonus to the first unit through.
- Market with drifting buy/sell prices; tribute with a transit loss.
- Wonders: unique per match, score points, each a set of modifiers.
- Governments: mutually exclusive tech pairs granting a hero unit.
- Generals (aura + forced march, ambush, decoys, entrench) and spies
  (bribe, informer, counter-intelligence, sabotage).
- Flanking and high-ground damage bonuses; entrenchment.
- Stealth and detection; pack/deploy for siege weapons.
- Aircraft based at an airbase or carrier, flying missions and returning to refuel.
- Doomsday counter for strategic weapons (ends the match for everyone).
- Victory types: conquest, capital, territory %, wonder points, economy, score, time, tech race.
- Peace/alliance diplomacy with shared borders and joint victory.
- Idle workers find work on their own after a delay.
- Game recording/replay, scenario editor, script triggers.
