# Game database

Files: [`../db/schema.sql`](../db/schema.sql) (89 tables, 4 views) and
[`../db/seed.sql`](../db/seed.sql) (the full generated content catalogue and hand-written
world rules). Both load cleanly in SQLite ≥ 3.37:

```sh
sqlite3 game.db < app/db/schema.sql && sqlite3 game.db < app/db/seed.sql
sqlite3 game.db "PRAGMA foreign_key_check; SELECT * FROM lint_def_without_subtype; SELECT * FROM lint_unproducible;"
# all three must print nothing
```

## How the database is used

| Half | Tables | Lifetime | Used by |
| --- | --- | --- | --- |
| **Rules** (Part A) | `def` and everything hanging off it | shipped with the game, read-only at runtime | loaded once into flat in-memory arrays indexed by `def.id` |
| **State** (Part B) | `match`, `player`, `entity`, `order`, … | one file per save | written on save, read on load; also the debugging view of a match |

The simulation never runs SQL inside a tick. SQL is the authoring format, the
save format and the inspection tool; the tick loop works on Rust structs built
from it. In Rust, use `rusqlite` with the `bundled` feature so no system
library is needed on any platform.

Units of measure: time in **ticks**, distance in **milli-tiles**,
percentages as integers (100 = 100 %), resource amounts as plain integers.
Integers only, so the simulation is deterministic (needed for replays and
lockstep multiplayer).

**Tick rate and game speed.** The simulation step is fixed at 20 ticks per
game second and never changes. Game speed is a percentage (`match.speed_pct`,
default 100): the loop runs `20 × speed_pct / 100` ticks per real second, so
200 % plays twice as fast with identical results. The player picks a default
speed in the settings screen and can change it during a solo match; in
multiplayer it is a match option set by the host. Rendering interpolates
between ticks, so it stays smooth at any speed.

## The model in one page

```
def ─┬─ age            ordinal, era, map scale
     ├─ faction        nation → race, transitions ("evolve")
     ├─ resource       base | rare (+ resource_yield)
     ├─ terrain        layer, buildable, …
     ├─ entity_type ───┬─ et_movement      moves
     │  (unit, building├─ et_weapon        attacks          (+ et_weapon_target)
     │   node, pickup, ├─ et_builder       builds, repairs
     │   wildlife)     ├─ et_harvester     harvests         (works at an et_gather_site)
     │                 ├─ et_healer        heals            (priest, medic, repair drone)
     │                 ├─ et_trader        runs city-to-city delivery routes
     │                 ├─ et_keeper        keeps carried nodes
     │                 ├─ et_container     garrison / transport / carrier
     │                 ├─ et_producer      takes train + research orders
     │                 ├─ et_territory     city, borders
     │                 ├─ et_energy        ability pool
     │                 ├─ et_supply        supply radius
     │                 ├─ et_gather_site   farm, mine, camp, university
     │                 └─ et_node          deposit placed by the map generator
     ├─ tech           age | track | upgrade | unit_upgrade | government | project | evolution
     ├─ ability        active abilities
     ├─ formation, stance
     ├─ material, mesh procedural look
     ├─ map_style      generator recipe
     └─ victory

cross-cutting (all keyed by def.id):
  production     who can make / research / construct what   (= one command-panel button)
  cost           what it costs, plus ramp per item already owned
  prerequisite   what must exist first (AND of OR-groups)
  availability   which factions may have it
  def_tag        tags used for targeting
  modifier       every numeric bonus in the game
  tech_effect    every non-numeric result of a tech
```

### 1. One catalogue (`def`)

Every definable thing has exactly one row in `def` with a stable text `key`
(`unit.citizen`, `tech.religion`). Subtype tables reuse that id as their
primary key. Because of this, **costs, prerequisites, tags, availability,
modifiers and translations work identically** for a unit, a building, a
technology or an ability — one table each instead of one per kind.

### 2. Capabilities are components

`entity_type` holds only what everything has (hit points, armour, footprint,
build time). What an entity can *do* is the set of `et_*` rows it owns:

| The request said | Component |
| --- | --- |
| moves | `et_movement` |
| attacks | `et_weapon` |
| builds, repairs | `et_builder` |
| harvests | `et_harvester` + the building's `et_gather_site` |
| heals | `et_healer` (target tag *biological* → priest/medic, *mechanical* → repair drone) |
| runs wealth delivery routes | `et_trader` |
| keeps carried nodes | `et_keeper` |
| takes production and research orders | `et_producer` |
| is a resource | `et_node` |

