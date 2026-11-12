# Game database

Files: [`../db/schema.sql`](../db/schema.sql) (84 tables, 4 views) and
[`../db/seed.sql`](../db/seed.sql) (a small placeholder dataset that exercises
every part of the schema). Both load cleanly in SQLite ≥ 3.37:

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
     │                 ├─ et_trader        trades, claims rare resources
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
| trades for gold, claims rare resources | `et_trader` |
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
| Holding Relics speeds research | source `rare.relics`, stat `research_time`, `pct -33` |

`stat` is a registry: a modifier naming an unknown stat fails the foreign key,
which catches typos in data.

### 5. Tags instead of flag letters

Targeting uses tags (`infantry`, `mechanical`, `biological`, `drone`, `city`).
Combat effectiveness uses a small `damage_class × armor_class` matrix instead
of a full unit-by-unit table, so adding a unit never requires touching every
other unit.

### 6. One map: planets set in space

The whole map exists from the first tick. It contains:

- **Planetary terrains** — large areas of land and sea where cities stand and
  ground battles happen. One is the shared home terrain; a map style may add
  more to colonise.
- **Open space** between them.
- **Resource planets** — small `node` entities in space. `claim_mode =
  'presence'`: whoever keeps a unit tagged `starship` within `claim_radius`
  owns the node and receives its income. No building is needed.

`layer` is the movement medium: `ground`, `fluid` (sea), `subfluid`, `air`,
`orbit`, `space`. Terrain belongs to a layer and a `movement_class` lists the
terrain it can cross, so vehicles that fly, dive or reach orbit are just
classes with more rows. Sea and space use the same rule set (port, vessels,
transports, trade lanes, first colony at the landing edge) on different
layers, which lets both exist on one map.

`map_region` names each area and gives it a **tier**; the `region` grid maps
tiles to regions. City limits and "first colony" rules are evaluated per
region.

**Players reach space one by one.** Each age belongs to a tier
(`age.map_scale`). A player's camera limit, visibility and reach are those of
*their own* current age: before their space age they see and act only within
the home terrain; on reaching it their view zooms out and space is revealed.
Others stay confined until they get there, so the first arrival claims
resource planets and colony sites unopposed. Nothing is regenerated and no
state is converted — it is purely a per-player reveal.

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
or in space. The result is stored once: whole-map arrays in `map_grid`, the
regions in `map_region`, and every deposit, forest stand, rare resource and
resource planet as an `entity` row of class `node`. Meshes and materials
flagged `baked` are produced before the first tick. Nothing is generated
while the match runs.

### 9. Computer opponents are data too

`ai_profile` + `ai_profile_weight` (personality), `ai_opening_step` (opening
list) and `ai_difficulty` (thinking speed, order budget, reaction time,
mistakes). A computer player acts only by adding rows to `order`, like a
human. Its memory is saved in `player_ai_state`. See
`research/05-computer-opponents.md`.

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
- **Balanced content** — the seed has placeholder numbers and ~13 entity types.

## Decided

- One map holding planetary terrains, open space and resource planets;
  each player's view widens when that player reaches the space age (section 6).
- Resource planets are claimed by starship presence; planetary terrains host
  buildings and ground battles.
- Identity ladder: historical nation → continental bloc → united humanity
  (section 7).
- The game ends with the Galactic Empires age. The intergalactic stage and
  the two non-human races are **parked**: no clean transition was found. The
  schema needs no change to add them later (a third tier, two `race`
  factions, optional `faction_transition` rows).
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