A unit and a building differ only by `class` and by which components they
carry. A carrier that builds drones, a mobile temple, or a spaceport that
moves are data changes, not new code.

### 3. The economy is an order book

Every request is a row in `order`: a unit's command queue and a building's
production queue are the same table, ordered by `seq`. An order names its
`kind` (from `order_kind`), its executor, and optionally a subject definition
(what to train/research/construct/cast) and a target (entity or point).

Life cycle: `queued → active → done`, or `blocked` with a reason
(`pop_cap`, `no_path`, `no_slot`), or `cancelled`. Costs are charged when the
order is accepted and recorded in `order_payment`, so a cancel refunds exactly
what was paid even if prices changed since. `order_kind.needs` names the
component the executor must have, so invalid orders are rejected by data.

### 4. Every bonus is a `modifier`

The reference game hard-codes several hundred named bonuses (one per nation
power, wonder, rare resource and government), each needing its own engine
code. Here they are all rows of one table:

```
source_def  →  applies_to (owner/allies/enemies/everyone)
            →  target (tag, exact def, or none for player-level stats)
            →  stat  →  op (add | pct | mul | set)  →  value
            +  optional aura radius, condition, duration, stack group
```

Evaluation order for a stat: `set`, then sum of `add`, then
`× (100 + Σ pct) / 100`, then the product of every `mul / 100`.

A source is active when its owner *has* it: a researched tech, the current
age, the player's faction, a claimed rare resource, a completed wonder, a
nearby aura unit, an ability effect that has not expired.

Examples in the seed:

| Design intent | Row |
| --- | --- |
| Age of Abundance: everything is free, build times stay | source `tech.age.abundance`, stat `cost.all`, `mul 0` |
| Religion makes temples push borders | source `tech.religion`, tag `temple`, stat `border_radius`, `add 2000` |
| Commerce adds a route slot | source `tech.commerce.1`, stat `caravan_limit`, `add 1` |

`stat` is a registry: a modifier naming an unknown stat fails the foreign key,
which catches typos in data.

### 5. Tags instead of flag letters

Targeting uses tags (`infantry`, `mechanical`, `biological`, `drone`, `city`).
Combat effectiveness uses a small `damage_class × armor_class` matrix instead
of a full unit-by-unit table, so adding a unit never requires touching every
other unit.

### 6. The universe: star systems, bodies and routes

Decided on 2026-11-13; see `world-model.md`. Every planet is a cube sphere,
planets belong to star systems, and systems are joined by routes.

| Table | Half | Holds |
| --- | --- | --- |
| `body_kind` | rules | the vocabulary `star`, `habitable`, `resource` |
| `map_style` (universe knobs) | rules | neutral systems, habitable planets and resource bodies per system (each a `_min`/`_max` pair), planet face size `planet_n_min/median/max` (64..=512), `route_extra_pct`, `star_map_radius` |
| `map_style_feature` with `region_kind = 'resource_body'` | rules | what resource bodies yield: a `node` type with `claim_mode = 'presence'`; `chance_pct` weights the kinds, and a `per_player` row puts `clumps_min` bodies of its kind in every home system |
| `star_system` | state | name and star-map position (integers, display only) |
| `body` | state | the star (orbit radius 0), habitable planets (`n`, `home_race_id`) and resource bodies (`node_entity_type_id`), each on its own orbit (radius, angle in millidegrees) |
| `route` | state | a lane between two systems (`system_a < system_b`) and its integer length |
| `entity.body_id`, `entity.face` | state | an entity's location on a habitable body's surface at `(face, x, y)`; no body = the flat map. Orbit, system-space and transit locations come in world-model step 4b |

The generator (`rom-mapgen`, integers only) gives each race present one home
system holding one home planet at the median size, shared by all of that
race's players, plus neutral systems as the knobs say. Home systems sit as far
apart as practical; other systems keep a minimum separation. Routes are a
minimum spanning tree plus `route_extra_pct` more of the shortest
non-crossing edges, and every home system has at least two routes when there
are more than two systems. **Fairness:** every home system holds the same
habitable planets and resource bodies, and the resource-body value within two
routes (the summed `et_node.income`, not counting other races' home systems)
of the poorest home system is at least 80 % of the richest one's.

**Resource bodies** are small `node` entities: `claim_mode = 'presence'`, so
whoever keeps a unit tagged `starship` within `claim_radius` owns the body and
receives its income. No building is needed.

`layer` is the movement medium: `ground`, `fluid` (sea), `subfluid`, `air`,
`orbit`, `space`. Terrain belongs to a layer and a `movement_class` lists the
terrain it can cross, so vehicles that fly, dive or reach orbit are just
classes with more rows.

**Players reach space one by one.** Each age belongs to a tier
(`age.map_scale`). A player's camera limit, visibility and reach are those of
*their own* current age: before their space age they see and act only on
their home planet; on reaching it their view zooms out to the system and the
star map. Others stay confined until they get there, so the first arrival
claims resource bodies and colony sites unopposed. Nothing is regenerated and
no state is converted — it is purely a per-player reveal.

`age.view_range` (milli-tiles) is the largest ground half-width the camera may
show in that age while the view is still bound to the home planet, measured
across the screen at the point looked at (0.6 × the eye distance, whatever the
tilt): 30 tiles in the first age (the home city and its border), 78 in the
Information Age (about a third of a 192-tile planet's face), `0` (the whole
globe) from the Fusion Age on.

A flat rectangular map remains for tests and skirmish maps: `map_style.planets`
and `space_pct` and the `planet`/`space` feature rows drive it, and `map_grid`
holds its arrays.

### 7. Ages, nations and races

`age.ordinal` orders the 14 ages (0–13); `age.map_scale` says which kind of map that
age is played on. Advancing is an ordinary `tech` of kind `age` with a
`set_age` effect.

`faction` is an identity ladder, recorded in `faction_transition`:

| Step | Kind | When | Forced |
| --- | --- | --- | --- |
| nation — a historical people (Romans, Egyptians, Chinese, Aztecs …) | `nation` | historical ages (0–7) | — |
| continental bloc (Africa, Americas, Asia, Europe, Oceania) | `bloc` | after the contemporary ages (8–9) | yes |
| united humanity | `race` | on reaching space (10+) | yes |

A faction is a roster and a set of bonuses, not a side. Several players can
be human and still be at war; sides come from `player.team` and `diplomacy`.
Moving a step to another age is a one-row change in `prerequisite`.

### 8. Map and resources are generated before play

`map_style` + `map_style_feature` describe what the generator scatters.
`map_style_feature.region_kind` says whether a feature belongs on a planet
surface, in a flat map's open space, or on a star system's resource bodies.
The result is stored once: the universe in `star_system`, `body` and `route`;
flat-map arrays in `map_grid`; and every deposit, forest stand, rare resource
and resource body as an `entity` row of class `node`. The generator takes one
faction per player and gives each race present its own home system and home
planet; `per_player` features are placed around each start, `scattered`
planet features on every planet.
The rules loader resolves names through `Rules::display_name`
(`faction_alias`, nearest ancestor first) and races through `Rules::race_of`. Meshes and materials
flagged `baked` are produced before the first tick. Nothing is generated
while the match runs.

### 9. Planet terrain coverage

Sphere styles have `polar_cap_pct INTEGER NOT NULL DEFAULT 20` (0–50, share
of each hemisphere's latitude), `start_latitude_max_mdeg INTEGER NOT NULL
DEFAULT 60000` (0–90,000), and `polar_terrain_id REFERENCES terrain(id)`.
A style with nonzero caps must reference unbuildable, non-sight-blocking
ground terrain. Seed styles reference `ter.ice`: layer ground, not buildable,
no sight blocking, material `mat.ice` with `base_colour` `#e8eef2`. Movement
rows copy the plain's ground classes at 50 % speed; ships have no row.
Ice replaces sea as well as land above the noisy cap boundary. Flat maps
ignore all three knobs. See `world-model.md`, “Planet surface”.

For a planet terrain feature with `coverage_pct > 0`, the sphere's target tile
count is `land_tiles × coverage_pct / 100`, counting only non-ice land before features
are placed (including protected start tiles). With `coverage_pct = 0`, it is
`avg(clumps_min, clumps_max) × avg(size_min, size_max) × players × tiles / 65,536`,
with the player factor replaced by 1 for rows other than `per_player`;
integer arithmetic retains half-integer averages until the final division.
The last unbuildable, sight-blocking ground terrain in style-row order follows
highest elevation, while other land terrains follow an independent seamless
3D moisture field, preferring the middle 60% of land elevations. Every start's
3-tile disc stays clear. Missing per-player terrain is grown in the distance
annulus scaled by `sqrt(tiles / 65,536)` (each radius capped at 40 tiles),
with a minimum outer radius of 4 on small worlds to clear the protected disc,
exchanging low-ranked global tiles to preserve coverage; on very small planets
start guarantees can exceed the budget. Scattered sites scale a rolled flat
count by the same area ratio, never below that flat count, and stay at least
12 tiles apart on valid land. The loader reports nonzero `coverage_pct` on
entity features or features outside `region_kind = 'planet'`. Flat generation
ignores `coverage_pct` and retains its original rules.

### 10. Computer opponents are data too

`ai_profile` + `ai_profile_weight` (personality), `ai_opening_step` (opening
list) and `ai_difficulty` (thinking speed, order budget, reaction time,
mistakes). A computer player acts only by adding rows to `order`, like a
human. Its memory is saved in `player_ai_state`. See
`research/05-computer-opponents.md`.

## Content generator

`db/content/generate.py` holds the roster, technology tracks and integer stat
formulas specified in `content.md`. Run `python3 db/content/generate.py` from
`app/` after editing those tables. It replaces only the marked generated block
in `db/seed.sql`; the game continues to load that seed directly. Run
`python3 db/content/generate.py --check` to detect stale generated SQL without
writing anything; CI runs the same check. The generator needs only Python 3's
standard library. Hand-written relationships to generated definitions follow
the block so their references resolve during loading.

## Adding content (checklist for implementers)

**A new unit**
1. `def` row (`kind='entity_type'`), then `entity_type`.
2. Components: at least `et_movement`; `et_weapon` (+ `et_weapon_target`) if it fights.
3. `def_tag` rows (body type and role — healers and modifiers depend on them).
4. `cost`, `prerequisite`, and a `production` row on the building that trains it.
5. Optional: `availability` (faction-only), `replaces_id` (unique variant), `upgrades_from_id`.
6. Run the two lint views.

**A new technology**: `def` + `tech`, `cost`, `prerequisite`, a `production`
row with `mode='research'`, then `modifier` rows for numbers and `tech_effect`
rows for unlocks.

**A new bonus of any origin**: only `modifier` rows. If it needs a stat that
does not exist, add the stat to `stat` and implement that one stat in code.

## What is deliberately not here

- **User settings** (graphics, sound, hotkeys, profiles) — per-user config
  file, not game rules. See `research/04-hud-and-screens.md`.
- **Scenario scripting / triggers** — later.
- **Final balance** — generated stats are first-pass values for playtesting.

## Decided

- A universe of star systems joined by routes; each player's view widens
  when that player reaches the space age (section 6, `world-model.md`).
- Resource bodies are claimed by starship presence; habitable planets host
  buildings and ground battles.
- Identity ladder: historical nation → continental bloc → united humanity
  (section 7).
- **Races are chosen before the match.** Each race starts on its own home
  world, develops equivalent technology under its own names and look
  (`faction_alias`, `entity_visual`), and meets the others only in space.
  Races differ through substitutes (`replaces_id`), `availability` and
  `modifier` rows, not through separate rule sets.
- The game ends with the Galactic Empires age. Travel between galaxies stays parked.
- Fixed 20-tick simulation; game speed is a user-settable percentage.

## Naming rule

No names taken from other games or from other fiction. Allowed: historical
peoples, real geography, ordinary military and scientific vocabulary, and
names invented for this project. The united-humanity faction is called
"Humanity" for now: the common science-fiction word for Earth people is also
a faction name in one of the reference games, so it is avoided.

## Open decisions

1. Names of ages 8–13.
2. The nation list. The seed has 14 as a starting point.
3. Map size cost: a map that includes space is much larger than a planet-only
   map. Pathfinding and fog grids for the space part can be coarser; to be
   measured.

## Carried nodes and trade routes

`et_node.claim_mode` also accepts `carry`: only a unit with the node's
`claim_tag_id` may collect it. The `carry` movement order targets that node
and delivers it to the nearest own completed `et_keeper`. `et_keeper` has an
entity-type primary/foreign key and a positive `capacity` measured in nodes.
`entity.container_id` records custody by a unit or keeper; contained nodes
leave spatial indexes. Keepers contribute each node's `resource_id`/`income`
to their owner's ordinary, modified and capped periodic income. The new role
tag `clergy` authorizes priests. Relics now yield wealth; the obsolete rare
resource, its resource-yield rows and research modifier have been removed.

`et_trader.route_income` is a delivery amount per `trade.route_unit` of the
accepted city-centre distance, paid at arrivals rather than per income period.
`trade.route_unit` is 10000 milli-tiles; `trade.min_route` is 20000 (the value
of `city.spacing`). `player.caravan_limit.base` is 2; `caravan_limit` modifiers
add route slots. Pending route orders reserve slots. Orders retain the home,
target, accepted length, delivery resource/amount and current leg in memory;
the existing `trade_route` table is unchanged (save support is a later task).
