# Game database

Files: [`../db/schema.sql`](../db/schema.sql) (143 tables, 4 views),
[`../db/seed.sql`](../db/seed.sql) (the full generated content catalogue and hand-written
world rules) and the model files in [`../db/models/`](../db/models/) (one per building or
unit model, task 55; see §10a and `docs/models-in-meshcraft.md`), which load after the seed:
materials, shared sub-assemblies, then models. All load cleanly in SQLite ≥ 3.37:

```sh
sqlite3 game.db < app/db/schema.sql && sqlite3 game.db < app/db/seed.sql
cat app/db/models/materials.sql app/db/models/shared/*.sql \
    $(ls app/db/models/*.sql | grep -v '/materials.sql$') | sqlite3 game.db
sqlite3 game.db "PRAGMA foreign_key_check; SELECT * FROM lint_def_without_subtype; SELECT * FROM lint_unproducible;"
# all three must print nothing
```

`rom build-rules --schema db/schema.sql --seed db/seed.sql --out rules.db` does the same
(the model files beside the seed, or `--models <dir>`) and refuses a database whose checks
fail.

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
     │                 ├─ et_healer        heals            (priest, medic, repair drone, supply cart)
     │                 ├─ et_trader        runs city-to-city delivery routes
     │                 ├─ et_keeper        keeps carried nodes
     │                 ├─ et_container     garrison / transport / carrier
     │                 ├─ et_producer      takes train + research orders
     │                 ├─ et_territory     city, borders
     │                 ├─ et_energy        ability pool
     │                 ├─ et_supply        supply radius
     │                 ├─ et_call_up       a worker called up as militia (task 141)
     │                 ├─ et_gather_site   farm, mine, camp, university
     │                 ├─ et_view_scale    drawn size in the space views (presentation)
     │                 ├─ et_inhibitor     jump inhibitor: holds and strikes enemy ships in its system
     │                 ├─ et_colony        automated colony: wealth, the sites its governor keeps
     │                 └─ et_node          deposit placed by the map generator
     ├─ tech           age | track | upgrade | unit_upgrade | government | project | evolution
     ├─ ability        active abilities
     ├─ formation, stance
     ├─ material, mesh procedural look (mesh_part: parts of composed models)
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
| heals | `et_healer` (target tag *biological* → priest/medic, *mechanical* → repair drone, supply wagon/truck) |
| runs wealth delivery routes | `et_trader` |
| keeps carried nodes | `et_keeper` |
| takes production and research orders | `et_producer` |
| is a resource | `et_node` |
| tells of ships jumping in (task 118) | `et_detector` |

A unit and a building differ only by `class` and by which components they
carry. A carrier that builds drones, a mobile temple, or a spaceport that
moves are data changes, not new code.

### 3. The economy is an order book

Every request is a row in `order`: a unit's command queue and a building's
production queue are the same table, ordered by `seq`. An order names its
`kind` (from `order_kind`), its executor, and optionally a subject definition
(what to train/research/construct/cast) and a target (entity or point).

Two orders are settings rather than queue entries (task 64): `set_rally` on a
producer whose `et_producer.has_rally` is set takes effect at the next tick
boundary without replacing or joining the queue; units it trains then walk to the
point, or to the entity (a harvester to a gather site it can work harvests
there); the producer itself as the target clears it. `disband` (`needs`
`movement`) removes a unit with no refund — at once, or after the orders before it
when queued; a carrier with units aboard refuses it.

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
| `body_kind` | rules | the vocabulary `star`, `habitable`, `resource`, and `black_hole` and `celestial` for the celestial home (task 76) |
| `map_style` (universe knobs) | rules | `star_region` (which catalogue stars a match takes, tasks 89 and 109: `nearest`, `single`, `wedge`, `all`), `systems_min/max` (a nearest region's systems in all), `region_min_den/max_den` (a wedge's share of the catalogue), `shared_home`, habitable planets and resource bodies per system (each a `_min`/`_max` pair), planet face size `planet_n_min/median/max` (64..=512), `route_extra_pct`, `star_map_radius` |
| `map_style_feature` with `region_kind = 'resource_body'` | rules | what resource bodies yield: a `node` type with `claim_mode = 'presence'`; `chance_pct` weights the kinds, and a `per_player` row puts `clumps_min` bodies of its kind in every home system |
| `star_system` | state | the star's name, 3D star-map position (x, y, z integers), catalogue star and constellation |
| `star_figure` | state | constellation stick-figure lines between this match's systems (display only) |
| `constellation`, `star`, `constellation_line`, `star_cluster` | rules | the real star catalogue: 745 stars in 86 constellations with published position, computed map coordinates and galactic longitude, stick figures, and the star clusters the zoom shows (task 162) ("Star catalogue") |
| `body` | state | the star (orbit radius 0), habitable planets (`n`, `home_race_id`) and resource bodies (`node_entity_type_id`), each on its own orbit (radius, angle in millidegrees) |
| `route` | state | a lane between two systems (`system_a < system_b`) and its integer length |
| `entity.body_id`, `entity.face` | state | an entity's surface or system-plane position at `(face, x, y)`; no body = the flat map; `entity_orbit` classifies ships orbiting a body |

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

**Players unlock the camera one by one.** Positive `age.view_range` keeps the
camera on the home planet; zero unlocks continuous zoom to the system and
star map (Fusion, ordinal 9, in the current seed). The limit follows the live
player age, including research completed during a match. `age.map_scale`
continues to describe gameplay tiers.

`age.view_range` is the largest ground half-width in milli-tiles while the
camera is bound to the home planet: 60,000 in the first age, 320,000 in the
Freedom Age, 450,000 in Automation, and zero from Fusion on. These are
the ground limits passed to the camera; task 41 supersedes the earlier
separate whole-globe-only age.

**Map styles (tasks 89, 109).** `map_style.star_region` says which catalogue stars a
universe takes; everything else about a system (habitable planets, resource
bodies, routes) stays in the per-system knobs. The code reads the region kind and
its numbers, never a style key. The launchers list the styles by `def.sort`.

| Style (key) | `star_region` | Systems |
| --- | --- | --- |
| Planet (`map.one_world`) | `single`, `shared_home` 1 | one seeded catalogue star whose system holds one planet (`habitable_per_system` 1..1) that every player starts on whatever the start placement, and 3 to 5 resource bodies; no routes. The key stays, and its flat-map columns are unchanged (the playthrough's style) |
| Solar (`map.solar`) | `single` | one seeded catalogue star whose system holds every home planet (with separate worlds, each race its own planet), `habitable_per_system_min..max` − 1 more planets, and the resource bodies; no routes |
| Cluster (`map.many_worlds`) | `nearest` | a seeded star and its nearest neighbours: `systems_min..max` (10 to 15) systems in all, never fewer than one per race, within `star_map_radius` (the key stays: saves and scripts name it) |
| Sector (`map.sector`) | `wedge`, 8 / 6 | the catalogue ordered by galactic longitude (`star.glon_mdeg`, ties by id): a seeded star and the ones after it around the circle, from 1/`region_min_den` (rounded up) to 1/`region_max_den` (rounded down) of the catalogue's stars: 94 to 124 of 745; the start and count drawn by each layout attempt from the seed |
| Quadrant (`map.quadrant`) | `wedge`, 8 / 4 | the same, 94 to 186 of 745 (until task 109 one fixed galactic quadrant, 163 to 225) |
| Galaxy (`map.galaxy`) | `all` | every catalogue star: 745 systems. Held back by task 89 (the match world grew by about 12 MB a system and was killed at 8.4 GB) and put back by task 99, appended last as before: system space now costs little until it is used (`world-model.md` "Scale") |

`systems_*` and `star_map_radius` bound the `nearest` region only (the
other rows carry 1 and the catalogue's extent, 8 300); `region_*_den` the `wedge`
region only (the others carry 1). In Sector, Quadrant and Galaxy the
systems beyond the homes are mostly empty: 0 to 2 habitable planets and 2 to 6
resource bodies each. `habitable_per_system_min` may therefore be 0; a home
system always holds its home planet. In Solar and Planet every home planet has the
`per_player` resource bodies as its own moons (one ore asteroid each in the seed).
In the `wedge` and `all` regions a home candidate's density is measured over its eight
nearest neighbours (three in the nearest region) and up to 256 layouts are tried; like a
nearest region's centre, each attempt draws its own wedge from the seed.

Schema change (task 109): `neutral_systems_min/max` became `systems_min/max`, totals with the
homes included (a Cluster is 10 to 15 systems whatever the number of races; counting neutral
systems could not say that); `star_region` value `quadrant` became `wedge`; new columns
`region_min_den`, `region_max_den` (a share of the catalogue as 1/N, so the owner's "1/8 to 1/6"
is exact in integers and follows the catalogue's size at load) and `shared_home` (0/1);
`map.sector` defined last in the seed (only the definitions after it, the model files' meshes,
move up by one). Both lint views stay empty.

**Start placement (task 89).** `match_option` `start_placement`, choices
`separate` (default: each race on its own home planet, as before) and `together`
(players of different races on as few home planets as fairness allows: all on one
planet, else dealt over two, three and so on). `rom-mapgen` reads the option's
default (`StartPlacement::of_rules`) or the choice it is given
(`generate_universe_placed`); a home planet is marked with the race of its first
player. Ordinary rows, no schema change.

Schema change: `map_style.star_region` (default `nearest`, so the earlier rows
keep their behaviour) and `star.glon_mdeg`; `habitable_per_system_min` may be 0
(the validator no longer asks for 1). Both lint views stay empty.

A flat rectangular map remains for tests and skirmish maps: `map_style.planets`
and `space_pct` and the `planet`/`space` feature rows drive it, and `map_grid`
holds its arrays.

### Space tiers and orbital weapons (task 43)

`map_scale` contains `planetary` (rank 0), `interplanetary` (rank 1), and
`interstellar` (rank 2). `age.map_scale` governs camera levels and ship reach;
`age.view_range` governs the planetary camera's ground half-width only.
Fusion remains rank 0 with zero range (whole globe); starfaring is rank 1
(home system); expansion and later ages rank 2 (star map and route travel).
(Since task 138 fusion is rank 1 too, and since task 155 its ships move in the home system;
see "The space ages split by distance".)
The numeric ranks are the rules contract; code does not branch on the keys.

`et_weapon.orbit_only INTEGER NOT NULL DEFAULT 0 CHECK (orbit_only IN (0,1))`
marks weapons that require the attacker to orbit the target's body. Their
`range_min`/`range_max` measure ground distance from the orbital subpoint to
the target footprint. Ordinary weapons retain their usual layer/range rules
(task 58 adds the `orbit` layer: ground → orbit and orbit → orbit).
Corvettes, space frigates, space cruisers and battleships get slot 1 explosive
bombardment weapons targeting `ground`; their original space weapons stay
in slot 0. Since task 75 these rows are generated (`orbital_bombardment()`, see
"Orbital bombardment" below) and the frigate's and later ones are kinetic area strikes.

`entity_orbit` records an orbiting entity's body, integer surface subpoint,
facing, movement remainder and last orbital shot for deterministic state and
render snapshots. The entity's underlying system-plane position identifies
the body marker; the orbit record classifies its location. `launch` is an
ordinary order against a completed own producer; training a space-only mover
on a sphere automatically queues it. `space.launch_ticks` and
`space.orbit_speed` set duration and orbital footprint speed. Universe state is saved
and restored by `rom-save` (see "Saving a match" below).

### 7. Ages, nations and races

`age.ordinal` orders the 15 ages (0–14). The seed writes ages 0–13; the content generator adds
age 14, the Galactic Federation (task 164, `LATER_AGES` in `db/content/generate.py`), with the
era, map scale, view and colonisation of age 13; `age.map_scale` says which kind of map that
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
3-tile disc stays clear. Per-player terrain is guaranteed twice around every
start, and the guarantee is a tile count, not just one tile: `avg(clumps) ×
avg(sizes)` tiles (within the planet's share per player) must exist in the
ring `start_min_dist..start_max_dist`, and the same count again in the ring
from `start_max_dist + 6` to 40 tiles (the city border radius), so expansion
finds more. Ring distances are plain tiles and do not grow with the planet
(task 52: they used to be scaled by `sqrt(tiles / 65,536)`, which put oil
19–34 tiles from a start on a 256 planet, outside a city's 16-tile economic
radius, so no well could be built). Seed values keep the near ring within the
economic radius: forest 3–13 tiles, mountain 6–14, oil 8–14 (a mine needs the
mountain tiles within 7 tiles of it for its slots; 6 before task 71). Missing tiles are grown
from the clump already there, or a new patch when it is boxed in, exchanging
low-ranked global tiles to preserve coverage; on very small planets start
guarantees can exceed the budget. Seed coverage is forest 12 %, mountain 4 %
of non-ice land (task 52, was 24 % and 7 %). Scattered sites scale a rolled flat
count by the same area ratio, never below that flat count, and stay at least
12 tiles apart on valid land. The loader reports nonzero `coverage_pct` on
entity features or features outside `region_kind = 'planet'`. Flat generation
ignores `coverage_pct` and retains its original rules.

### 10. Terrain materials and baked feature objects

Every ground and fluid terrain has a `material_id`. `material.program` picks a
shader of the material library (`grass`, `sand`, `stone`, `snow`,
`forest_floor`, `oil`, `water`, and for objects `foliage`, `bark`, `crop`); the
renderer draws it up close and blends to the map colour across a band of camera
distances. `material.params` is a flat JSON object:

| Param | Meaning |
| --- | --- |
| `colour`, `colour_alt` | the two tones of the near look (`#rrggbb`) |
| `scale` | pattern size in tiles |
| `base_colour` | map colour of a feature terrain (far out) |
| `shore_material` | material of this terrain's tiles that touch water (beaches) |
| `objects` | mesh standing on the terrain's tiles |
| `edge_objects` | mesh on tiles at the terrain's edge (else `objects`) |
| `object_material` | material of those meshes (else the terrain's own) |
| `snow_material` | material of object parts above the mesh's `snow_line` |

The `oil` ground program makes black pools in dark soil with a wet sky reflection
and a sun highlight. It uses the ordinary `colour`, `colour_alt` and `scale`
parameters and the same near-to-map-colour blend as other terrain. Its declared
`base_colour` also colours the minimap. Terrain unknown to the local player
(`World::knows_terrain`, governed by resource `first_age_id`) uses plain land
for material, map colour, minimap and baked-object visibility. Age changes
refresh only chunk colour/material buffers touching changed tiles, including
corners across cube seams; no terrain geometry or objects are generated in play.

`mesh.generator` names a generator (`forest_centre`, `forest_edge`,
`rock_massif`, `crop_rows`); its params (`variants`, `count`, `height`,
`radius`, `trunk_height`, `trunk_colour`, `snow_line`, `blade_width`,
`blades`) are in tiles. All are `baked = 1`: the variants are built from the
map seed when the planet or map is published to the renderer, before the first
tick, and the feature tiles get one instance each. A building type drawn as
objects names its mesh and material in `entity_visual` (the farm's crop rows);
the row without faction and age is the one drawn. A composed row for the same
type (the farm's barn, task 38) is drawn as well, standing among the rows. Unknown programs and
generators fall back to the map colour and the plain building box. The rows
were appended after the generated content so existing definition ids did not
move (task 34).

### 10a. Composed models of buildings and units (task 37)

A mesh with `generator = 'composed'` is a model built from named parts: rows of
`mesh_part` (added by task 37), each one shape of the renderer's part kit
placed in the model's frame (x east, y up, z south, in tiles; origin on the
ground at the footprint centre; the front faces south). `name` is unique in
its mesh (the loader also refuses duplicates) and gives the part id
`<model id>/<name>` used in reviews. `material_id` NULL takes the
`entity_visual` row's material; `team = 1` makes the part's main surface wear
the owner's colour; `attach` names an earlier part, or a socket (`head`,
`right_hand`, `left_hand`, `back` of a `figure`; `rider`, `back`, `head`,
`chest` of a `quadruped`; written `socket` or `part.socket`), whose frame the
part moves with. `params` is flat JSON.

`material.program = 'emissive'` draws a part's `colour` and `colour_alt` without
sunlight, night dimming, shadow-map occlusion or baked contact shade. Use it for
furnace mouths, muzzle flashes, light strips, reactors and engines; ordinary
`tint` and the attack-only `flash` animation still apply. It emits no light onto
neighbouring surfaces and adds no bloom. The model materials `mat.glow` and
`mat.flash` in `db/models/materials.sql` use it; geometry and joints are unchanged.

Every part takes `x`, `y`, `z` (its base centre), `yaw` (degrees,
counter-clockwise seen from above), `roll` and `pitch` (degrees, default 0: the
part leans about its base centre, `roll` tipping its top towards +x, then
`pitch` towards +z, before the yaw; on `gable_roof` and `hip_roof`, whose own
`pitch` is the slope, only `roll` leans), `repeat` with `dx`, `dz` (copies,
each moved by that much), `far` (0 leaves it out of the far version), `strike`
(degrees the part swings forward about its base, along its own x axis, when its
unit attacks: a catapult's arm; a part attached to it with the opposite `strike`
keeps hanging, as a trebuchet's counterweight does), `recoil` (tiles the part
kicks back along its own −y when its unit fires: a gun barrel lying along its
aim), `flash` (1: the part, and what is attached to it, shows only for a moment
as its unit fires: a muzzle flash; 2 to 5: that many flashes on alternate keyframes from the
shot, a machine gun's burst), `traverse` (degrees the part swings about its upright axis and
back onto the target before each shot: a gun turret laying its gun), `bank` (the part flies:
it bobs and, under way, banks from side to side by up to that many degrees, one bank each way
in a two-second cycle, carrying everything attached: an aircraft's fuselage; task 46) and
`tint` (`#rrggbb`, multiplies the material colours). A part with both leans at 0 bakes
exactly as it did before leaning existed (task 38). The shapes and their own
knobs:

| Shape | Knobs |
| --- | --- |
| `box` | `w`, `d`, `h`, `taper`, `gable` (pitch of gable ends along x); openings `door`, `door_h`, `door_x`, `windows` (front and back), `side_windows`, `window_w`, `window_h`, `window_y`, `window_rows`, `row_gap`; colours `opening`, `door_colour` |
| `plinth` | `w`, `d`, `h`, `steps`, `inset` |
| `gable_roof`, `hip_roof` | `w`, `d` (of the block below), `pitch`, `overhang`, `t` |
| `flat_roof` | `w`, `d`, `t`, `overhang`, `parapet`, `parapet_t` |
| `dome`, `cylinder`, `cone` | `w` (diameter), `h`, `segments`; `cylinder` also `taper` and `bottom` (1 closes the foot; default 1 on a leaning cylinder such as a log or a wheel, else 0) |
| `column_row` | `count`, `radius`, `spacing`, `h`, `segments`, `capital` |
| `wall` | `w`, `h`, `t`, the openings of `box`; `ring` (four walls round `w` × `d`), `gap` (gate in the front) |
| `crenellation` | `w`, `d`, `h`, `t`, `merlon`, `ring`, `gap` |
| `fence` | `w`, `d`, `h`, `spacing`, `rails`, `ring`, `gap` |
| `palisade` | `w`, `d`, `h`, `t`, `spacing`, `ring`, `gap` |
| `banner` | `h`, `fw`, `fh`, `pole`, `notch`; colour `pole_colour` (the cloth is the part) |
| `awning` | `w`, `d`, `h`, `rise`, `posts`; colour `post_colour` |
| `arch` | `w`, `h`, `t` (a wall along x), `span` (width of the round-headed opening in its middle), `spring` (height where the head begins), `segments` |
| `hull` | `w` (length along x), `d` (beam), `h` (depth amidships), `sheer` (rise of both pointed ends), `segments`, `float` (1: rides the water, carrying the parts attached to it), `heel` (degrees a floating hull leans while under way: a sailing ship); an open boat, planked inside and out |
| `figure` | `h` (1 tile), `build`, `robe` (1: a long robe to the ground), `seat` (1: sits astride, hips on its origin + 0.4 `h`); motion `attack` (0 overhead strike, 1 thrust, 2 overhand throw, 3 draw a bow held in the left hand, 4 whirl a sling, 5 raise a long firearm or crossbow to the shoulder with both hands, aim, fire and lower it: the body turns its left shoulder towards the target, the butt beds in the right shoulder beside the cheek and the left hand holds the barrel up, 6 level a pistol, fire and lower it, 7 brace: the arms keep their hold on a mounted gun and the body hunches into
it, shaking from the shot: a machine gunner or a gun crew), `work` (0 swing a tool, 1 raise
both arms, 2 tend: bent over, both hands reaching down to someone, a medic), `hold_right`, `hold_left` (degrees an arm is carried forward, still, keeping what it holds upright; 0 swings), `stride` (degrees of leg swing, default 28), `pace` (seconds per step cycle, default 1); colours `skin`, `hair`, `legs` (the tunic is the part) |
| `quadruped` | `h` (size, 1: a horse with its back 0.82 up), `l`, `w` (body length and width), `stride`, `pace` (gait), `plain` (1: no ears, mane or hooves, for a team in harness); colours `mane`, `hoof` (the hide is the part); faces +z |
| `wheel` | `w` (diameter), `t` (thickness), `spokes` (bars across the faces), `segments`; upright on the ground, axle along x; turns as its unit moves |
| `oar_bank` | `w` (length of the bank along x), `count` (oars a side), `beam` (distance between the oarlocks across z), `length`, `drop` (degrees down to the water); each oar sweeps as its unit moves |
| `sail` | `w`, `h`, `billow` (belly towards +z), `lateen` (1: a triangle, its foot closing to the yard's −x end; lean the part to slant the yard); colour `yard_colour`; the cloth is the part |
| `shield` | `w` (diameter, or width with `h`), `h` (0: round), `t`, `boss`, `segments`, `point` (with `h`: the share of the height over which the sides close to a point at the bottom, a heater or kite shield); colour `rim`; centred on its origin, facing +z |
| `bow` | `h`, `bend` (towards +z), `w` (stave thickness); colour `string`; held at its middle (the origin) |
| `track` | `l` (length along z), `h`, `t` (width across x), `wheels` (road wheels, 1–8), `segments` (sides of each rounded end); colour `wheel_colour`; a tank's track run resting on the ground: a belt open at its sides round road wheels that turn as its unit moves (task 46) |
| `propeller` | `w` (diameter), `blades` (2–6), `chord` (blade width); colour `hub_colour`; faces +z with its axis through the origin and spins all the time, three turns a cycle; pitched −90° it is a rotor (task 46) |
| `torus` | `w` (outside diameter), `t` (diameter of the tube; default a quarter of `w`), `segments` (faces round the ring, default 16), `sides` (faces round the tube, default 8); lies flat with its lowest edge on the ground, centred on its origin (a reactor torus; roll 90 stands it up as a ring; task 49) |
| `include` | `mesh` (key of another composed mesh), `scale`; its parts without a material take the include's |
| `custom` | `file` (its triangles: a `.mesh` file under `assets/models/`, usually `<model id>/<part>.mesh`; format in `docs/models-in-meshcraft.md`); faces name their own material, colour and team flag, `-` taking the part's material (task 55) |

Composed models move (task 39), as presentation only. Baking records the
joints of the moving shapes: a figure's body, legs and arms (and one joint per
hand for what it holds), a quadruped's body, legs, neck and tail, each wheel,
road wheel, propeller and oar, a floating hull and every part with `bank`, `traverse`,
`strike`, `recoil` or `flash`; parts
attached to a part or socket move with its joint. A shot goes off at the same
point of the attack cycle for the shooter's arm, the barrel's recoil and the
flash (`FIRE_AT` in `rig.rs`), so they stay in step. Each unit's snapshot carries what it is doing —
idle, move, attack or work, read from the simulation's public state (see
`activity_of` in `rom-app`'s `sim_thread.rs`) — and the GPU poses the model from
keyframes baked before play, the render clock and a phase per entity. Shadows
keep the models at rest. A model has at most 40 joints, itself included (`MAX_JOINTS`;
raised from 32 by task 45 for a wagon drawn by four horses).

The loader refuses include cycles; the application reports an unknown shape
or knob once on stderr and skips it. Object programs for parts are
`plaster`, `brick` (also ashlar), `timber`, `roof_tile`, `thatch`, `metal`,
`cloth`, `emissive`, plus `stone` and the plain colour; their `colour`, `colour_alt` and
`scale` work as for terrain materials.

**Where the rows live (task 55).** Each composed model is its own file
`db/models/<model id>.sql` (the `mesh` definition, its `mesh_part` rows and its
`entity_visual` row), shared sub-assemblies are `db/models/shared/<mesh key>.sql`, and the
materials only models wear are `db/models/materials.sql`. The files load after the seed
(materials, shared, models; each group by file name), in `rom build-rules` and in the game,
which embeds them (`rom-models`); the model definitions therefore take the last `def` ids.
Moving the rows out of `seed.sql` kept every other row and id, and the same model rows by
key. The shape `custom` was added to the kit for free-form parts made in Meshcraft. `rom
model export`/`import` convert a model to and from a Meshcraft project
(`docs/models-in-meshcraft.md`).

**Review candidates (task 73).** `entity_visual.candidate` (new nullable TEXT column, a
lower-case name) marks a look offered for review and kept out of play: the game never draws it
(`ModelBook::resolve` and the scene looks skip it), while the gallery, `--model`, the gallery
manifest and the model export list it under its own id `<entity>@<age>[+<faction>]~<name>`, kept
in its own file `db/models/<that id>.sql`. Adopting a candidate is one change in its file: drop
`candidate` (the column and its value) and remove or re-date the row it replaces; the file name
and id then lose `~<name>`. The first candidate was `bld.auto_university@automation~lab`, an
alternative look for the lab; the owner approved it in review round 12 (2026-11-17) and it is
now the lab's only look, `bld.auto_lab@automation`. No candidate is open.

`entity_visual` picks a model per entity type: the most specific row wins —
a row of the owner's faction (or its nearest ancestor) before a row for every
faction, then the latest `age_id` not after the owner's current age (no age:
every age). The model id is `<entity key>@<age without "age.">` (`@any` with no
age), with `+<faction key>` for a faction's own row. Budgets, style and the
review loop (`--gallery`, `--model`, `--parts`, `--gallery-manifest`,
`docs/art/approved.toml`) are in `docs/art-direction.md`.

### 10b. How large ships are drawn in the space views (task 57)

`et_view_scale` (added by task 57; presentation only, the simulation never
reads it) gives a unit type two drawn sizes: `planet_pct`, its size above a
planet as a percentage of its true size (in orbit; it eases back to its true
size as the ship comes down to land), and `system_pct`, its class in space:
since task 96 a ship in space is drawn at its true size there (its model's
length at `planet_pct`, in the planet's tiles), and `system_pct` over the
smallest of all the rows scales its floor on screen (14 pixels for the
smallest class) and picks its marker far out (a chevron, a diamond from 1.5
times the smallest class). Before task 96 it was a percentage of a size that
stayed fixed on screen. No row means 100 and 100. Since task 72 the seed gives the starships two to three
times their size above a planet, in orbit and landed (orbital craft three
times in orbit, their own size once down in the air), and a quarter to a half
of the reference on the plane, by class (survey ship smallest, battleship
largest); the rows are generated (`VIEW_SCALE` in `db/content/generate.py`).
The renderer takes them through `rom-app`'s `looks::ship_scales` and never
looks at a key. The same model is drawn at every level; between the planet
and the plane its size and place blend smoothly with the camera's distance.
See "Big starships on the planet (task 72)" for the ground a landed ship covers.

### 11. Computer opponents are data too

`ai_profile` + `ai_profile_weight` (personality), `ai_opening_step` (opening
list) and `ai_difficulty` (thinking speed, order budget, reaction time,
mistakes). A computer player acts only by adding rows to `order`, like a
human. Its memory is saved in `player_ai_state`. See
`research/05-computer-opponents.md` and, for what is built,
`docs/computer-player.md`.

Notes from task 60:

- **Weights read so far:** `econ.worker_share_pct`, `econ.expand_eagerness`,
  `tech.age_priority`, `mil.army_share_pct`, `mil.defense_share_pct`. No
  weight key had to be added.
- **Opening steps must be reachable in order.** A step whose subject is still
  locked when its turn comes is waited for one minute and then skipped, so a
  building's technology is a step of its own before it. The seed's books were
  rewritten on that rule and now cover all four profiles (the old rows asked
  for a barracks before the technology that unlocks it).
- **Patrol routes (task 126).** No new order kind, column or save field: the queued `patrol`
  orders of one unit are the route. When the first of them starts and more patrol orders wait
  behind it, an order for the start position joins the back of the queue once; on arrival at a
  point the order finishes and a copy goes to the back, so the queue rotates and the unit
  walks the loop forever. A lone patrol keeps its back-and-forth (`home` swap). Orbit patrols
  are unchanged. An order other than a patrol queued behind a patrol is dropped when it would
  be applied (counted as cancelled): the patrol never ends. A save in mid-loop carries the
  rotated queue, so it continues from there.
- **The income bonus is a granted handicap (task 64).** See "Handicaps" below.
- **Order vocabulary (task 65).** `order_kind` gained `stance` (subject = a combat
  stance; applied when it leaves `pending`, the queue stays), so a computer
  player sets stances through orders like everything else. `explore`, `patrol`,
  `guard` and `raze` (an own building; an unfinished one refunds
  `economy.cancel_refund_pct` of its base price for the work not done; never a
  finished city) now run. `move`, `attack_move` and `patrol` may name a
  `formation` as subject: one tick's orders of a player, kind and formation
  walk at the slowest member's speed when the formation `keeps_slowest`. No other
  row was needed; the difficulty in the economy comes from the existing
  `ai_difficulty` columns (see `docs/computer-player.md`).

### Handicaps: modifiers a player is granted (task 64)

A difficulty's income bonus (and any other per-player handicap) is an ordinary
`modifier` whose source is a technology of kind **`handicap`**. Such a
technology is never researched: the host grants it to one player at match setup
with `World::grant_handicap(player, tech)` (only before the first tick), which
records it in the player's technologies like a researched one, so its modifiers
apply, are saved (`player_tech`) and hashed with everything else.

- `ai_difficulty.handicap_tech_id` names the technology a computer player of
  that difficulty is granted. The seed has one, `tech.handicap.income_25`
  (`income.*` `pct 25` on every income stat), on `toughest`.
- The validator: a handicap has no `production` row, no `tech_effect` (effects
  are applied by research only) and no prerequisites, and nothing requires it;
  it is exempt from the reachability check. A difficulty with a non-zero
  `income_bonus_pct` must name a handicap, and that handicap's `income.*`
  modifiers must all be `pct` of exactly `income_bonus_pct` — the column stays
  as the readable figure and the two cannot drift apart.
- A different handicap (a cheaper army, a wider sight) is another `handicap`
  technology with its modifier rows and a column or setup choice that names it;
  no code changes.

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

**A new building or unit model**: a file `db/models/<model id>.sql` with a `def`
(`kind='mesh'`) and `mesh` row with `generator='composed'`, its `mesh_part` rows, and an
`entity_visual` row (entity type, age from which it applies, mesh, default material), in
the style of the existing files (or made in Meshcraft and written by `rom model import`).
Check it with `--model <id> --parts` and keep it within the budgets of
`docs/art-direction.md`.

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
- The Galactic Federation (14, task 164) is the last age, after the Age of Galactic War (13,
  called Galactic Empires until then), but the match has no end: a player that has
  researched everything at the library may ascend (task 76, `docs/ascension.md`); a victory is
  a notice, not an end. Travel between galaxies stays parked.
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

A node is drawn with a composed model when its type has an `entity_visual` row, as buildings
are (task 82): the relic site wears `node.relics@any` (`db/models/node.relics@any.sql`, a
weathered platform, a trilithon and a gilded reliquary; new material `mat.gold`). Nodes are
owned by the neutral player, whose faction and age pick the row like any owner's; a node type
without a row keeps the marker the renderer draws. Each relic site is one relic: a priest
carries the whole site away, and a dropped relic stands as a whole site again.

`et_trader.route_income` is a delivery amount per `trade.route_unit` of the
accepted city-centre distance, paid at arrivals rather than per income period.
`trade.route_unit` is 10000 milli-tiles; `trade.min_route` is 20000 (the value
of `city.spacing`). `player.caravan_limit.base` is 2; `caravan_limit` modifiers
add route slots. Pending route orders reserve slots. Orders retain the home,
target, accepted length, delivery resource/amount and current leg in memory;
a trade route in progress is saved in the order's `route_*` columns (the old
`trade_route` table was dropped).

## Workers inside, units leaving buildings (task 52)

**Workers inside a gather building.** A worker that holds a gather slot
(`entity.work_site`) is *inside* the building: `Entity::is_inside`,
`World::occupants(building)` (id order). Nothing new is stored. Inside workers
are not drawn and cannot be picked on the map; enemies cannot see or target
them (`sees_entity`); their own side still lists them. The building's
selection panel shows their portraits ("Workers 4 / 10"); clicking one selects
that worker, shift adds it, a double click selects every worker of its type
inside. The container component (`et_container`, `entity.container_id`) was
not used: it models nodes carried by units and, since task 54, units aboard
ships; building garrisons do not exist yet. Any order given to a worker takes it out of the slot (as before). The
order kind `release` (`needs = gather_site`, target none) lets a gather
building send every worker out: they leave their slot and work order and
stand beside the building; with a rally point set the app also orders them to
walk there. Idle citizens look for work again after
`citizen.auto_work_delay_ticks`, so released workers who are not given a task
go back to work by themselves. Right-click and the **Release all workers**
button (Order tab, slot 3, hotkey R) with a gather building selected act on
everyone inside it.

**Where a produced unit appears.** `complete_training` puts a mover on the
free tile its movement class can enter nearest the building's gate within 10
tiles of the footprint, around all four sides (`World::free_tile_beside`:
water for ships, never a building tile). A mover that finds none and can stand
in space (spacecraft) launches as before; otherwise the order stays at the head
of the queue as `Blocked("no_spawn_tile")` and the HUD queue row says "No free
ground or water beside it". (In a universe a ship used to count as a
spacecraft to launch, find no room in space and stay at 100 % forever.)
Other blocked reasons the HUD words: `pop_cap`, `space_age`.

**Footprints are cleared.** When a site is placed and when a building
completes, every walking unit standing on its footprint is moved to the nearest
free tile of its movement class (`World::clear_footprint`), so builders are
never shut inside.

## Carriers, landing and colonies (task 54)

No table or column was added: carrying uses the existing component and state.

- **Carrier = `et_container` + `et_movement`.** `capacity` is in population
  (`pop_cost`), raised or lowered by `garrison_capacity` modifiers;
  `accept_tag_id`, when set, limits who may board. A passenger must be a unit
  that moves and cannot cross a space terrain itself (ships do not carry ships)
  and has no container of its own. The seed gives the new **Transport Ship**
  (`unit.transport_ship`, line `transport` in the generator, spaceport,
  starfaring, unarmed, `pop_cost` 2) a capacity of 20, and every other starship
  1 (survey ship, corvette) or 2 (frigate, cruiser, battleship): one or two
  squads. The city's `et_container` row (capacity 10) stays unused: a container
  that does not move is not a carrier, and building garrisons are not built yet.
- **Passengers** have `entity.container_id` = the carrier, as carried nodes do:
  off the map (no spatial index, no avoidance, no sight, not visible or
  targetable), skipped by the tick, by idle work, automatic targeting and
  attrition, still counted in population and for conquest. They ride along
  between systems. When a carrier dies, its passengers die with it (counted
  as losses, credited to the same killer).
- **Order kinds.** `enter` (existing, `needs = movement`, entity): board an own
  carrier — beside it when it has landed on the unit's planet, or, when it
  orbits the unit's planet, at the nearest own completed building that trains
  spacecraft (a spaceport). `land` (new, `container`, point): from the orbit of
  that planet, onto open ground (`layer = ground`, `buildable`, no building)
  outside every enemy city's economic radius; the descent takes
  `space.land_ticks` (new constant, 60); then everyone aboard comes out on free
  tiles around it, and the carrier stays landed. `lift` (new, `movement`,
  none): a landed carrier returns to orbit after `space.launch_ticks`.
  `unload` (existing, `container`, entity or point): landed — passengers come
  out around it (to a point: they walk there); in orbit — at the target own
  spaceport below. A passenger of a landed carrier that is given any order
  steps out first; aboard a carrier in space it refuses orders (`Aboard`).
- **Landed ships** stand on the planet's surface (`entity.body_id` that planet,
  no `entity_orbit` row) and are listed in the simulation's landed set; they do
  not walk. They are saved as the `landed` flag in `entity_flag`. Surface weapons can hit them; their own space weapons cannot fire (since task 58 their `orbit` rows reach what orbits above).
- **Colonies.** The first city a player founds on a planet may stand outside
  any border (never inside another player's territory), as free colonisation
  allows everywhere from the enlightenment age. Its borders, economy and
  production are those of any city.

## Orbital craft, carriers and ground batteries (task 58)

Schema changes, all deliberate (both lint views stay empty, the seed loads):

- **`et_movement.climb_ticks`** (new, `INTEGER NOT NULL DEFAULT 0`, `>= 0`). A unit
  whose movement row has `climb_ticks > 0` is an **orbital craft**: it flies in
  its planet's air with its ordinary movement class (`aircraft` in the seed),
  climbs from there to the orbit of that planet by itself in `climb_ticks`
  (order `lift`), and comes back down into the air above a point of that planet
  in the same time (order `land`). In orbit it stays with that planet: `move`,
  `attack_move`, `patrol`, `guard`, `explore`, `orbit` and `travel` are refused
  (`CraftInOrbit`); it can attack, board a carrier in the same orbit (`enter`;
  from the air it climbs first) or come down. It leaves the planet only aboard
  a carrier. `rom-rules` reads the column into `Movement::climb_ticks`.
- **Weapon reach between layers is data**: `et_weapon_target.layer = 'orbit'`
  (the `orbit` layer existed and was unused). An entity in a planet's orbit is
  on the `orbit` layer (it was `space` before task 58). A weapon with an
  `orbit` row hits it from the same orbit or from system space (the ordinary
  same-plane rule), and **from that planet's surface or air**, its range
  measured on the ground to the target's subpoint (ground → orbit). Orbit →
  ground stays `et_weapon.orbit_only`; orbit → orbit and space → orbit are an
  `orbit` row with an ordinary weapon. Starship weapons have both `space` and
  `orbit` rows. Landed ships with such weapons can fire up at what orbits above.
- **`et_weapon_target.hit_pct`** is now read: it scales the weapon's accuracy
  against that layer (100 = unchanged; every row before task 58 is 100). The
  laser battery's weapon gets an `orbit` row at 40 %: a weaker anti-orbit
  attack.
- **`order_kind`**: `land` now needs `movement` (it needed `container`), so a
  craft can come down; `lift`, `enter` and `unload` keep their needs, with
  notes for craft and launching carriers.
- **Tags** `passenger` (every unit that rides transports and starships: all units
  except spacecraft and orbital craft) and `orbital_craft`. Which carrier takes
  which units is `et_container.accept_tag_id`: the transport and every other
  starship accept `passenger`, the carrier accepts `orbital_craft`.
- **`et_container.launches`** (existing column, now read): a carrier in orbit
  whose container `launches` sends its craft out into that orbit with `unload`
  (the button reads *Launch craft*); landed, it sets them down around itself
  like any carrier. A carrier in free system space does not launch: craft have
  nowhere to hold there.

No simulation state was added: craft in orbit use `entity_orbit`, climbing and
descending are the progress of their `lift` / `land` / `enter` order, and craft
aboard a carrier are passengers (`container_id`), so saves (task 62) carry them
unchanged.

Content (generator, `orbital_combat()`, defined after every other generated
definition so earlier ids stay stable):

| Key | Where | Stats |
| --- | --- | --- |
| `unit.orbital_fighter` | airfield, starfaring | 420 HP, armour 4 vehicle, speed 5000 (aircraft), climb 100 ticks; 40 explosive, range 5 tiles, every 20 ticks, at `air`, `orbit`, `space`; 220 metal 80 energy, 400 ticks |
| `unit.orbital_bomber` | airfield, starfaring | 520 HP, armour 4 vehicle, speed 4000, climb 120 ticks; slot 0: 30 explosive, 4 tiles, 40 ticks, at `orbit`, `space`; slot 1 (`orbit_only`): 90 explosive, 8 tiles, 60 ticks, at `ground` (task 75: 300 kinetic, 80 ticks, a 4-tile blast); 260 metal 100 energy, 450 ticks |
| `unit.carrier` | spaceport, starfaring | 2000 HP, armour 6 shield, speed 2400 (starship), `pop_cost` 2; 25 energy, 6 tiles, at `space`, `orbit`; container 10 population, accepts `orbital_craft`, `launches`; 700 metal 300 energy, 500 ticks |
| `bld.ground_battery` | built by citizens and worker drones, starfaring, military page, own territory, plain | 2400 HP, armour 5 structure, 3 × 3, sight 16 tiles; 150 explosive, 14 tiles to the subpoint, every 30 ticks, at `orbit`; 400 metal 200 energy (task 75: 9 000 HP, armour class `hull`, 520 explosive, 20 tiles, 40 ticks; see "Orbital bombardment") |

`et_view_scale` rows: carrier 450 / 45, fighter 300 / 25, bomber 320 / 25. A
ground battery contests a landing: a carrier descending onto ground within its
range is in orbit until it touches down, so it is shot at all the way down.

## Sight (task 68)

Every base sight (`entity_type.los`, milli-tiles) comes from the content generator
(`unit_sight`, `ship_sight`, `building_sight` in `db/content/generate.py`); no code
reads a key. Rules of thumb, in tiles:

- **Units by role.** Citizens 4; other civilians and support 5 (repair drones 6); melee
  foot 5, foot with guns 6; ranged and anti-armour or anti-air teams their reach + 1
  (at least 6); heavy cavalry and tanks 7, light cavalry and fast vehicles 8; siege 5
  (short: siege needs spotters); warships 8 + age ÷ 3; aircraft 12 (drones 13); the
  scout of each age sees furthest on the ground: 11 (ancient to gunpowder), 12
  (enlightenment to modern), 14 (from the information age).
- **Ships by altitude and size.** `2 × ORBIT_ALTITUDE + 3 × size class`, with the
  orbit 8 tiles up (as drawn) and size classes 0 (orbital craft) to 4 (battleship,
  carrier); the survey ship sees 8 more. So 16 to 28 tiles, twice to four times a
  ground unit's.
- **Satellite.** 96 tiles: an eighth of a median planet's circumference (768
  tiles), the horizon seen from about 0.41 planet radii up and the furthest the cube
  geometry measures (half a face of a median planet); on smaller planets the reach
  limit (half a face) caps it.
- **Buildings by type and size.** One tile beyond the farthest-seeing ground unit of
  the building's first age, plus half a tile per tile of footprint: a 2 × 2 sees one
  tile more than its base, an 8 × 8 four.
- **Armed buildings** see at least `ARMED_SIGHT_PCT` (150 %) of their weapon's
  maximum range, so they spot what they can hit (towers, cities, forts 15 tiles; the
  ground battery 21). An upgrade of their range carries a sight row of 150 % of it.

| Units | Sight (tiles) |
| --- | --- |
| citizen, worker_drone | 4 |
| spearman, hoplite, pikeman, medic, catapult, trebuchet, bombard, cannon, field_gun, howitzer, rocket_artillery, railgun_battery, supply_wagon, supply_truck, scholar, merchant, caravan, freight_wagon, lorry, cargo_drone, priest | 5 |
| musketeer, fusilier, rifleman, infantry, assault_infantry, combat_robot, exo_trooper, marine, shock_marine, slinger, archer, crossbowman, arquebusier, light_infantry, machine_gunner, heavy_machine_gunner, repair_drone | 6 |
| gun_drone, pulse_gunner, beam_trooper, anti_tank_rifleman, bazooka_team, missile_team, railgun_team, cataphract, knight, cuirassier, lancer, light_tank, tank, main_battle_tank, robot_tank, fusion_tank, grav_tank, flak_gun, missile_launcher, laser_battery | 7 |
| horseman, light_cavalry, hussar, dragoon, armoured_car, scout_car, hovercraft, grav_skimmer, trireme, war_galley | 8 |
| carrack, frigate, ironclad | 9 |
| destroyer, missile_cruiser, drone_cruiser | 10 |
| scout, hover_frigate | 11 |
| ranger, biplane, fighter, jet_fighter, interceptor, bomber, strategic_bomber | 12 |
| drone_fighter, drone_bomber | 13 |
| recon_drone | 14 |
| orbital_fighter, orbital_bomber | 16 |
| corvette | 19 |
| space_frigate, transport_ship, space_freighter | 22 |
| space_cruiser | 25 |
| survey_ship | 27 |
| battleship, carrier | 28 |
| space_probe (task 130) | 40 |
| satellite | 96 |

| Building | Sight | Footprint | Weapon range | First age |
| --- | --- | --- | --- | --- |
| bld.barracks | 14 | 4×4 | 0 | ancient |
| bld.dock | 14 | 4×4 | 0 | ancient |
| bld.farm | 14 | 4×4 | 0 | ancient |
| bld.granary | 14 | 4×4 | 0 | classical |
| bld.library | 14 | 4×4 | 0 | ancient |
| bld.lumber_mill | 14 | 4×4 | 0 | classical |
| bld.market | 14 | 4×4 | 0 | ancient |
| bld.mine | 14 | 4×4 | 0 | classical |
| bld.oil_well | 14 | 2×2 | 0 | industrial |
| bld.siege_workshop | 14 | 4×4 | 0 | classical |
| bld.smelter | 14 | 4×4 | 0 | medieval |
| bld.stable | 14 | 4×4 | 0 | classical |
| bld.temple | 14 | 4×4 | 0 | classical |
| bld.university | 14 | 4×4 | 0 | classical |
| bld.woodcutter_camp | 14 | 4×4 | 0 | ancient |
| bld.bastion | 15 | 2×2 | 10 | enlightenment |
| bld.bunker | 15 | 2×2 | 10 | modern |
| bld.factory | 15 | 4×4 | 0 | industrial |
| bld.fort | 15 | 6×6 | 10 | medieval |
| bld.refinery | 15 | 4×4 | 0 | industrial |
| bld.tower | 15 | 2×2 | 10 | ancient |
| bld.vehicle_plant | 15 | 4×4 | 0 | industrial |
| bld.city | 15.5 | 7×7 | 10 | ancient |
| bld.airfield | 16 | 6×6 | 0 | industrial |
| bld.auto_oil_well | 16 | 2×2 | 0 | automation |
| bld.defence_turret | 16 | 2×2 | 10 | automation |
| bld.fortress | 16 | 6×6 | 10 | modern |
| bld.star_fort | 16 | 6×6 | 10 | enlightenment |
| bld.auto_farm | 17 | 4×4 | 0 | automation |
| bld.auto_mine | 17 | 4×4 | 0 | automation |
| bld.auto_lab | 17 | 4×4 | 0 | automation |
| bld.auto_woodcutter_camp | 17 | 4×4 | 0 | automation |
| bld.fusion_plant | 17 | 4×4 | 0 | fusion |
| bld.citadel | 18 | 6×6 | 10 | fusion |
| bld.auto_colony | 17.5 | 5×5 | 0 | abundance |
| bld.spaceport | 19 | 8×8 | 0 | starfaring |
| bld.ground_battery | 21 | 3×3 | 14 | starfaring |

**Sight technologies** (`SIGHT_TECHS`, ordinary `modifier` rows on `los`, all on
existing technologies; none was added):

| Technology | Age | Bonus |
| --- | --- | --- |
| Drill (`tech.military.2`) | classical | infantry +1 tile |
| Mathematics (`tech.science.2`) | classical | towers and cities: range +1 tile, sight +1.5 tiles |
| Astronomy (`tech.science.3`) | medieval | ships +2 tiles |
| Optics (`tech.science.4`) | gunpowder | everything +15 % |
| Electronics (`tech.science.7`) | modern | buildings +20 %, aircraft +20 % |
| Network Warfare (`tech.military.8`) | information | military units +15 % |
| Orbital Doctrine (`tech.military.10`) | fusion | starships +25 % |
| Fortification, Bombardment, Strategic Reserves (fort) | medieval to modern | forts: range +1 tile (existing), sight +1.5 tiles (new) |

The stat engine applies `los` modifiers when a technology completes
(`refresh_entity_stats`), and the fog of war follows (test
`sight_technologies_widen_the_fog_of_war`). A new tag `tower` marks the tower line
(as `fort` marks forts) so range and sight rows can name it.

**Defences fire while busy.** An armed entity that cannot move (a defence, a city)
fires on its own whatever its order queue holds: it keeps its aim (`entity.aim`, new
state, below) while the target is alive, visible and in range, takes the nearest
visible enemy in range when it has none (every ten ticks, or at once when its target
is lost), and fires when reloaded. It stays stand-ground; an explicit `attack` order
aims it instead. The early weapons of the city, tower and fort shoot arrows: their hits
land after the arrow's flight (`et_weapon.projectile_speed`, since task 102 the speed of
every arrow, see "Projectiles you can see"); the gun-age variants fire shells, bullets,
beams or plasma at their kind's speed. (The city's weapon is `pierce` in every age in the
data, so a city shoots arrows in every age.)

Screenshots (scripts `tasks/68-*-script.txt`): `img/sight-tower-firing.png`,
`img/sight-city-training-firing.png` (the city fires while its queue trains),
`img/sight-fort-firing.png`, `img/sight-fog.png`, `img/sight-fog-optics.png` and
`img/sight-fog-before-after.png` (a scout's lit disc before and after Optics).

## Ships in orbit (task 68)

- **Geosynchronous.** A ship or craft in a planet's orbit holds over one ground point
  (`entity_orbit` keeps it); the drift of task 43 (`space.orbit_speed`) is gone.
  `space.orbit_speed` is now only the speed of an orbiting entity without a movement
  row.
- **Moving within orbit.** `move`, `attack_move` and `patrol` to a point of the planet
  a ship orbits move its ground point at the ship's own speed along the great circle
  (steering at a point six tiles ahead, from the integer cube-sphere mapping; straight
  within half a face). It never descends. A patrol circles the planet on the great
  circle through its start (`order.home`) and its point, fighting like an attack-move,
  until given another order. An ordered attack carries the ground point toward its
  target until in range.
- **Landing.** `land` to a point first moves the ship within its orbit above the
  point, then comes straight down. The Land button lands on the point below.
  `World::landing_problem` says why not (water, rough ground, a building, an enemy
  city; since task 72 for the whole ground under the hull, below).
- **Boarding only on the ground.** `enter` a ship in orbit and `unload` in orbit are
  refused (`SimError::ShipNotLanded`); orbital craft keep boarding and launching from
  carriers in orbit.
- **Space freighter** (`unit.space_freighter`, spaceport, starfaring): a route trader
  that flies (`et_trader.runs_routes`, wealth, 8 per arrival per `trade.route_unit`).
  Its route joins an own city of the planet it stands on or orbits and an own or
  allied city of another planet of the same star system; the length is the distance
  between the two planets in the system plane. It runs the route as legs queued in
  front of the `trade` order (lift off, fly into orbit above the next city, land beside
  it, deliver within 14 tiles of it).
- **Satellite** (`unit.satellite`, spaceport, starfaring; information since task 130):
  launched into orbit above its spaceport, no weapon (one since task 134: "Armed satellites"), 150 HP, sight 96 tiles. The new tag `orbit_bound` keeps it
  there: only `stop`, `stance`, `disband`, `launch`, `attack` and moves within the
  orbit are accepted (`SimError::BoundToOrbit`); it cannot land (only carriers and
  route traders that fly can).

Schema and save format (deliberate; both lint views stay empty, the seed loads):

- **`entity.aim_id`** (save PART B, nullable): the enemy a defence keeps firing at.
  Save format 4; format 3 saves are refused.
- **`entity_orbit.facing`** may now be any heading 0–359 (it was a multiple of 90:
  a ship moving within its orbit faces its way along the great circle).
- Tags `tower` and `orbit_bound`; two units and their `et_view_scale` rows (freighter
  300 / 35, satellite 220 / 20); sight modifier rows; arrow speed on three weapons.
  No PART A column was added.

## Ships fight back (task 154)

No schema change; the rules of automatic fire for ships, in `rom-sim` (`stance.rs`,
`combat.rs`, `space.rs`):

- **Stance in orbit.** A ship in orbit fights by its combat stance (it stood its ground
  whatever its stance before, hold fire included). An orbital craft or a ship bound to its
  orbit (the satellite) still stands its ground there, unless told to hold fire.
- **Acquisition.** An idle ship, or one holding a station (its only order a player's `orbit`
  and it is there: in that orbit, or in the system's space within `space.orbit_reach` of the
  body or formation place it holds), looks for enemies every ten ticks as on the ground:
  aggressive within `unit.respond_range`, defensive within its reach + 3 tiles, stand ground
  within its reach, hold fire never. In a system's space and from its orbits, aggressive
  looks no less far than its weapons reach (the battleship's 13 tiles beat the 12 of
  `unit.respond_range`). Ships in one orbit all stand at the planet's position in the system
  plane, so they always find each other; what is on the planet below (`orbit_only`
  weapons) a ship in orbit takes on only within reach.
- **Fire back.** An idle unit or a ship holding a station answers the enemy that strikes it
  at once (not at the next ten-tick round): aggressive and defensive whatever the distance,
  stand ground (and a ship in orbit against what is on its planet or in its orbit) only
  within its reach, hold fire never. A unit with other orders keeps them (moving units keep
  moving), as on the ground; raid and raze stances do not answer by themselves, as before.
  Stand ground answering at once is new on the ground too; aggressive and defensive units
  there already did.
- **Leaving the orbit.** A ship in orbit (not a craft, not bound to the orbit) leaves it on
  an attack, ordered or automatic, at an enemy in the system's space or in another planet's
  orbit that it cannot reach from where it is, and closes on it. After an automatic attack
  it goes back to that orbit over the same ground point (an automatic `orbit` order queued
  behind the attack). On an attack move or a patrol in orbit it keeps to the orbit and lets
  such an enemy go once out of reach. Defensive: still in orbit it is home, and does not
  chase beyond six tiles of the planet.
- **Holding a station.** The automatic attack of a ship holding a station goes in front of
  its `orbit` order, which stays queued and takes it back to the station afterwards (until
  task 154 the attack replaced the hold).

## Routes and jumps (task 70)

Owner's choice (2026-11-16, 2026-11-17): ships keep flying freely inside a star system; the
routes between systems are charted, jumps along them are charged first, and jump inhibitors hold
enemy ships in a system. Ordinary rows plus the deliberate schema changes below; both lint
views stay empty and the seed loads.

**Charting.** A player knows a route once one of its ships has completed a voyage along it, or
a research revealed it (`World::knows_route`, `known_routes`). A voyage along a route its player
does not know takes `space.charting_pct` (500: five times) of the usual transit and charts the
route on arrival; a ship lost on the way charts nothing. A ship's path (`World::travel_path`) is
the shortest by time, so an uncharted route counts five times its length. Known routes are per
player, in the state hash (only once there are some), the snapshot and the save.

**Research** (spaceport, data only: `tech_effect.effect = 'enable_rule'` with these rule keys;
code looks for the key, never the technology):

| Technology | Age | `rule_key` | Effect |
| --- | --- | --- | --- |
| `tech.enhanced_sensors` Enhanced Sensors | expansion | `route_sensors` | once a second, charts every route leaving a system where the player has a ship or a planet; no voyage of the player is slowed by charting |
| `tech.interstellar_trade` Interstellar Trade | abundance | `interstellar_trade` | a space freighter may run a route to an own or allied city in another star system |
| `tech.automated_route_survey` Automated Route Survey | galactic | `route_survey` | every route is known |

**Charging a jump.** A ship ordered along a route charges where it is (task 111; until then it
first flew to the edge of its system toward the next system) for its stat `jump_charge` (new
entity stat): the new nullable
column `et_movement.jump_charge_ticks`, or the rule default `space.jump_charge_ticks` (160 ticks,
8 s) where a type has none. By hull size class: survey ship and corvette 120 ticks (6 s); space
frigate, transport and space freighter 160 (8 s); space cruiser 200 (10 s); battleship and carrier
240 (12 s). While charging it does not move or fire and can be hit; any new order (not a queued
one) cancels the charge. The charge lives in the travel order: `total_ticks` the ticks it needs
now, `progress_ticks` the ticks charged, status `blocked: charging`, then `blocked: depart`
(`World::jump_charge`). Ships whose travel orders were given in the same tick by the same player
from one system to one destination form a group; the group departs on the tick its last member
has charged (orders the simulation makes itself, a freighter's legs, travel alone). Transit time
still follows the route's length.

**Jump from anywhere (task 111).** Owner (2026-11-19): "jump to hyperspace from anywhere inside
the system to the edge of an unknown system, anywhere inside a known system; wind-up time still
applies on departure; the departure takes place from orbit". Routes stay: a jump still goes
along a route to a connected system, and a voyage over several systems is several jumps.

- *Departure.* A `travel` order charges where the ship is in its system's plane: from the orbit
  it holds (it leaves the orbit and charges at the planet's place in the plane) or from open
  space. A ship landed on a planet (`entity_flag` `landed`) may take the order too: it first
  lifts off to the orbit above, as its `lift` order would (`space.launch_ticks`; the travel
  order stays `active` meanwhile and `World::jump_charge` is none), then charges; it belongs to
  its group and holds the group back until it has charged. Orbital craft still leave a planet only aboard a
  carrier (task 58), so none climbs for a jump. Charge time, groups, "a new order cancels the
  charge" and the inhibitor's hold are as before.
- *Explored systems* (`World::system_explored`, `explored_systems`): every system for an ascended
  player; its home system; the celestial home; and any system on whose plane the player or an
  ally has explored a tile, which a ship of theirs does by being there (the fog of the system
  plane, already in the hash and the save: no new state). The star map labels the others
  "(unexplored)" from the same rule (`space_view::known_systems`). A system a ship is bound for
  is no longer counted before it arrives.
- *Arrival.* On the last leg of a voyage the ship comes out at the point its order names, kept
  inside the plane, if its player has explored the system it jumps into, and at the edge facing
  the system it came from if not; the choice is made on departure of that leg, for the whole
  group, and written into the travel order's `target_pos`. Passing through a system it comes out
  at the edge and charges there for the next jump. Charting is unchanged.
- *Orders.* The destination point travels in the `travel` command (`target_pos`, a point of the
  destination system's plane). A right click on a point or a body of another system's plane, at
  the system level, sends the selection there; on the star map, a known system sends it to the
  star, an unknown one to its edge (the hover says "they arrive at the edge"). A space freighter
  on an interstellar route jumps to the planet of its partner city (the edge, the first time).
  Computer players aim at the body they want, or, when an enemy inhibitor they know of (they
  have explored its ground) would strike an arrival there, straight out from its planet just
  beyond `arrival_radius`; no weight needed.
- *Inhibitor blow by distance.* Column `et_inhibitor.arrival_radius` (milli-tiles of the system
  plane, default 0 = no blow): an enemy ship is struck only when its arrival point lies within
  that distance of the planet the inhibitor stands on (`World::arrival_blow(player, at)`).
  `bld.jump_inhibitor` 12 000 (12 tiles): twice the orbit hold (`space.orbit_reach`, 6 tiles) and
  as far out as the planet's moons (10 to 12 tiles), so the planet's orbit and its near space are
  covered; under the 16 tiles of margin between the outermost orbit (at most 240 tiles from the
  star) and the plane's edge, so a ship that arrives at the edge is never struck; and well under
  half the gap between neighbouring planet orbits, which grow 1.4 to 1.9 times outward (with
  the outermost at 240 tiles the gaps run about 95, 57, 35 tiles inward), so an attacker can still
  jump into the system away from the held planet. Several inhibitors: the largest blow whose
  radius covers the point applies, ties to the lowest id.

**Jump inhibitor** (`bld.jump_inhibitor`, starfaring, since task 118 expansion (its sight and
build time stay those it was given for starfaring), 3 × 3, 3 000 hp, 500 metal and 300 energy,
built by citizens and worker drones in own territory on plain ground; no weapon). New component
table `et_inhibitor(entity_type_id, reach, charge_pct)` (since tasks 104 and 111 also
`arrival_damage` and `arrival_radius`): `reach = 'system'` (the only value)
covers the whole star system of the planet it stands on; `charge_pct` 50. While a complete,
active one stands, ships of its owner's enemies in that system may depart only back along the
route they arrived by (the new PART B table `entity_arrival` records it; a ship built there may
not leave at all) and charge `charge_pct` longer. Several do not stack (the largest applies).
Own and allied ships are free. A refused route is `SimError::HeldByInhibitor` at submission; a
queued or running travel order ends `failed: inhibited` (a freighter's trade order likewise).
Destroying it lifts the effect on the next tick; it has hit points like any building, so
bombardment from orbit or troops on the ground answer it (task 71's rule: it has no weapon, so
it is no strength that needs a counter).

**Arrival strike (task 104).** Column `et_inhibitor.arrival_damage` (hit points, default 0;
`bld.jump_inhibitor` 1 000): every ship of the inhibitor owner's enemies that arrives in its
system by a jump (since task 111: within `arrival_radius` of its planet, see above) loses that
many hit points once, at the arrival tick (`World::arrive_transits`,
in entity id order; what it destroys is removed in the same tick and credited to the
inhibitor's owner, with the usual plunder). It is a fixed blow, deliberately **not** through the
armour table (`damage_class` × `armor_class`, minus armour): the owner asked for "a fixed amount
of damage", the inhibitor has no weapon row to carry a damage class, and the line between the
hulls it destroys and those it only hurts should not move with armour research. Several do not
stack: the largest blow applies, ties to the lowest id (as `charge_pct`), so a defender cannot
make line ships die by building more. Own and allied ships, ships built in the system and ships
already there when it is completed are not struck, nor are craft aboard a carrier (owner,
2026-11-19: the carrier shelters them; only the carrier takes the blow, and if it dies they go
down with it, as ever). Base starship hit points (no hit-point
modifier applies to starships) and what the blow takes: space freighter 500, corvette 800 and
survey ship 1 000 are destroyed; space frigate 1 300 keeps 300 (77 % lost), transport 1 500 keeps
500 (67 %), space cruiser and carrier 2 000 keep 1 000 (50 %), battleship 3 600 keeps 2 600
(28 %). Orbital craft jump only aboard a carrier, so they are never struck. The blows are kept a
minute for the views (`World::arrival_strikes`, not hashed and not saved, like blasts); the hit
points they take are state like any other. The answer under task 71's rule: the inhibitor now
does damage, but only to ships arriving, and its counters are those of any building (orbital
bombardment by the line ships that survive the blow, troops landed from a transport); the fight
matrix pits armed units and armed buildings against each other in the field and does not model
it. Computer players do not weigh the blow: their fleets (`rom-ai` fleet: claims, escort
strikes, colony voyages) may send light hulls into a held system and lose them, which the owner
accepted (2026-11-19).

**Interstellar trade.** With `interstellar_trade`, `validate_trade` accepts a space freighter's
target city in another star system (`SimError::NeedsInterstellarTrade` before the research).
The route's length is the path its ships take: the distances in each system's plane (home
planet to the route's edge, edge to edge through a system passed, edge to the target planet)
plus each route's length times `trade.route_length_mtiles` (1 000: a unit of route length counts as
one tile). The
freighter flies it as legs: lift off, a `travel` leg (to the edge, charge, jump, charting at the
slow rate the first time, cross), orbit the target planet, land, deliver; then back. Income per
arrival is `et_trader.route_income` (8) per `trade.route_unit` of that length. Measured by
`space_view::trade_tests` (universe seed 1, abundance age, a freighter's round trip once its
route is charted): to the nearest other planet of the home system the route is 339 tiles, pays
271 per arrival and takes 249 s there and back, **130 wealth per minute**; to the nearest planet
one jump away it is 846 tiles (477 flown in the two planes, 369 route units), pays 676 per
arrival and takes 377 s, **214 wealth per minute** (1.65 times). The first trip out, charting
the route, takes longer.

Schema and save format (deliberate):

- **Task 111.** Column `et_inhibitor.arrival_radius` (deliberate schema change; lint views
  empty, seed loads; the value comes from `db/content/generate.py`). No new state: explored
  systems are read from the fog of the system planes, and the arrival point is the travel
  order's `target_pos`, both already hashed and saved, so the save layout is unchanged (format
  13); the rules fingerprint changes, so earlier saves are refused as with any rules change.
- **Task 104.** Column `et_inhibitor.arrival_damage` (deliberate schema change; lint views
  empty, seed loads). The save layout is unchanged (format 12): the rules fingerprint changes, so
  saves of earlier versions are refused as with any rules change.
- **PART A.** `et_movement.jump_charge_ticks` (nullable); table `et_inhibitor`; stat
  `jump_charge`; constants `space.jump_charge_ticks`, `space.charting_pct`,
  `trade.route_length_mtiles`; the three rule keys above documented on `tech_effect.rule_key`.
- **PART B, save format 7** (format 6 saves are refused): `player_route(player_id, system_a,
  system_b)` the routes each player knows; `entity_arrival(entity_id, system_id)` the system each
  ship last arrived from. A charge needs no table: it is the travel order's `progress_ticks`,
  `total_ticks` and status, which the save already keeps.

## Big starships on the planet (task 72)

Owner (2026-11-17): the starships are drawn 2-3 times their size in the planet view, in
orbit and landed (a first try at 7-10 times was too big); a landed ship avoids only buildings
and mountains, may stand on forest and partly over water; bigger ships hover higher. Until
task 72 a model was drawn at its own size whatever `planet_pct` said (only its pick box and
health bar grew); now the model, pick box, health bar and hull length share one size.

- **View scales** (`et_view_scale.planet_pct`, presentation): survey ship 200, corvette 210,
  space frigate and space freighter 230, transport 240, space cruiser 260, carrier 280,
  battleship 300 (task 57's class order); orbital fighter 300 and bomber 320 and the
  satellite 220 unchanged. The rows are generated (`VIEW_SCALE`); the hand-written block
  that held most of them since task 57 is gone. A starship keeps its planet scale when it
  lands; a type with `et_movement.climb_ticks > 0` (an orbital craft) eases to its own size.
- **The ground a landed ship covers** is simulation data, never the view scale:
  **`et_movement.landing_radius`** (new nullable column, milli-tiles; stat `landing_radius`,
  new) — the tiles whose centres lie within it of the tile the ship stands on. NULL covers
  that tile only. The generator sets it to the model's reach from its centre (`HULL_REACH`)
  times its planet scale, so it is the hull the planet view draws; rom-app's test
  `landing_radius_is_the_drawn_hull` checks the two agree within 1 % and 100 milli-tiles.
  Values: survey ship 3 100, corvette 3 900, space freighter and transport 4 600, space frigate
  4 900, carrier 6 600, space cruiser 7 800, battleship 14 600 (only the transport, the
  carrier and the freighter land).
- **The rule** (`World::landing_problem`, used by the Land button, `land` orders, and a
  freighter's landing legs through `landing_site`): no tile under the hull may hold a
  building, nor have a terrain with the new tag **`no_landing`** (on `ter.mountain`); forest
  and every other ground terrain (ice included) are fine; water is fine as long as at least
  **`space.landing_land_pct`** (new constant, **20**) of the tiles are ground (layer
  `ground`). Not inside an enemy city's economic radius, as before.
- **Around the hull.** Passengers come out on free tiles just outside it, and troops board at
  its edge (`landed_bounds`). A freighter's landing search reaches twelve tiles past its hull
  and it delivers when its centre is within fourteen tiles plus its landing radius of the
  city.
- **Hover height** is presentation only (the simulation keeps no altitude; ground fire is
  measured to the ground point): `rom-render`'s `layers::hover_altitude` holds a ship
  8 + 0.6 × its drawn length tiles above sea level, so bigger ships hold higher; a bigger ship
  over a smaller one of another class rises clear of it (`entities::stack_by_size`), and ships
  whose hulls still overlap at one height (one class over one point) are pushed apart sideways
  by their size (`entities::spread_aloft`).
- **Freighters explain themselves.** `World::trade_offer` (read-only) answers whether a trader
  could run a route to a city and what it would pay, the check `submit` makes; the interface
  rings valid partners and names the reason for the others.

Schema, all deliberate (both lint views stay empty, the seed loads): PART A column
`et_movement.landing_radius`; stat `landing_radius`; constant `space.landing_land_pct`; tag
`no_landing` with one `def_tag` row; et_view_scale rows changed. No PART B change: the save
format stays at 7.

## Automated colonies (task 73)

Owner (2026-11-17): in the abundance age a city can be upgraded to an automated colony facility
that gathers from a planet by itself, keeps up to five automated farms, five woodcutter camps and
five automated labs, does not count towards the city limit, can be placed anywhere and adds to
wealth; the automated university is renamed the Automated Research Lab; the survey ship stays
buildable. Content in `automated_colonies()` of the generator (one call line), the survey line
in `LINES`; both lint views stay empty and the seed loads.

**Schema (deliberate).**

- PART A table **`et_colony(entity_type_id, resource_id, income, site_share_pct)`**: a type with
  this row is an automated colony. It must also be a city (`et_territory.is_city`) and have an
  `et_builder` row (the validator checks both). Its owner earns `income` of `resource_id` each
  income period while it stands complete and active, plus `site_share_pct` of the
  `et_gather_site.self_income` of every complete site whose city is the colony
  (`entity.city_id`, same owner); both are income terms of the colony and of each site.
- PART A table **`et_colony_site(colony_type_id, site_type_id, cap)`**: what the colony's
  governor keeps standing. The cap is per upgrade line, and it is also that line's per-city limit
  inside the colony's area (`World::check_city_limit`): a colony holds five labs although a lab is
  one per city elsewhere, and at most five camps, which have no limit elsewhere.
- PART A column **`entity_visual.candidate`**: review candidates kept out of play (§10a).
- `order_kind` **`pause_governor`** and **`resume_governor`** (`needs = 'colony'`, a new value of
  `needs`; target none): settings applied at once like `stance`; the colony's queue stays.
- PART B: `entity_flag.flag` takes **`governor_paused`**. **Save format 8**; format 7 saves are
  refused.

**What the simulation does with it.**

- **Outside the city limit.** `World::city_count` counts only cities that count
  (`counts_as_city`: a city type without `et_colony`); construct orders of a colony skip the
  `CityLimit` check. Spacing (`city.spacing`) and every other city rule still apply; as a city
  type under free colonisation (abundance is past the enlightenment age) it may stand on any
  plain land outside other players' borders, on any planet where a builder of its owner stands.
  A player's first city is still the first of any kind (`has_any_city`).
- **It builds its own sites.** A builder without movement works on a site whose footprint lies
  within its own economic radius, where it stands (`World::approach`); anything else fails with
  `out_of_reach`. The colony therefore takes ordinary `construct` and `repair` orders.
- **The switch.** `World::governor_paused(colony)`; the set is hashed only while one is paused,
  in the snapshot (`Snapshot::governors_paused`) and the save.
- Queries: `is_colony_type`, `counts_as_city`, `line_root`, `colony_cap`, `colony_buildings`.

**Data** (`db/content/generate.py`, `automated_colonies`):

| Row | Value |
| --- | --- |
| `tech.auto_colonies` "Automated Colonies" | research at the city, abundance age, 1 300 wealth and 650 knowledge, 900 ticks |
| `bld.auto_colony` "Automated Colony" | abundance age, needs the research; 5 × 5, 3 000 HP, armour 4 structure, sight 17.5 tiles, 450 ticks; 400 metal and 200 energy (ramped); built by citizens and worker drones on plain ground; tags `building`, `mechanical`, `city` |
| its `et_territory` | city, border 24 tiles, economic radius 16 tiles (as a city's), capturable |
| its `et_builder` | `build_pct` **200**: it builds its sites at twice a worker's pace |
| its `et_colony` | **30 wealth** per income period (30 s), plus **10 %** of its sites' own income as wealth |
| its `et_colony_site` | **5** `bld.auto_farm`, **5** `bld.auto_woodcutter_camp`, **5** `bld.auto_lab` |
| its `production` | construct those three; no train rows (no citizens) |
| `prerequisite` | the research is an alternative (group 1) to each automated site's upgrade research, so the colony's sites can be built without a university or farm first. It applies everywhere (owner, 2026-11-17): after Automated Colonies a lab may also be built directly beside any city, one per city, without a university there first; a university is still upgraded to a lab by the lab's upgrade research |
| `cost` of the automated sites (new, `AUTO_SITE_PRICES`) | **automated farm 200 metal and 100 timber, automated woodcutter camp 200 metal and 100 timber, Automated Research Lab 300 metal and 150 knowledge**, no ramp (owner, 2026-11-17: a governor must really wait when its owner cannot pay). Until task 73 they had no price of their own; a university, farm or camp upgraded in place still pays through its upgrade research only |
| `bld.auto_lab` (was `bld.auto_university`) | renamed **Automated Research Lab** with task 73, then **Auto Lab** with key `bld.auto_lab` (owner 2026-11-17; its upgrade research `tech.up.auto_lab`, "Upgrade to Auto Lab"; `AUTO_SITE_NAMES` in the generator, definition ids unchanged); rules, upgrade from the university and income (70 knowledge a period) unchanged |
| `unit.survey_ship` | a line of its own (`survey` in `LINES`), so the corvette no longer upgrades from it: "Upgrade to Corvettes" is gone, the corvette needs only the starfaring age, and survey ships stay trainable beside it; its stats, sight and role are unchanged |

With everything of one colony standing (five farms at 10 food, five camps at 100 timber, five labs
at 70 knowledge a period), the colony adds 30 + 10 % × 900 = 120 wealth a period. Its fifteen
sites cost 3 500 metal, 1 000 timber and 750 knowledge. A governor pays them as it places them
and waits, naming what is missing ("needs 200 Metal"), when its owner cannot pay. Note: the
abundance age's own research makes every price zero (`tech.age.abundance`, `cost.all × 0`), so a
player who researched the age pays nothing and a governor waits only for a player who reached
the age without that research (a granted age, as in the screenshots). The owner kept this
(2026-11-18): the automated sites' prices stay as they are, free after the abundance research.

Removing "Upgrade to Corvettes" moved the ids of every definition generated after it down by one
(and shifted the spaceport's research grid); the new definitions come last.
`rom playthrough --seed 1` now builds 33 buildings (the colony once; 32 before) with the same
183 technologies (Automated Colonies in, Upgrade to Corvettes out): hash `be5aa51a1e931472`
(was `853ceecaa69fcd4e`). The automated sites' prices left it unchanged (measured): the playthrough
never pays one, whether it upgrades its sites or builds them after the abundance research.

## Orbital bombardment (task 75)

The owner (2026-11-17): a few volleys of a frigate, cruiser or battleship flatten a city and
its surroundings; some units survive; ground batteries keep the ships at bay. No schema change;
the data is generated in `orbital_bombardment()` of `db/content/generate.py` (the starships'
bombardment rows were hand-written after the generated block until now and are removed there).

**`et_weapon.splash_pct` now falls off** (simulation, `World::splash`). Every other enemy within
`splash_radius` of the impact, on a layer the weapon targets, takes `splash_pct` % of what the
hit would do to it at the centre of the blast, falling linearly to nothing at the edge;
distance is measured to a building's footprint and a unit's edge (it was the units' centres at a
flat share). Neutralised entities, passengers, entities in orbit and the neutral player are
never hit. No weapon had a splash before task 75, so nothing else changed. The struck target
takes the full hit as before.

**New rows, no new columns:** damage class `kinetic` (bombardment from orbit) and its
`damage_matrix` rows: structure 300 %, flesh 50 %, plate 50 %, vehicle 40 % (every other armour
class 100 %). The ground battery wears the existing armour class `hull` (kinetic and explosive
damage 100 %). A new armour class for it was tried and dropped: computer players weigh units
against enemies not yet seen by an average over every armour class, so a new class changed
their armies in the first ages and with them the duels (moderate–easiest 94–0, 6 draws).

**Bombardment** (slot 1, `orbit_only`, target layer `ground`; reach in tiles of ground from the
ship's subpoint):

| Ship | Age | Damage | Reach | Reload | Splash radius, share at the centre | Was |
| --- | --- | --- | --- | --- | --- | --- |
| corvette | starfaring | 60 explosive | 10 tiles | 60 ticks (3 s) | 2 tiles, 50 % | 37 explosive, 12 tiles, 60 ticks, no splash |
| space frigate | expansion | 2 000 kinetic | 12 tiles | 100 ticks (5 s) | 12 tiles, 100 % | 39 explosive, 12, 60, none |
| space cruiser | abundance | 2 600 kinetic | 13 tiles | 100 ticks | 14 tiles, 100 % | 42 explosive, 12, 60, none |
| battleship | galactic | 3 200 kinetic | 14 tiles | 80 ticks (4 s) | 16 tiles, 100 % | 44 explosive, 12, 60, none |
| orbital bomber | starfaring | 300 kinetic | 8 tiles | 80 ticks | 4 tiles, 100 % | 90 explosive, 8, 60, none |

The survey ship has no bombardment; the carrier bombards through the bombers it carries.

**Ground battery:** 9 000 HP (was 2 400), armour 5 `hull` (was `structure`); its weapon at
the `orbit` layer 520 explosive (was 150), 20 tiles to a ship's subpoint (was 14), every 40
ticks (was 30); sight 30 tiles (its weapon's reach × 1.5). It outranges every bombardment and
hits a ship of any age for about half its hit points.

**Measured** with `rom fight-matrix` (`crates/rom-cli/src/orbit_matrix.rs`): one ship against an
undefended town of its age, the city and one of every other building its builders raise (the
newest of each line; 25 buildings reaching 17.7 tiles from the city's centre; every research of
the age), twenty soldiers of the age on four rings out to the town's edge; the ship aims at the
city, then at the standing building nearest its ground point.

| Age | Ship | Volleys to level the town | To knock out the city | Soldiers left |
| --- | --- | --- | --- | --- |
| starfaring – galactic | corvette | not levelled in 10 min | 14 | 20/20 |
| starfaring – galactic | orbital bomber | 54 (a carrier holds ten) | 2 | 19/20 |
| expansion – galactic | space frigate | 5 | 1 | 5/20 |
| abundance – galactic | space cruiser | 5 | 1 | 4/20 |
| galactic | battleship | 3 | 1 | 6/20 |

**Simulation rules that came with it** (`rom-sim`):

- A neutralised city under an enemy that bombards from orbit (a ship with an `orbit_only`
  weapon whose ground point lies within the city's economic radius) is not restored by its
  owner's units in the radius (task 67's rule); ships alone never take it. Without this a single
  soldier left alive anywhere in the radius put the city back at a quarter of its hit points
  after every volley.
- A ship whose only order is to hold the orbit it is in acquires targets like an idle one (the
  attack replaces the hold); before, a ship sent into an orbit sat under enemy fire without
  firing back.
- `World::blasts()` (where an area weapon struck: tick, centre, radius, owner, type) and
  `World::wrecks()` (buildings that fell, not neutralised: id, tick, place, type, owner) are kept
  for a game minute for the views. They are a report: not hashed and not saved (save format
  unchanged, 8); a loaded game shows no scorch from before the save. (Task 80 adds the stains,
  which are saved.)

Effect on the playthrough (`rom playthrough --seed 1`): `hash=52f0f01b8fc09b6e` (was
`be5aa51a1e931472`), the same buildings, units, techs and ticks; the code of 0.0.75 gives the same
new hash on the new data, so the change is the data alone (the battery's hit points and the
weapons' rows).

## Ground stains (task 80)

The owner (2026-11-17): destroyed buildings and units leave a stain on the ground proportional
to their stature, which dissipates after a few rotations of the planet; only destroyed
buildings, not demolished ones, leave a stain and debris.

**Constants** (generated in `ground_stains()` of `db/content/generate.py`, no new tables in
PART A): `world.stain_days` = 6 (days: a stain lasts `world.stain_days × world.day_ticks` =
36 000 ticks, 30 game minutes at the default day) and `world.stain_cap` = 2 000 (stains kept
per surface; past it the oldest goes). Both are required by `rom-sim` (0 or more); 0 days or a
cap of 0 keeps no stains.

**What leaves one** (`rom-sim`, `World::remove_dead`, the destruction point task 75 found for
wrecks): a building or unit on a planet surface (a habitable body, or the only map of a world
without a universe) that lost its last hit points: combat, splash, orbital bombardment,
attrition, an attacker spent by its own attack. A building its owner pulls down (`raze`) is
recorded at the order and leaves none; neither do boarding, garrisoning, upgrades (the entity
is retyped, not removed), disbanding, ships in orbit or in space (they stand on the
system's plane), or units that go down with their carrier (they are aboard). A complete city
is struck down and captured (task 67), never destroyed, so it leaves no stain.

**Size** (constants in `rom-sim/src/stains.rs`): a building's radius is 0.6 × its footprint
diagonal (`BUILDING_STAIN_PERMILLE` 600; a 4 × 4 farm 3.39 tiles, a 7 × 7 city site 5.94,
an 8 × 8 spaceport 6.79); a site under construction leaves that times its progress, at least
`STAIN_FLOOR` (0.6 tiles). A unit's is 2.5 × its collision radius (`UNIT_STAIN_PERMILLE` 2500;
every unit has 250, so 0.625 tiles), a landed ship's at least its `landing_radius`, never less
than the floor. The views widen a unit's mark to 0.65 × its model's longest extent when that is
larger (`rom-app`, `UNIT_STAIN_PER_EXTENT`), so a tank marks more ground than a soldier; the
state keeps the simulation's radius.

**State.** Each surface keeps its stains oldest first: the id the entity had, its type and
owner, its position, the radius (milli-tiles) and the tick of destruction, appended in id order
and dropped at the end of their life (`World::fade_stains`, each tick). They enter the state
hash once a surface has any (worlds without stains keep their hashes) and are saved in the
PART B table `surface_stain(body_id, seq, entity_id, type_id, owner_id, x, y, face, radius,
tick)`: **save format 9**; format 8 saves are refused, as every older format is. Nothing in the
simulation or the computer players reads them; `rom duel` results are unchanged.

**Drawing** (`rom-render/src/stains.rs`, `scene/stains.rs`, `assets/shaders/stain.*`): the HUD
snapshot carries the stains of the planet shown on tiles the player sees now. The renderer
copies the terrain's own triangles of the land tiles under each stain (so it follows the relief
and stops at the shore) into one soot mesh, and a building's debris (low broken pieces in its
model's three main colours, a few in its owner's, some charred) into one debris mesh; both
meshes are made before play with fixed room (43 690 and 21 845 triangles, newest stains first
when full) and rewritten only when the stains in view change. The soot is drawn right after
the land, nearer the eye along the view ray and without writing depth, so units, buildings,
trees and shadows stay over it. The stain shader draws the ragged mark (soot at the centre, ash
flecks, churned earth at the rim, varied by the stain's id), lit by the day, the night and the
shadows like the ground (the soot per tile corner, the debris per pixel), and fades it from
the game clock: 1 − 0.45 × age, times a smooth fall to nothing over the last 40 % of its life,
so most of the fading comes in the last third; debris weathers and settles into the ground as
it fades. Overlapping stains blend towards the soot colour and never past it. Task 75's
minute-long scorch now fades in eight seconds, and its standing rubble is the stain's debris.

**Ruins of an ascension (task 76).** When a player ascends, each of its buildings on a planet
leaves the stain of a destroyed building (soot and debris) as its ruin; its units leave none.

## Ascension (task 76)

The rules, the state and the computer players' part are described in `docs/ascension.md`.
Schema and seed (deliberate; both lint views stay empty, the seed loads), all generated by
`ascension()` in `db/content/generate.py`, after every other definition so earlier ids stay:

- **PART A.** `tech_effect.effect` takes **`ascend`**: completing the tech takes its player out
  of the material plane; it may be researched only once every other technology its producer
  offers is researched. `tech.ascension` (kind `project`, nominal 7 500 ticks, 10 000 knowledge
  and 10 000 wealth) at the library's projects page, in the last age (Galactic Empires until
  task 164, now the Galactic Federation). `order_kind`
  **`descend`** (target a point, no executor). `body_kind` **`black_hole`** and **`celestial`**
  (the celestial home's centre and its mythical planet; neither has a surface). Constants:
  `ascension.duration_pct` 250, `ascension.descent_research_pct` 90,
  `ascension.descent_other_pct` 15, `ascension.descent_citizens` 5,
  `ascension.ai_descend_ticks` 12 000, `celestial.dir_x/y/z` (the unit vector to the galactic
  centre in millionths, computed from its right ascension and declination),
  `celestial.distance_pct` 150, `celestial.planet_orbit` 6 000.
- **PART B, save format 10.** `player.origin_faction_id`, `player.ascended_tick` (set exactly
  while the status is `ascended`, a CHECK), `player.ascensions`; `player.status` takes
  `ascended`. A `star_system` without a catalogue star (`star_id` NULL) is the celestial home.

## How a match ends (task 117)

The rules are in `docs/ascension.md`, "How a match ends". Schema and seed (deliberate; both
lint views stay empty, the seed loads):

- **PART A, constants.** `stats.history_period_ticks` 1 200 (one sample per player a game
  minute) and the score weights `score.gathered_per` 100, `score.unit_trained` 5,
  `score.building_completed` 10, `score.city_founded` 100, `score.city_captured` 150,
  `score.kill` 2, `score.tech` 20, `score.age` 300, `score.ascension` 2 000 (hand-written in
  `seed.sql`, beside the other match constants).
- **PART B, save format 14.** `match_human(player_id)`: the players people play;
  `match_end(id = 1, tick, reason, team)`: the end once reached (`no_human_left`, or
  `last_standing` with its team); `player_stat(player_id, spent, units_trained,
  buildings_completed, cities_founded, cities_captured, cities_lost, techs_researched,
  peak_land, peak_army, defeated_tick, defeated_by)`; `player_gathered(player_id, resource_id,
  amount)`; `player_age_reached(player_id, age_id, tick)`; `player_history(tick, player_id,
  score, land, army, cities, stock, age_id)`. **Format 13 saves still load**: they have no
  humans (the report then reads human seats from `player.controller`), no end, zero counters and
  no history. Older formats are refused.
- **State hash.** The counters and the history of every player are always hashed (so the
  playthrough's hash changed with task 117 while its world did not); the humans and the end only
  when there are any, so matches without people hash the same either way.

## Alerts and jump detectors (task 118)

Owner, issue #3 (2026-11-21): alerts when a city is attacked, a building that tells of ships
jumping into the system (fusion age), the survey ship as a sensor (both, "Yes, exactly."), and
the jump inhibitor no earlier than the age of expansion. Behaviour and wording:
`docs/hud.md` "Alerts" and `docs/world-model.md` "Jump detectors". Rows come from
`db/content/generate.py` (`alerts_and_detectors()`, defined after every other generated
definition so earlier ids stay stable); deliberate schema change, both lint views stay empty and
the seed loads.

- **New component table** `et_detector(entity_type_id, scope, radius, enabled)`: `scope =
  'system'` watches every jump arrival in the star system of the planet the entity stands on;
  `scope = 'radius'` watches arrivals within `radius` (milli-tiles of the system's plane) of the
  entity while it stands still (no orders) in a system's plane. `enabled` is the base of the new
  entity stat **`detector`**: an entity detects while the stat is above 0, so a type can carry
  the component switched off until a modifier switches it on. A detector must be complete and
  active (not neutralised, not assimilating, not aboard). It watches for its owner only, and
  sees ships of players not on its owner's team. Several detectors of one player give one alert
  per arriving owner and system and tick. Code reads the component and the stat, never a key.
- **`bld.jump_detector`** "Deep Space Tracking Station": fusion age, 2 × 2, 1 000 hp, armour 3,
  300 metal and 150 energy (ramped), built by citizens and worker drones (military page) in own
  territory on plain ground like the inhibitor, `limit_scope = 'per_city'`, `limit_count = 1`
  (the limit columns have no per-planet scope; several do not duplicate alerts);
  `et_detector('system', 0, 1)`. Its hit points are those of a light building so the town
  `rom fight-matrix` levels from orbit (`orbit_matrix.rs`) still falls to a battleship in fewer
  volleys than to a cruiser (at 1 500 both took four). Model `db/models/bld.jump_detector@fusion.sql` (for the owner's
  model review). Computer players build it through the construction manager's ordinary rule for
  a building with no other role (one, low priority, once the fusion age unlocks it): no code
  change and no weight.
- **`unit.survey_ship`** gains `et_detector('radius', 40000, 0)`; **`tech.survey_sensor_array`**
  "Survey Sensor Array" (spaceport research, starfaring age, 800 ticks, 1 100 wealth and 550
  knowledge) carries the modifier `detector set 1` on `unit.survey_ship` (`target_def_id`).
  Listed first on the spaceport's research page, so Enhanced Sensors, Interstellar Trade and
  Automated Route Survey each move one slot right.
- **Constants** `alert.attack_repeat_ticks` 600 (one under-attack alert per city, or per field
  near a city, or per body without a city, in this window from the alert) and
  `alert.marker_ticks` 200 (how long an alert is kept and marked, and arriving ships stay shown).
- **The inhibitor** moves to the age of expansion: `first_age_id` and its prerequisite. Nothing
  else required it.
- **State.** Alerts are computed by the simulation in the step (blows that damaged an entity,
  jump arrivals) and kept `alert.marker_ticks` for the views (`World::alerts`, like the arrival
  strikes): not hashed and not saved; the repeat window is not saved either, so a loaded game
  may alert once more. They change nothing in the game. No save format change; the rules
  fingerprint changes, so earlier saves are refused as with any rules change.

## Aircraft land and take off (task 123)

Owner (2026-11-21): jets and biplanes have two planes of action like the orbital craft, the
ground and the air, "they can land and takeoff (but not in the orbital plane obviously)".
Behaviour: `docs/world-model.md` "Aircraft in the air, on the ground". Rows come from
`db/content/generate.py` (`aircraft_take_off()`, values only, so every id stays); deliberate
schema change, both lint views stay empty and the seed loads.

- **`et_movement.takeoff_ticks`** (new, `INTEGER NOT NULL DEFAULT 0`, `CHECK (takeoff_ticks >= 0
  AND (takeoff_ticks = 0 OR climb_ticks = 0))`): ticks an aircraft takes to take off from the
  ground into the air above its planet, and to land; 0 = it never lands (always airborne, as
  every flyer was before). A type with `takeoff_ticks > 0` is an **aircraft**; it never has
  `climb_ticks` (orbital craft keep theirs and never land). `rom-rules` reads it into
  `Movement::takeoff_ticks`. Seed: biplane 60, fighter 70, drone fighter 70, jet fighter 80,
  interceptor 80, bomber 90, drone bomber 90, strategic bomber 100. The repair drone (also of the
  `aircraft` movement class) hovers and keeps 0; so do the orbital fighter and bomber.
- **`et_movement.takeoff_run`** (task 119, nullable `INTEGER CHECK (takeoff_run >= 0)`): the
  ground this type rolls while taking off and glides while landing, milli-tiles; NULL = the
  constant `air.takeoff_run`; 0 = straight up and down (the three rotorcraft). Read by
  `rom-rules` into `Movement::takeoff_run` and by `rom-sim/src/aircraft.rs` for the roll and the
  glide; existing aircraft keep NULL.
- **Constant** `air.takeoff_run` 3000 milli-tiles: the ground an aircraft rolls along its
  heading while it takes off (accelerating), and glides over while it lands (slowing).
- **Orders.** No new kind: `land` (point) and `lift` (none) serve aircraft too, their notes
  updated. `land` is refused (`CannotLand`) unless a landing spot lies within twelve tiles of
  the point on the aircraft's own planet; `lift` is refused (`NotLanded`) for an aircraft in
  the air. Every order that moves an aircraft or makes it fight (`move`, `attack`,
  `attack_move`, `patrol`, `guard`, `explore`, `enter`, …) takes off first.
- **Landing spot** (`World::landing_spot`): the tile nearest the point, within twelve tiles,
  whose terrain is on the `ground` layer and not tagged `no_landing` (no water, no mountain),
  with no building tile within the aircraft's reach plus `LANDING_MARGIN` (half a tile) and
  half a tile more (buildings hold whole tiles), so the wings stay clear of every building; and
  as far from every other aircraft that stands, takes off or lands there or whose `land` has
  chosen its spot as both their reaches and the margin; chosen in id order. The **reach** is
  `et_movement.landing_radius` (existing column; for an aircraft: half the larger horizontal
  extent of its drawn model, to the nearest hundred milli-tiles, from `AIRCRAFT_REACH` in the
  generator: biplane and fighter 1 400, jet fighter 1 600, drone fighter 1 200, interceptor
  2 100, bomber 2 000, strategic bomber 2 200, drone bomber 1 900). `rom-app`'s test that a
  landing radius is the drawn model's half extent covers them. An airfield
  (a building that trains a type with `takeoff_ticks > 0`, from `production`; no key is read)
  is landed at by picking it: the aircraft park on the free ground beside it. Its own tiles hold
  its hangar and tower, which a landed model would cut through.
- **Where new aircraft appear** (the rule asked for by the card): an aircraft trained by a
  building comes out **landed**, on the free landing spot nearest the building's gate, and
  takes off when it is sent somewhere (its rally point, if
  any). It comes out in the air only when no spot is free nearby, or when its producer is not a
  building (no such producer exists today). From data: the producer's class and the product's
  `takeoff_ticks`; aircraft only ever come from the airfield now, so in practice they come out
  landed beside or on it.
- **Combat.** An aircraft on the ground, taking off or landing is on the `ground` layer for the
  weapons that hit it and, failing a `ground` row, still in reach of a weapon's `air` row
  (`World::layer_row`: a parked aircraft is a ground target that anti-air reaches too). It
  cannot fire. Idle on the ground it acquires targets and retaliates by its stance like any
  idle unit, and the automatic attack takes off first: parked fighters scramble when an enemy
  they can hit comes into sight. Airborne nothing changed.
- **State** (`World::flight`, `BTreeMap<EntityId, u64>`): the ticks each aircraft on or near
  the ground has climbed toward its flight height, 0 = landed; airborne aircraft have no entry.
  Taking off counts up (and rolls), landing counts down (and glides); an aircraft whose orders
  no longer take it up settles back to the ground where it is. Hashed only while not empty, so
  worlds without a grounded aircraft keep their hashes. The landing spot is the `land` order's
  `home`, its glide start the order's `target_pos`.
- **Save format 15.** PART B table `entity_flight(entity_id, climbed)`; formats 13 and 14 still
  load, with every aircraft in the air. The rules fingerprint changes, so earlier saves are
  refused as with any rules change.

## Rotorcraft (task 119)

Generated in `db/content/generate.py` (line `rotorcraft`, members marked `late` so earlier ids
stay; `rotorcraft_hover()` adds the tags after every other definition). Units `unit.attack_helicopter`
(modern), `unit.gunship_helicopter` (information), `unit.hover_gunship` (automation), `ui_group`
`rotorcraft`, trained at the airfield, `aircraft` movement class with `takeoff_ticks` 30/36/24 and
`landing_radius` 1600/2000/900 (the drawn models) and `takeoff_run` 0 (they lift and land vertically). Tags: `hover` (movement; all three), `low_flight`
(movement) and `drone` on the hover gunship. No schema change.

## Drone operators (task 120)

Task 128 turned the cloud into a grid of drones that are targets of their own and dropped
`drone_hp`; see "The drone grid (task 128)" below for what changed. As built in task 120:

Owner, issue #3: an infantry unit that is "the operator" and "a cloud of drones, let's say 16";
the cloud gets there first, the operator stays behind at the greatest distance allowed; each
drone attacks and is spent (one kills or badly hurts an infantry unit, about ten destroy an
armoured vehicle); the operator issues new drones continuously; to destroy the unit you must
destroy the operator. Deliberate schema change; both lint views stay empty and the seed loads.

- **New component table** `et_drone_cloud(entity_type_id, capacity, leash, strike_range,
  rebuild_ticks, drone_hp, drone_speed)`: the most drones (a new unit has them all), the farthest
  the cloud flies from its operator and how near a target it must be to strike (milli-tiles), the
  ticks per new drone while below capacity, the damage that brings one drone down, and the
  cloud's speed (milli-tiles per second). Read into `EntityType::drone_cloud` (`rom-rules`).
- **The strike is the type's `et_weapon` row.** The card proposed `strike_ticks` and
  `strike_damage` columns; they are the weapon's `cooldown_ticks` and `damage` instead, so the
  strike goes through the normal damage path (damage class, armour, the `attack` and `range`
  stats) and everything that reads weapons (automatic targeting, the computer players, the fight
  matrix, the `fire` sound) sees it unchanged. Each shot needs a drone and the cloud within
  `strike_range` of the target, flies from the cloud, and spends the drone. The weapon's
  `range_max` is where the operator stops from its target: validated to be at most
  `leash + strike_range` (and the `range` stat is capped there), with movement and no orbital
  weapon (`Rules::validate`).
- **Being shot down.** A weapon with an `air` row in `et_weapon_target` meets the cloud of an
  operator whose cloud holds drones instead of the operator (`World::layer_row`): it aims at the
  cloud and measures its reach to it; each hit brings down `damage × shots / drone_hp` drones (at
  least one), not through the armour table; no splash on clouds. Weapons without an `air` row
  must reach the operator. An operator is seen where its cloud is seen.
- **Moving.** Each tick after the units have moved, a cloud flies toward the enemy its
  operator's current order attacks (the nearest footprint point of a building), else the point
  the order goes to, else the operator, never beyond the leash; aboard a carrier it waits. A
  drone made for an empty cloud starts at the operator.
- **Seed** (`db/content/generate.py`, `drone_operator()`, after every other definition so earlier
  ids stay): damage class `fragmentation` (vehicle and hull 18 %, structure and shield 10 %, people
  100 %); `unit.drone_operator` "Drone Operators", information age, barracks, `ui_group`
  `drone_operator`, 130 hp, armour 2 (flesh), sight 9 tiles, population 2, 400 ticks, 100 food,
  80 metal, 40 oil; foot, 1.2 tiles/s; weapon 210 fragmentation, reach 8 tiles, every 10 ticks,
  ground only, projectile `drone` (new value of `et_weapon.projectile`, 10 tiles/s); cloud 16
  drones, leash 7 tiles, strike range 1 tile, a drone every 120 ticks, 8 damage a drone, 5
  tiles/s. One drone against assault infantry: 210 − 2 = 208 ≥ 205 hit points; against the main
  battle tank 210 × 18 % − 3 = 34, ten drones for 328 hit points.
- **State** (save format 16): `entity_drone_cloud(entity_id, drones, body, face, x, y, rebuild)`;
  `World::drone_cloud`. Hashed while any cloud exists, so other worlds keep their hashes.
  Formats 13 to 15 still load (they have no operators).
- **Computer players** train it through the ordinary army choice; `rom-ai` counts a cloud
  weapon at the pace its operator issues drones (`rebuild_ticks` when longer than the reload).
- **Views.** `EntityView::cloud` (`CloudView`: drones and the cloud's point); the render thread
  draws up to its drones as small quadrotors circling the point (offsets from the operator's id
  and the render tick) and each strike as a streak from the cloud with a small blast.
- **Sounds.** Its own lines, `fire` (recipe `drone_strike`: a dive whine and a small blast) and a
  `moving` loop of footsteps with a faint swarm hum (the `swarm` parameter `assign.py` sets for
  types with an `et_drone_cloud` row).

## The drone grid (task 128)

Owner, review round 17: the drones hover above the operator "in a rectangular grid of 9 or 16",
deploy forward to the attacked target, "one drone attacks one enemy unit"; "all parts of the unit
are targetable, drones are targetable individually", constantly replenished while the operator
lives; "only the operator takes hit points"; the model review shows the grid. Deliberate schema
change; both lint views stay empty and the seed loads.

- **`et_drone_cloud`** loses `drone_hp` (a drone has no hit points: one hit brings it down) and
  gains `spacing` and `hover` (milli-tiles between neighbouring slots and above the ground);
  `capacity` is now 1 to 64 (the slots are bits of a word; `CHECK` and `Rules::validate`).
  `DroneCloud::side` is ⌈√capacity⌉ and `DroneCloud::slot_offset(slot)` gives a slot's place
  ahead and right of the grid's centre: rows from the front, each from the left, `spacing`
  apart, centred.
- **State** (`rom_sim::Cloud`): the grid's centre (it flies as task 120's cloud did: to the
  attacked enemy, ahead on a move, over the operator at rest, never beyond the leash), its
  `facing` (degrees, turning toward the way it flies at most 15° a tick, `TURN_PER_TICK`; it
  keeps its facing while still), `ready` (bit k: slot k holds a drone), the drone `joining`
  (slot and position) and `rebuild`. Slot positions are the offsets turned by the facing, in
  milli-tiles (`World::drones_of`).
- **Strikes.** A shot of the operator's weapon sends one drone at one enemy within
  `strike_range` of the grid's centre: the ordered target first, then the nearest visible enemy
  (distance, then id), passing over an enemy that the drones its owner already sent (hits and
  shots of operator weapons on their way) will kill by the damage path's numbers. The drone in
  the slot nearest that enemy leaves; it dives `√(ground² + hover²)` at the weapon's
  `projectile_speed` and is spent.
- **Targets.** No screening any more (`World::layer_row` reaches the operator as any foot
  soldier). `World::drone_row`: a weapon with an `air` row reaches each drone in a slot or
  joining one (not those diving). `World::aim_part` chooses: a weapon that hits only the air the
  nearest drone; only the ground, the operator; both, the operator on an ordered attack and the
  nearer of the operator and its nearest drone otherwise (the operator on a tie). The shot
  carries the drone's slot (`PendingHit::drone`); its hit brings that drone down whatever the
  damage and splashes nothing; a slot emptied meanwhile is a miss.
- **Replenishing.** Below capacity a new drone every `rebuild_ticks` takes off from the operator
  for the first empty slot and flies there at `drone_speed`; a drone due while another is still
  on its way puts that one in its slot. An operator's end (death or disbanding) ends its grid
  and the drones it sent (`despawn` drops its hits and shots).
- **Seed.** `et_drone_cloud`: 16 drones (four by four), leash 7 tiles, strike range 1 tile,
  a drone every 120 ticks, 5 tiles/s, spacing 0.6 tiles, hover 1.5 tiles. The weapon sends a
  drone every 5 ticks (four a second; task 120 sent one every 10), and the dive flies at 4
  tiles/s (`PROJECTILE_SPEED['drone']`, 10 before), about 8 ticks from a slot above the target,
  so one or two drones are in the air at once. Damage unchanged: one drone for assault infantry,
  ten for the main battle tank.
- **State, save format 17.** `entity_drone_cloud(entity_id, body, face, x, y, facing, ready,
  rebuild, join_slot, join_face, join_x, join_y)`; `pending_hit` and `pending_shot` gain
  `drone` (NULL for a shot at the unit itself). Format 16 saves load with every grid full and
  facing east. Hashed while any grid exists; a hit's drone only when it has one, so worlds
  without operators keep their hashes.
- **Views.** `CloudView` carries the centre, facing, side, spacing, hover, the ready slots, the
  joining drone and, while blending two snapshots, the slots whose drone left (where a dive
  starts). The render thread draws a quadrotor of the part kit (owner-coloured body, crossed
  arms, four turning rotors, a small charge) 0.3 tiles across (the card's 0.18 read as specks) in every slot, bobbing gently,
  instanced with the projectile models (`ProjectileInstance::team` and `phase`). The gallery and
  the model sheet draw an operator with its full grid from its type's `et_drone_cloud` row.

## Capitals revealed by research (task 130)

Owner (2026-11-23): every player's own capital is always known; an adversary's capital city,
planet and star system are revealed gradually by research, from the information age to the last.
Everything is data: four new `tech_effect` rows of `effect = 'enable_rule'` and one added to
Unification, no new effect kind and no code that reads a technology's key.

| Age | Research (producer) | `rule_key` | Reveals |
| --- | --- | --- | --- |
| any | — | — | own and allied capitals in full, always |
| information | Orbital Survey (spaceport; needs a satellite owned) | `planet_survey` | every building on the player's home planet, kept current as if seen; units stay in the fog. Shared with the team, as sight is |
| starfaring | Unification (library, exists) | `capital_system` | the capital **system** of every player met |
| starfaring | Deep Space Survey (spaceport; after Orbital Survey) | `capital_planet` | the **planet**, where the system is known |
| expansion | Long-range Imaging (spaceport; after Deep Space Survey) | `capital_city` | the **city**, where the planet is known |
| galactic | Total Network (Deep Space Tracking Station, the jump detector of task 118) | `all_capitals` | every capital at city level, met or not |

Costs and times follow the other spaceport research: `100 × (age + 1)` wealth, `50 × (age + 1)`
knowledge, `300 + 50 × age` ticks (Orbital Survey 800 / 400 / 650, Deep Space Survey 1 100 / 550
/ 800, Long-range Imaging 1 200 / 600 / 850, Total Network 1 400 / 700 / 950).

- **First contact.** A player has met another once it, or a member of its team, has seen a unit or
  building of the other (`World::has_met`, `World::contacts`), or knows one of its buildings by a
  survey. It is one way: a capital is revealed to the player who saw, not to the one seen (a
  scout watched from a city has not met the city's owner). Contacts are kept for good; allies
  have always met. Checked at the end of every tick.
- **Survey.** `World::surveys(player, body)`: a member of the player's team holds
  `planet_survey` and has `body` as its home planet. On such a surface every foreign building
  (sites and capitals too) is in the player's memory of buildings (`remembered_buildings`,
  `visible_entities` as `Remembered`), refreshed whenever a building there is placed, changes
  owner or type, or goes, exactly as for a tile in sight; the surface's buildings are all
  remembered the tick the survey starts. Units are never shown by it.
- **Capital knowledge.** `World::known_capitals(viewer) -> Vec<KnownCapital>` by owner: level
  (`System` < `Planet` < `City`, each implying the coarser), system, body (from planet level),
  city id and position (city level only; `None` above), and whether the viewer sees the city
  now. Own and allied capitals are always at city level. An enemy capital: the level research
  gives (above; Unification needs a contact), raised to city level by sight (the city in view
  now, or remembered, which includes a survey). **Research keeps what it reveals current at
  once** (a capital destroyed is dropped, a replaced capital is followed); what sight alone
  showed stays where it was last known until the viewer sees that place again without it. A
  capital taken by another team keeps its place and its owner's capital it remains
  (`World::capital`).
- **The space probe** (`unit.space_probe`, spaceport, information age): 80 hit points, unarmed,
  sight 40 tiles, 8 tiles a second, 80 metal and 40 oil, bound to its planet's orbit (tag
  `orbit_bound`) like the satellite. Patrolled round the planet it sweeps a band 80 tiles wide.
- **Spaceport in the information age.** `bld.spaceport` and `unit.satellite` need the
  information age (their `prerequisite` rows); every other ship and research of the spaceport
  keeps its starfaring or later prerequisite, so from information to fusion it offers only
  satellites, probes and the Orbital Survey. Their stats and `first_age_id` stay those of the
  starfaring age (it names the age of their look); the views draw a type its owner builds before
  the age of its first look with that look. The satellite now costs 150 metal and 100 oil
  (energy came in the fusion age then; since task 146 it comes in the automation age). A ship
  bound to its orbit may be launched before the space age (`World::may_launch`); every other
  launch still needs it.
- **PART B, save format 18.** `player_contact(player_id, other_id)` and `capital_known(player_id,
  owner_id, level, city_id, pos_x, pos_y, pos_face, pos_body)`. Saves of formats 13 to 17 load
  without them: players meet again as they see each other, and research reveals at once. The
  surveys are derived from the research and are not saved.
- **State hash.** Contacts and known capitals are hashed only once there are any, so worlds
  where nobody has met anybody keep their hashes.
- **Computer players** research the new technologies through the existing research manager
  (side research by worth) and build the spaceport and the tracking station as other buildings;
  they do not train probes. Their attack targeting already values a capital it remembers; a
  capital known only by research is not yet an attack goal (docs/computer-player.md).

## Armed satellites (task 134)

Owner (2026-11-23), asked what should counter satellites and probes before the starfaring age:
"satelites can attack". The satellite fights for the orbit over its planet; whether it should also
fire at the ground or the air is not decided (below).

**The weapon** (data only: one `et_weapon` row and its `et_weapon_target` row, generated last by
`armed_satellites()` in `db/content/generate.py`, so every earlier weapon id stays):

| Damage | Class | Reach | Reload | Layer | Projectile |
| --- | --- | --- | --- | --- | --- |
| 20 | `kinetic` (shield 100 %) | 80 tiles, along the orbit | 20 ticks (1 s) | `orbit` | `railgun`, instant |

What it does to each target, per hit after armour (`damage_matrix`, flat armour, at least 1):

| Target | Hit points, armour | Per hit | Falls after |
| --- | --- | --- | --- |
| space probe | 80, 0 | 20 | 4 hits (3 s) |
| satellite | 150, 0 | 20 | 8 hits (7 s) |
| corvette, space freighter | 800 / 500, 3 | 17 | 47 / 30 hits |
| survey ship, space frigate, transport ship | 3 to 5 armour | 15 | |
| space cruiser, carrier | 6 | 14 | |
| battleship | 8 | 12 | |
| orbital fighter, orbital bomber (vehicle: kinetic 40 %) | 4 | 4 | |

A satellite deals a starfaring starship at most 17 a second; the ground battery deals a corvette
517 every 2 s (258 a second) from 20 tiles. Measured in `rom-sim/tests/armed_satellites.rs` on a
median planet (192 tiles a face): a probe that flies into reach falls 66 ticks later (the first
shot waits for the shared acquisition pass, every 10 ticks); two satellites 50 tiles apart destroy
each other in the same tick, 141 ticks after the fight starts; a probe 90 tiles away (in sight, out
of reach) and a soldier on the ground below are never fired at.

**Reach along the orbit** (`rom-sim`, the one rule this needed). Every entity in a planet's orbit
stands at the planet's marker in system space, so ship against ship in one orbit measured 0 and
every weapon reached the whole orbit; a satellite with this weapon would have shot down every probe
over the planet. A ship bound to its orbit (tag `orbit_bound`: the satellite, the probe) now
measures its reach from its ground point to the target's (`World::bound_origin`, used by the aim,
the reach check and the automatic search), fires only from that orbit (not from the ground
before its launch) and reaches only what orbits its own planet: never a ship over a moon or a
neighbouring planet, nothing in open space, nothing on the ground. Every
other ship measures as before. On a cube planet nothing reaches beyond half a face, so on planets
smaller than the median the reach is that half face. An ordered attack moves the satellite along
its orbit toward its target (the existing chase in orbit); an idle one fires on what comes into
reach and stays put. The automatic search of a ship in orbit no longer looks at the ground below
it unless it has a weapon that strikes the ground (`orbit_only`): nothing it found there could be
hit, so no result changes.

**Setup.** `World::launch_for_setup(type, owner, above)` puts a spacecraft straight into the
orbit above a point, as a launch from a spaceport there would; `rom fight-matrix` fields
satellites with it (a ship bound to its orbit refuses the `orbit` order that brings other ships in
from space), and measures any reach no farther than half a face of its planet. The matrix's
observer (the farthest-seeing unit that lets both sides see the field) was the farthest-seeing
unarmed unit, the satellite; it is now the farthest-seeing unit with no weapon at the ground, the
water or the air, still the satellite. Picked by "unarmed" it became the space probe (40 tiles),
the sides lost sight of each other and 33 ground strengths went unanswered.

**Computer players** (`rom-ai`): a ship bound to its orbit is the space manager's watcher,
armed or not (`fleet::Kind::Other`), so one satellite is still trained over each planet with a
spaceport and none is sent to escort, claim or strike; it is not a military unit for the army and
production managers (`production::is_military`). They use the weapon through the data: their
satellites fire on enemy satellites, probes and ships in reach. An enemy satellite they see now
counts among the enemy ships the warship is chosen against.

**Not done: ground and air.** The weapon has only the `orbit` layer. To also strike the ground, a
second row (slot 1, `orbit_only`, layer `ground`, as the starships' bombardment); the air from
orbit would need the `air` layer reached from orbit, which the simulation does not measure today
(from orbit only `orbit_only` weapons reach the planet's surface).

**Measured.** `rom fight-matrix`: `strengths=270 unanswered=0`, the same rows as 0.0.125. The
satellite (first age starfaring, the age of its look) is in the orbit and ship-against-ship rosters
from that age on; it outranges every ship and beats none at equal cost (3 seeds of the starfaring
age, `--orbit-fight`: 0‰ left against the corvette, the orbital fighter and the survey ship, which
keep 1000‰), so it is no strength and no answer, and the ground battery still answers every ship
that bombards. `rom duel` moderate–easiest, seeds 1–20: 20–0, mean tick 54 449, as before (no duel
reaches the information age).

**Effect.** Rules fingerprint and the playthrough and ascension hashes move with the data: the
playthrough's flat test map has no orbit, so the satellite it trains stays on the ground beside its
spaceport, and being armed it now counts in the player's army (`World::army_size`, the match
report's peak army, which is hashed). The code of this task on the earlier data gives the earlier
hashes. Sounds: `unit.satellite.fire` (two variations, `fire.railgun`) and the line
`voice.satellite.attack`, baked alone (`tools/sounds/README.md`); every existing file is unchanged.

## Bringing life (task 77)

The rules, the state and the views are described in `docs/bringing-life.md`. Schema and seed
(deliberate; both lint views stay empty, the seed loads), hand-written at the end of `seed.sql`
after every other definition so earlier ids stay:

- **PART A.** `def.kind` takes **`brush`**, with its subtype table **`life_brush`** (`family`
  land, water, elevation, biome or deposit; `terrain_id` laid on the disc or NULL; `node_type_id`
  a map node set at the centre or NULL; `height_op` keep, set, raise or lower toward `height`
  on the 0..255 scale; `radius` in milli-tiles); `lint_def_without_subtype` checks it. `map_style`
  gains **`life_n`** (64: the face size of the dormant surface every resource body gets before
  play) and **`life_terrain_id`** (`ter.barren`). New ground terrains `ter.barren` (Barren Rock),
  `ter.desert` (Desert), `ter.snow` (Snowfield), walked like the plain (its movement rows are
  copied) but not the plain every building needs; materials `mat.barren`, `mat.desert` (snow
  wears `mat.ice`). `order_kind` **`bring_life`** (target an entity, the body's marker; no
  executor) and **`paint`** (target a point, subject a brush; no executor). Constants
  `life.sea_height` 100 and `life.ground_height` 112. The 27 brushes of the seed: 20 by hand,
  the deposits from the rules (every terrain a gather site needs or stands on other than the
  plain, at two sizes; every map node that is kept or carried).
- **PART B, save format 11.** `body_dormant(body_id, n, terrain_id)` and
  `body_life(body_id, player_id, since_tick, strokes, heights)`. A body brought to life is a
  `habitable` body row with `n` and a `surface` row; its marker is a planet marker.

## Every nature paintable (task 114)

Seed only (deliberate; both lint views stay empty, the seed loads), at the end of `seed.sql` after
every other definition so earlier ids stay; no schema change:

- **Five `life_brush` rows** (32 in all). `brush.ice.small` / `.large` ("Ice", "Ice, wide") and
  `brush.barren.small` / `.large` ("Barren Rock", "Barren Rock, wide"): family `biome`, `keep`,
  radius 4 000 and 9 000, made for every ground terrain no brush laid yet. Barren rock is the
  dormant ground, so it also clears a biome. `brush.deposit.relics.large` ("Relic Site,
  scattered"): family `deposit`, radius 4 000, made for every map node a player keeps or carries.
- **Constant `life.node_spacing`** 3 000 milli-tiles: a node brush wider than a tile scatters its
  sites over the disc at least this far apart (`rom-sim` `World::paint_stroke`).

## Market trading (task 107)

Owner, 2026-11-19: a player with a market buys and sells resources for wealth at prices that rise
when it buys and fall when it sells, recovering over time; the idea of Reference A's market, in our
own numbers. Schema and seed (deliberate; both lint views stay empty, the seed loads; the data is
written by `market_trading` in `db/content/generate.py`, after every other definition, and adds
none, so every id stays):

- **PART A.** Component table **`et_market(entity_type_id, currency_id)`**: the building trades
  there, every trade paid in `currency_id`, which must be a resource that is not itself tradeable
  (`Rules::validate`). The seed gives it to `bld.market`, paying in `res.wealth`. `resource.tradeable`
  is 1 for food, timber, metal, oil and energy and 0 for wealth (the currency), knowledge (learned,
  not bought) and the rare resources (claimed, not traded); `resource.market_base_price` (now
  `CHECK > 0`) is the price of one lot at rest: food and timber 100, metal 130, oil and energy 160.
  `order_kind` **`buy`** and **`sell`** (`needs = 'market'`, target a def: the resource). The
  `enable_rule` key **`market_trading`** is set by `tech.commerce.1` (Barter, the market's own
  prerequisite). Constants `market.lot_size` 100 (units in a lot), `market.price_step_pct` 4,
  `market.spread_pct.base` 30, `market.spread_pct.min` 2, `market.price_min_pct` 30,
  `market.price_max_pct` 300, `market.drift_period_ticks` 20, `market.drift_hundredths` 2. Player
  stat **`market_spread`** (the spread before modifiers is the constant): Banking
  (`tech.commerce.4`), Globalisation (`tech.commerce.8`) and Post-scarcity Markets
  (`tech.commerce.12`) take 6 points off each, 30 → 12. Computer player weights `market.sell_above`
  1500, `market.wealth_reserve` 300, `market.buy_max_pct` 140, `market.sell_min_pct` 70,
  `market.lots_per_thought` 1 (`docs/computer-player.md`, "At the market").
- **Prices are per player.** One player's selling does not cheapen another's goods: simpler, the
  same in single player and against computer players, and nobody can ruin a rival's market from
  across the map. Each player and tradeable resource has a **pressure** in hundredths of a lot,
  0 at rest. The price of a lot is `base × (10 000 + step × pressure) / 10 000`, the pressure held
  between the bounds where the price is `price_min_pct` and `price_max_pct` of the base. A lot
  bought adds 100 to the pressure, a lot sold takes 100 off. Buying costs the price at the
  pressure before the lot plus half the spread, rounded up; selling pays the price at the pressure
  after the lot less half the spread, rounded down. So a lot sold straight back meets the same
  price less the whole spread and always loses, even at a bound. Every `drift_period_ticks`
  (at the start of each tick divisible by it, before that tick's trades) each pressure moves
  `drift_hundredths` toward 0: one
  lot of pressure is gone in 50 game seconds. The spread is `market.spread_pct.base` after the
  player's `market_spread` modifiers, at least `market.spread_pct.min` (and at most 198).
- **Orders.** `buy`/`sell` are given to an own complete market (`IncompleteExecutor`,
  `InactiveExecutor` otherwise), need the rule (`SimError::NeedsMarketTrading`), a tradeable
  resource the player's age has discovered and that is not the currency (`NotTradeable`), and the
  price or the lot in store (`InsufficientResources`). They are applied as they leave `pending`,
  in submission order, at the price of that moment, and leave the market's queue alone (like
  `set_rally`); one whose money or goods are gone by then fails `cannot_afford` (reported like any
  failed order). Read-only: `World::market_quote(player, resource)` (lot, price, base, buy, sell,
  spread), `market_spread`, `market_pressure(s)`, `tradeable_resources`, `can_trade`,
  `market_currency(entity)`.
- **State.** `World.market`, only the pressures that are not 0, enters the state hash only while
  one is not 0 (worlds where nobody trades keep their hashes), and is part of the snapshot.
- **PART B, save format 13.** `market_pressure(player_id, resource_id, pressure)`, only rows that
  are not 0. Format 12 saves are refused.

## Capture and difficulty (task 67)

Owner's choice (2026-11-16) after tasks 61 and 65 ended most duels in stand-offs:

- **Workers do not hold a city.** A unit holds or contests a struck-down city only when
  it is armed and neither gathers nor builds (no `et_harvester` row, no `et_builder`
  row): `World::holds_cities(type)`. Workers still fight back; they no longer keep a
  city at 0 hit points from falling, nor restore it.
- **A held-down city falls.** A neutralised city with armed attackers in its economic
  radius starts a countdown for the strongest attacking team; after
  `city.capture_hold_ticks` (new constant, 2400 ticks = 2 game minutes) it is captured
  by that team's strongest player even though defenders remain. A change of attacking
  team restarts it; no attacker in the radius ends it. `World::capture_countdown(city)`
  and `World::capture_countdowns()` give the team, start and end tick (for the HUD of
  the city's owner). While the countdown runs the owner's builders cannot restore the
  city by repair (the order waits).
- **State and saves.** The countdowns are hashed (only while one runs, so other worlds
  keep their hashes), in the snapshot (`Snapshot::sieges`) and saved in the new PART B
  table `city_siege(city_id, team, since)`. Save format 5; format 4 saves are refused.
- **`ai_difficulty` columns** (all with defaults, so older rows load):
  `grace_ticks` (no attack or raid on enemy towns, and no orbital strike, before this
  tick), `first_wave_max` (most units in a wave of the main army; 0 = no limit),
  `wave_growth_per_hour` (units the limit grows per game hour after the grace),
  `army_vs_enemy_pct` (the army stays within this share of the largest enemy army seen,
  never below eight population; 0 = no limit) and `economy_pct` (share of the profile's
  workers kept). Seed values:

| Difficulty | grace_ticks | first_wave_max | wave_growth_per_hour | army_vs_enemy_pct | economy_pct | reaction_ticks |
| --- | --- | --- | --- | --- | --- | --- |
| easiest | 57600 (48 min; 36000 until task 78) | 5 | 10 | 40 | 50 | 600 (was 200) |
| easy | 54000 (45 min; 12000 until task 78) | 8 | 20 | 80 | 80 | 300 (was 120) |
| moderate | 3600 (3 min) | 0 | 0 | 0 | 100 | 60 |
| tough, tougher, toughest | 0 | 0 | 0 | 0 | 100 | as before |

  The brain does not know which player is a person, so the grace and the wave limit
  apply to every opponent.

  Task 78 (the owner wants easy kept gentle, 2026-11-17): with a 10-minute grace the easy
  level's first wave came at about 19 minutes from 0.0.69 on (about 44 in 0.0.67, held back
  by the attack ratio rather than the grace). Task 78's economies no longer stall, so the
  armies of the low levels are also ready sooner: with 30 minutes the easiest level's first
  wave came at 36 minutes (42 in 0.0.69). `grace_ticks` is now 54000 (45 minutes) for easy
  and 57600 (48 minutes) for easiest, so both come no earlier than in 0.0.67 and 0.0.69 and
  easiest last; nothing else changed (`rom first-attack`, `docs/computer-player.md`).

## Counters and siege (task 71)

The owner's rule (2026-11-16): a unit or armed building that outranges and beats a target
type has a countermeasure in the same age that is effective now or after one research.
`rom fight-matrix` measures it (see `docs/computer-player.md`, "Every strength has an
answer"). What the data needed to pass it, all ordinary rows and no schema change:

| Change | Was | Now | Why |
| --- | --- | --- | --- |
| Siege armour class (generator, siege line) | `structure` | `vehicle` | Arrows did a fifth of their damage to siege, so nothing of the first ages could hurt it; engines and gun carriages are vehicles. |
| Siege `et_weapon.range_min` (generator `SIEGE_DEAD_ZONE`) | 0 | 3000 | Siege outranged and beat every unit of its age. Units that close in now stand under its aim, so fast melee (scouts, cavalry) sallying out beats it. |
| `damage_matrix` melee → structure | 50 % | 150 % | In the first age no siege exists; spearmen must be able to break a city of their age. |
| `combat.attrition_per_period` | 60 hp | 30 hp | Attrition left any wave in enemy land at 1 hp within a minute, before supply exists (second age on); no first-age army could take a city. |
| `et_gather_site.node_radius` of node sites (generator `NODE_REACH`) | 6000 | 7000 | A town whose rock lay at the edge of its economic radius found no place for a mine and had no metal for the whole match (traced duel draws). |
| Tower first age (generator `BUILDINGS`) | ancient | classical | Nothing of the first age answers a tower at equal cost; from the classical age the catapult does. The `turtle` opening builds a barracks and six spearmen instead of two towers. |

Siege with a weapon whose `range_min` is set is read by the computer players as "needs an
escort" (`rom-ai`, by the column, not by name).

## Unit series: warship classes and building upgrades (task 92)

Details and the tables as built: `docs/unit-series.md`, section 11. No schema change.

- **A series is its `ui_group`.** Each unit's `upgrades_from_id` points at the one before it in
  the same `ui_group`; the generator's `SHIP_CLASSES` gives the warships and starships their
  class (`frigate`, `destroyer`, `line_ship`, `strike_ship`; `corvette`, `space_frigate`,
  `space_cruiser`, `battleship`), so classes coexist and an upgrade retires only its own class.
  New definitions (Men-of-War, key `unit.ship_of_the_line`; Dreadnoughts; their upgrade
  researches, the two building upgrades, the refits Fire Control and Armour Belts) are written
  after every other generated definition (the generator's `late` steps).
- **Hitting ships**: the city, tower and fort lines, the siege line and the bomber line have an
  `et_weapon_target` row for `fluid`.
- **Building upgrades** `tech.up.vehicle_plant` (stable) and `tech.up.factory` (siege workshop),
  like Farm → Automated Farm; the new building takes the old one's palette after its own.
- **Superseded buildings**: `Rules::is_superseded_building(def, active)` — a researched
  `tech_effect` `upgrade_type` from the building, and no worker slots
  (`et_gather_site.slots_max = 0` or no gather site). The simulation refuses to construct such a
  building (`SimError::Superseded`); a farm, woodcutter's camp, mine, oil well or university stays
  buildable beside its automated successor.
- The damage class `kinetic` and its matrix rows (task 75) are written at the start of the
  generated block, before the starship guns that use it.

## Projectiles you can see (task 95)

The owner (2026-11-18): projectile weapons throw projectiles you can see; big ones have a model,
small ones are lines, laser beams glow. What a weapon throws could not be told from its other
columns (a shell, a rocket and a missile are all `explosive`; a sling stone, an arrow and a bolt
all `pierce`), so it is a column of its own:

`et_weapon.projectile TEXT NOT NULL DEFAULT 'none'`, one of `none`, `arrow`, `bolt`, `stone`,
`bullet`, `shell`, `rocket`, `missile`, `torpedo`, `beam`, `plasma`, `railgun`.

- **Presentation only.** `rom-rules` loads it (`Weapon::projectile`, enum `Projectile`); the
  simulation never reads it and nothing hashes it, so the playthrough hash is unchanged. The
  views draw each shot as its kind from the weapon's cooldown (`rom-app`, `recent_shot`), the
  render thread flies it (`rom-render`, `projectiles.rs`).
- **Generated.** `PROJECTILES` in `db/content/generate.py` names the kind per armed type (and per
  `(type, slot)` for a second weapon: the destroyer's anti-aircraft gun, bombardment from orbit).
  The generator stops when a weapon with a reach (`range_max > 0`) has no kind; melee blows keep
  `none`. No row was added or moved: definition and weapon ids are unchanged.
- **Flight time (task 102).** The owner (2026-11-19): damage is "counted when the projectile is
  supposed to land … by the numbers", the flight is pure animation. Every weapon whose
  projectile travels has a `projectile_speed` (milli-tiles per game second): `PROJECTILE_SPEED`
  in `generate.py` gives one per kind, `PROJECTILE_SPEED_OF` the few weapons that differ.
  When it fires, the simulation computes the landing tick, `fire tick + max(1, distance ×
  tick rate / projectile_speed)`, with the distance it measured for the shot (to a building's
  footprint, a unit's edge, an orbiting target's ground subpoint), and queues the hit; at that
  tick the hit lands on the target by id wherever it is (no collision, no dodging). A target
  that dies or is neutralised first wastes the hit. Nothing the renderer does reaches the
  simulation: the views read the landing tick of the hit on its way
  (`World::hits_in_flight`) and draw the flight ending then, so the drawn speed is the data's.

  | Kind | Tiles per second | Why |
  | --- | --- | --- |
  | arrow | 15 | slow enough to watch on its arc: 0.3 s over an archer's five tiles, 0.7 s from a tower |
  | bolt | 20 | a crossbow's flatter, faster shot |
  | stone | 10 | sling stones and siege stones on a high arc, the slowest thrown thing |
  | bullet | 40 | fast: two or three ticks over a rifle's reach |
  | shell | 24 | a tank's, a gun tower's or a warship's shell; the howitzer lobs its shell at 14 |
  | rocket | 16 | a rocket's motor, slower than a shell |
  | missile | 20 | a guided missile |
  | torpedo | 8 | unused (no submarine) |
  | plasma | 30 | a visible glowing charge, slower than a bullet |
  | beam | 0 | light: hits the tick it fires, drawn as a held, flickering line |
  | railgun | 0 | a rail slug would cross the longest reach within a tick or two: hits at once, drawn as a fading line |

  The longest flight in the data is one second (a catapult or a ground battery at full
  reach). Task 106: to and from orbit the flight is longer. Constant `space.orbit_height`
  (**8000** milli-tiles, the height the planet view draws). A shot with exactly one end in
  orbit (ground or air at a ship in orbit, a weapon in orbit at the ground) flies
  `isqrt(ground² + height²)` for its landing tick, so one straight overhead no longer takes the
  one-tick minimum. Weapon reach stays measured on the ground (task 58); orbit to orbit and
  ground to ground are unchanged. `World::flight_distance` is the one place the rule lives and
  `rom-app` calls it too.

  Balance at 0.0.95 → task 102: the fight matrix keeps 255 strengths with none unanswered;
  duels over seeds 1..100, balanced:moderate against balanced:easiest 95–0 (5 draws, mean tick
  52532) → 94–1 (5 draws, 56192), moderate against moderate 50–41 (9 draws, 67173) → 48–42
  (10 draws, 70179). Fights last a little longer (volleys at targets that die first are
  wasted); no speed needed changing. Screenshots: `img/102-archers.png`, `img/102-shells.png`
  (scripts `tasks/102-*-script.txt`).
- `torpedo` is in the vocabulary and drawn, but no weapon uses it yet (there is no submarine).

## A building that changes at an age advance (task 146)

No schema change. The owner (2026-11-23): "we should have energy collecting building before the
fusion age (lets make a nuclear reactor that will become a fusion reactor in the age before
fusion)". `bld.nuclear_reactor` (Nuclear Reactor) is built from the automation age (`age.automation`
and `tech.science.8`), one per city, on the fusion plant's 4 × 4 footprint and plain ground, and
yields energy without workers (`et_gather_site.self_income` 35; the fusion plant's is 40).
`res.energy.first_age_id` is now `age.automation` (it was `age.fusion`), so energy shows from
that age on.

- **The change is an effect of the age technology.** `tech.age.fusion` carries a second
  `tech_effect` row besides its `set_age`: `upgrade_type` from `bld.nuclear_reactor` to
  `bld.fusion_plant`. Completing any research applies its `upgrade_type` rows to every entity of
  the researching player of the `from` type (`World::complete_research`), so the age advance
  changes each reactor, finished or under construction, into a fusion plant in place, keeping its
  hit-point percentage, as the research at a gather site does for the automated sites. No code
  knows the keys.
- **Its line.** `bld.fusion_plant.upgrades_from_id` is the reactor: the plant is the next step
  of the reactor's line (the computer players' catalogue and the colony caps read lines, never
  keys).
- **Superseded at Fusion.** The reactor has no worker slots, so the researched `upgrade_type`
  takes it out of the build menu (`Rules::is_superseded_building`); fusion plants are built
  directly from then on. A player who starts in the fusion age or later never sees it.
- **Look.** `bld.nuclear_reactor@automation` (white composite: a containment building under a
  team-coloured dome, a hyperbolic cooling tower with a team crown, a vaulted turbine hall and a
  switchyard); from the fusion age the entity is a fusion plant and wears its look. Its sounds are
  its own (`tools/sounds`, appended after every other type).

The rules fingerprint changes with the new rows. `generate.py` writes the reactor after every
other definition (`nuclear_reactor`), so every earlier id stays.

## The Freedom age (task 146)

The owner (2026-11-23): "the information age should be called the freedom age". Only names
change: `def.name` of `age.information` is "Freedom Age" and of `tech.age.information` "Advance
to Freedom" (the generator's `AGE_NAMES`); the key `age.information`, the model ids
`…@information` and the music files `information.*` keep their names.

## Placement and resource discovery (task 41)

Every building with an `in_city_radius` row also has an `in_own_territory`
row, including automated variants copied by the content generator. The
first checks all four corners of the building footprint against the economic
radius of one complete city using unfolded cube-face distance (task 52: it
used to test only the centre, so a farm or mine could stand half outside the
city that serves it). The second checks every footprint tile against
the owner's national border. Economic radius growth never grants territory.

**Free colonisation (task 52, owner decision).** `age.free_colonisation`
(0/1, seed: ordinal 4 and later, the enlightenment age on) lets a player of
that age found a **city** anywhere not inside another player's territory: the
`in_own_territory` row of a city type then means "no footprint tile owned by
another player", and the refusal is `InForeignTerritory`. Before the flag the
rule is unchanged (own borders only, except the first city). Only city types
change: every other building keeps `in_city_radius` and `in_own_territory`.
City spacing, the city limit and every other row still apply. A construct
order whose site is on another body than the builder is refused (`OtherBody`);
the placement preview turns red when no selected builder stands on the site's
body, and clicking says so. Carrying units across space: see "Carriers, landing
and colonies" above.

**City area as a setup option (task 158, owner 2026-11-25).** `rom_sim::MatchOptions::city_area`
(`CityArea::LimitsBuilding`, the default, or `Free`; `--city-area on|off` on the game, `rom
duel`, `rom ai-match` and `rom serve`, the setup screen's **City area**, the network setup's
`MatchSetup.city_area`). Off (`Free`), an `in_city_radius` row passes inside one complete own
city area **or** when every footprint tile lies in the owner's borders, and its refusal is
`NotInOwnTerritory`; every other row (the building's own `in_own_territory`, terrain, spacing,
node and enemy-city rows) is unchanged, and so is wall placement. The economic radius is
untouched: it grows with the buildings inside it, is drawn, and is what a city's shield covers
(task 157); a building outside every area belongs to no city (`city_id` NULL). Hashed only when
off; saved in `match.city_area` (format 22). No seed row: the option is not in `match_option`.
Pictures: `img/158-setup-city-area.png` (the Play form), `img/158-farm-outside-city-area.png`
(a farm built outside the dashed ring, `docs/tasks/158-farm-script.txt`).

**The economy's pace as setup options (task 159, owner 2026-11-25).**
`rom_sim::MatchOptions::pace` (`EconomyPace { gather_rate, research_time }`, percent; set with
`World::set_pace` before the first tick). **Gathering rate** 100 (default), 50, 25, 10, 5 or 2;
**research time** 100 (default), 200, 400, 1000 or 2000. Values outside the lists are refused
(`SimError::InvalidOptions`, and at every parser). `--gather-rate <pct>` and
`--research-time <pct>` on the game, `rom duel`, `rom ai-match`, `rom replay-hash` and `rom
serve`; the setup screen's **Gathering rate** and **Research time**; the advanced Game group's
fields of the same names; presets' `gather_rate` and `research_time`; the network setup's
`MatchSetup.gather_rate` and `research_time` (the lobby shows them when not 100 %); the match
report's options line, `report.json`'s `gather_rate` and `research_time`.

- *Scaled by the gathering rate* (everything a player draws from the world), all through
  `World::credit_gathered`: the income of `pay_income` (workers in gather slots, a site's
  `self_income`, an automated colony's own income and its share of its sites, relics kept in a
  temple, claimed presence resource bodies), trade deliveries (runner and partner alike), and
  `completion_bonus` rows (a building's first yield). The commerce cap applies before the
  scaling, so the economy is the same game at a smaller scale.
- *Not scaled*: starting resources and grants, refunds (cancel, delete, a defeated player's
  queue), plunder (`plunder_pct` of what a destroyed or captured entity cost: a share of what the
  enemy paid),
  market trades, ascension descent resources, costs, unit speed, combat, build and training
  times, deposits.
- *Remainders.* What falls below one unit stays, in hundredths, in `gather_remainders` per
  player and resource (removed at zero; hashed and saved), so 2 % of a trickle still adds up.
  At 100 % nothing is scaled and no remainder is kept.
- *Research time* multiplies `production_duration` of every research after its
  `research_time` modifiers, age advances included, and the ascension's duration. Prices are
  unchanged.
- *Shown rates.* `World::income` gives `effective` and `trade` at the gathering rate (the HUD's
  +N, the computer players' view), as do `trade_routes`, `trade_offer` and `body_yield`; the
  HUD scales the commerce cap it shows beside the rate alike.
- Hashed only off the defaults (the options and the remainders), so default matches keep their
  hashes; the rules fingerprint is unchanged. No seed row.

Calibration (one `balanced:moderate` computer alone, `rom ai-match --universe --seed 1`): the
last age (ordinal 13) came at 149 561 ticks (2.1 h) at 100/100, 465 601 (6.5 h) at 25/400,
1 101 841 (15.3 h) at 10/1000 and 8 479 521 (117.8 h, about 4.9 days) at 2/2000; at 2 % the
first ages are the slow ones (the classical age at 7.7 h, the eighth at 99 h), the automated
ages then go quickly. 2/2000, the slowest setting, gives this computer about five days of game
time to the last age.

Pictures: `img/159-setup-economy-pace.png` (the Play form at 10 % gathering, 400 % research
time), `img/159-hud-rate-gathering-10.png` and `img/159-hud-rate-gathering-100.png` (the same
five farmers at the same moment: food +5 against +50 a period, commerce 5/7 against 50/70;
`docs/tasks/159-hud-script.txt`).

`resource.first_age_id` is a discovery gate. Terrain-to-resource associations
come from existing `et_gather_site.resource_id`, `needs_node_terrain_id` and
`on_terrain` placement rows. A terrain with several yields is known from the
earliest of their discovery ages; any ungated yield makes it ordinary ground.
Terrain with no gather yield stays known. This keeps plain land known when a
late-age gather site also uses it. A required but undiscovered deposit makes
placement preview return the discovery age; oil wells remain locked through
their ordinary production prerequisites too.

The app caches the current player's known tile identities by age and viewed
body and publishes the unknown terrain kinds to the renderer (task 44).
Undiscovered deposits use plain land on the surface and minimap; another
player's age never reveals them. On discovery, only chunks touching changed
tiles receive new colour/material buffers, and the minimap refreshes. The
`oil` land program supplies glossy black pools; `water` remains a sea material.

## Saving a match (task 62)

`rom-save` writes one SQLite file per save, containing PART B of the schema
only (the rules are not repeated). `rom_sim::World::snapshot()` exports the
complete simulation state as plain structs and `World::restore(rules, snapshot)`
rebuilds a world; `rom_save::save` / `load` map the snapshot to the tables and
back. No SQL runs in `rom-sim`.

- **Rules check.** `match.rules_version` (the `schema_meta` row) and
  `match.rules_fingerprint` (FNV-1a over the loaded rules) must equal those of
  the rules passed to `load`; otherwise `SaveError::RulesMismatch`. The
  fingerprint is only comparable between builds of the same rules model.
  `read_header` returns version, tick and match settings without the rules.
- **Match settings.** `MatchInfo` carries the seed, `map_style` key, speed, the
  caller's own `match_setting` pairs and one seat per player (`player.slot`,
  name, controller, AI profile, colour). Players without a seat get the next
  free slot.
- **Computer players.** `player_ai_state(player_id, profile_key, difficulty_key,
  version, rng_state, opening_seq, data)` stores what `rom_ai::Brain::save` hands
  in per player and returns it unchanged (task 64 added the generator state and
  opening step, which `Brain::restore` checks against the bytes; format 2).
- **Rally points.** `entity_rally(entity_id, target_entity_id | x, y, face, body)`
  (task 64). Not part of the state hash while no producer has one, so worlds
  without rally points keep their hashes.
- **Economy pace of task 159 (format 24).** `match.gather_rate`, `match.research_time` and
  `gather_remainder(player_id, resource_id, remainder)` (hundredths, 1 to 99). Files written
  before format 24 have none of them and are refused (owner: no conversion).
- **City area option of task 158 (format 22).** `match.city_area` (`on` or `off`). Formats 13
  to 21 load with `on`, the rule they were played under.
- **City names of task 144 (format 20).** `city.name` (see "City names"). Formats 13 to 19
  load with their cities named from their owners' lists in id order.
- **Drone grids of task 128 (format 17).** `entity_drone_cloud(entity_id, body, face, x, y,
  facing, ready, rebuild, join_slot, join_face, join_x, join_y)`, `pending_hit.drone`,
  `pending_shot.drone` (see "The drone grid"). Format 16 saves load with every grid full.
- **Drone clouds of task 120 (format 16).** `entity_drone_cloud(entity_id, drones, body, face,
  x, y, rebuild)` (see "Drone operators"). Formats 13 to 15 still load, without operators.
- **Aircraft on the ground of task 123 (format 15).** `entity_flight(entity_id, climbed)` (see
  "Aircraft land and take off"). Formats 13 and 14 still load, with every aircraft in the air.
- **The match end and report of task 117 (format 14).** `match_human`, `match_end`,
  `player_stat`, `player_gathered`, `player_age_reached`, `player_history` (see "How a match
  ends"). Format 13 saves still load, without them; format 12 and older are refused.
- **Market prices of task 107 (format 13).** `market_pressure` rows (see "Market trading").
  Format 12 saves are refused.
- **Light system space of task 99 (format 12).** `player_vision.explored` holds (word, run)
  pairs in LEB128 instead of raw words, and is empty when nothing is explored;
  `surface.territory` is empty when no tile is owned. An untouched star system writes its
  terrain as one run and almost nothing else. Format 11 saves are refused.
- **Bringing life of task 77 (format 11).** `body_dormant` and `body_life` rows (see
  "Bringing life"). Format 10 saves are refused.
- **Ascension of task 76 (format 10).** `player.origin_faction_id`, `ascended_tick`,
  `ascensions` and the status `ascended` (see "Ascension"). Format 9 saves are refused.
- **Ground stains of task 80 (format 9).** `surface_stain` rows, oldest first per surface
  (see "Ground stains"). Format 8 saves are refused.
- **Paused governors of task 73 (format 8).** `entity_flag` rows `governor_paused`. Format 7
  saves are refused.
- **Held-down cities of task 67 (format 5).** `city_siege(city_id, team, since)`: a
  struck-down city's capture countdown. In the state hash only while a city is held
  down. Format 4 saves are refused.
- **Orders of task 65 (format 3).** `"order".pace` (a formation's walking speed
  cap, milli-tiles per second) and `"order".engaged_id` (the enemy a `guard` is
  fighting off), NULL otherwise; both enter the state hash only when set, so
  existing worlds keep their hashes. `order_failure(player_id, seq, tick,
  order_id, executor_id, kind, subject_def_id, target_entity_id, target_x..body,
  reason)` keeps each player's latest 64 failures of accepted orders
  (`World::order_failures`), oldest first; a report, not hashed. Format 2 saves
  are refused.
- **Encoding.** Positions are four columns; unsigned 64-bit values are stored as
  the same bits read as signed; terrain and territory grids are run-length
  encoded blobs; explored bits are raw words; `entity.move_remainder` is
  decimal text. Foreign keys to PART A document the link and are not enforced
  (the save has no PART A tables).
- **Changed from the early design.** Dropped as unused by the simulation:
  `map_grid`, `player_explored`, `diplomacy`, `diplomacy_offer(_item)`,
  `market_price`, `unit_group`, `entity_effect`, `trade_route`;
  `match_setting` is free-form text. `entity` lost rally and node columns and
  gained `max_hp`, `los`, `killer_id`, `gather_slots`, `idle_ticks`,
  `heal/attrition_remainder`, `location`; `entity_cooldown` is keyed by weapon.
  Added: `surface`, `surface_touched`, `path_request/waiter/search/open/node`,
  `player_vision`, `vision_memory`, `vision_building`, `vision_viewer`,
  `entity_transit`, `entity_flag`, `income_term`, `income_remainder`,
  `order_payment/waypoint`, `pending_hit/shot`, `body_position/marker/claim`,
  `emitter`, `city`, `city_widener`, `match_victory`; orders gain the pending
  queue (`place`), trade-route and walking columns.
- **Rebuilt on load, not stored.** The spatial buckets, path occupancy, the
  per-tile viewer counts (rebuilt from `vision_viewer`), the motion cache,
  income rates (summed from `income_term`) and gather-slot occupants (from
  `entity.work_site_id`). The state hash after a load equals the hash before
  the save; the save tests also run both worlds on for several hundred ticks.

### The save menus (task 63)

The app writes saves on the simulation thread, between two ticks, and starts a game from
one with `--load <file>` (a new process: the launcher's list and the in-match menu both
use it). Files are in `$XDG_DATA_HOME/ascension/saves/` (else `~/.local/share/ascension/saves/`;
`%APPDATA%` on Windows, `~/Library/Application Support` on macOS). A manual save is
`save-<name>.rom`; quick saves rotate through `quicksave-1..3.rom`, autosaves (every 5 game
minutes, `--autosave <minutes>`, 0 is off) through `autosave-1..3.rom`.

**Stored beside the world** (`match_setting` rows and the match row, all read back by
`read_header` without loading the rules):

| Key | Meaning |
| --- | --- |
| `name`, `kind` | the player's name for it; `manual`, `quick` or `auto` |
| `saved_at` | Unix seconds, for the date column and the newest-first order |
| `age` | display name of player one's age at the save |
| `args` | the launch arguments that decide the map (`--seed`, `--style`, `--sphere`, `--universe`, `--single-planet`, `--map-size`, `--players`, `--factions`, `--ai`, `--age`, `--sun`, `--watch`), one per line |
| match `seed`, `map_style`, `speed_pct`; seats | the seed, the style key, the game speed; one seat per player (controller, AI profile) |

**Rebuilt on load, not stored.** Loading regenerates the planet or map from `args` and the
seed (the terrain pictures, the universe, the model looks, the sun), builds a throw-away
fresh world for the starting data, and replaces its world with the restored one. Rebuilt too:
fog of war and hidden-terrain views (from the restored vision), selection and control groups
(empty), the camera (placed over player one's first city, on its planet), the viewed body
(the home planet), the sun's phase, the event line, the computer players' memory comes from
`player_ai_state`. Game speed comes from the match row. The game starts **paused**; Pause
(HUD button) or Resume (menu) runs it. A save made with other rules (version or fingerprint)
is listed with a refusal and cannot be loaded; `--load` of one exits with the same message.

## Star catalogue (task 59)

`star`, `constellation` and `constellation_line` are generated by
`db/content/generate.py` from `db/content/stars.py`. Positions are published
values from primary catalogues, never invented; the map positions made from them
are curated so that no two systems lie on top of each other (task 161, below):

- **Positions and distances.** The Hipparcos catalogue, new reduction: F. van
  Leeuwen, "Validation of the new Hipparcos reduction", A&A 474, 653 (2007),
  ESA, as served by the CDS (VizieR catalogue I/311, `hip2`). We use the
  right ascension and declination (ICRS, epoch 1991.25), moved to J2000.0 with
  the catalogue's own proper motions, and the parallax; the distance is
  1 / parallax. Stored as integers: `ra_mdeg` and `dec_mdeg` in
  milli-degrees, `dist_cly` in hundredths of a light-year (1 parsec =
  3.261564 ly). A star is used only when its parallax is at least three times
  its formal error. The one exception is Deneb, whose Hipparcos parallax is
  too uncertain: its distance is 802 pc from Schiller and Przybilla,
  A&A 479, 849 (2008).
  Acknowledgement: this research made use of the VizieR catalogue access
  tool, CDS, Strasbourg, France (Ochsenbein, Bauer and Marcout 2000, A&AS 143,
  23) and of the Hipparcos data (ESA 1997; van Leeuwen 2007).
- **Proper names and constellations.** The IAU Catalog of Star Names
  (IAU-CSN, IAU Division C Working Group on Star Names; IAU products are
  released under Creative Commons Attribution): the proper name, the
  Hipparcos number and the constellation of each named star. A star has a row
  when the IAU gives it a name and it is brighter than magnitude 4.5 (or is
  within about 6 parsecs, which brings in Proxima Centauri and Barnard's
  Star), or when it has a Bayer designation and is brighter than 4.5.
  The constellation names and abbreviations are the IAU's.
- **Bayer and Flamsteed designations.** The Yale Bright Star Catalogue
  (D. Hoffleit and W. H. Warren, 5th revised ed., 1991; CDS V/50), matched to
  Hipparcos by Henry Draper number (from the IAU list) or, failing that, by
  position within 72 arcseconds and magnitude within one. Stars with a proper
  name keep it; the others are called by their Bayer name ("Iota Aurigae",
  "Zeta1 Lyrae") or Flamsteed name ("61 Cygni"). Two rows are renamed:
  Alpha Centauri (the IAU name of the A component is Rigil Kentaurus) and
  Epsilon Eridani (IAU name Ran). Toliman, the B component (the Hipparcos
  row Alpha2 Centauri), and Proxima Centauri, the C component, are merged
  into Alpha Centauri by task 161 (below) and have no row of their own.
- **Map coordinates.** Computed once in the generator, in `position()`:
  `r = 100 * sqrt(distance in light-years)`, then
  `x = r cos(dec) cos(ra)`, `y = r cos(dec) sin(ra)`, `z = r sin(dec)`,
  rounded to integers, then curated (below). The map generator and the
  simulation read the integers only.
- **Curated map positions** (task 161; the owner decided this on 2026-11-26).
  `curate()` in `stars.py`, after `position()`; `generate.py` prints what it
  did. The minimum spacing D is `MIN_SPACING` = 60 map units (3D); since task
  162 the cluster view spreads each cluster for the system view whatever D is
  ("Star clusters" below).
  - *One system per physical star.* Rows whose Bayer designations differ only
    by the component number (a bare letter counts as one: "Alpha1 Centauri",
    "Alpha2 Centauri") and that lie closer than D are one row: the one with an
    IAU proper name, else the lowest Hipparcos number, named by its proper
    name or by its designation without the number. Known physical companions
    (`COMPANIONS` in `stars.py`) join their primary whatever their designation
    and distance; the primary keeps its place and name. The list holds one:
    Proxima Centauri (HIP 70890) into Alpha Centauri (HIP 71683), since
    Proxima is gravitationally bound to Alpha Centauri AB (P. Kervella,
    F. Thévenin and C. Lovis, "Proxima's orbit around Alpha Centauri",
    A&A 598, L7, 2017). A figure line to a dropped row goes to the kept one;
    a line from a star to itself is dropped. Five rows are merged, 745 to 740
    systems: Alpha2 Centauri and Proxima Centauri into Alpha Centauri, Delta3
    Tauri into Secunda Hyadum, Theta1 Tauri into Chamukuy, Delta2 Gruis into
    Delta Gruis (the Delta1 row). Xamidimura and Pipirima (Mu1 and Mu2
    Scorpii) lie 60.6 apart and stay two systems.
  - *Every other close pair moves apart.* Stars are placed in a fixed order,
    the named ones (name differs from designation) first, then by Hipparcos
    number; a star closer than D to one placed before it moves. First along
    its line of sight: the smallest change of map radius, farther or nearer,
    ties farther, within 25 % of it; its direction, and so the sky seen from
    the Sun and `glon_mdeg`, are unchanged. Otherwise to the nearest free
    point, searched on growing shells of 512 fixed directions about its
    catalogue place. `ra_mdeg`, `dec_mdeg`, `dist_cly` and `glon_mdeg` keep
    the catalogue values; only `x`, `y`, `z` move.
  - *Moved* (25 stars, map units, all along the line of sight): Kappa
    Phoenicis 17, Gamma Persei 2, Iota Persei 29, Taygeta 48, Maia 6, Merope
    56, Alcyone 119, Atlas 119, Secunda Hyadum 16, Upsilon Tauri 59, Ain 21,
    Chamukuy 36, Phi1 Orionis 39, Rho Geminorum 5, Gamma Centauri 53, Alcor
    38, Arcturus 27, Kappa Centauri 47, Epsilon Serpentis 26, Beta Apodis 49,
    Shaula 44, Eta Sagittarii 8, Iota2 Cygni 23, Beta Sagittae 22, Eta
    Cephei 20.
  - *Nearest-neighbour distances*, before and after: smallest 4.9 and 60.0,
    5th percentile 50.3 and 60.9, median 204.7 and 205.7; stars with a
    neighbour within 40 units, 28 and none; within 60, 51 and none.
  - A saved universe keeps the systems it was made with.
- **Galactic longitude** (task 89). Computed once in the generator, in
  `galactic_longitude()`, from the stored right ascension and declination in the
  IAU 1958 galactic frame (J2000: north galactic pole at RA 192.859, Dec +27.128;
  galactic longitude of the north celestial pole 122.932):
  `l = 122.932 - atan2(cos(dec) sin(ra - 192.859), sin(dec) cos(27.128) -
  cos(dec) sin(27.128) cos(ra - 192.859))`, in `glon_mdeg` (milli-degrees,
  0..359 999). It numbers the galactic quadrants as astronomers do: I 0-90,
  II 90-180, III 180-270, IV 270-360 degrees (173, 162, 183 and 222 stars). Hand
  checks against the published galactic coordinates: Sirius 227.230 (published
  l = 227.23), Vega 67.449 (67.45), Deneb 84.285 (84.28); and the galactic
  centre's own position, RA 266.405 Dec -28.936, gives 0.000.
- **Hand checks** (positions against the Bright Star Catalogue's own J2000
  coordinates and the IAU list's; parallaxes against the Bright Star
  Catalogue's older trigonometric values):

  | Star | Hipparcos RA, Dec, parallax | Distance | Map x, y, z | Checks |
  | --- | --- | --- | --- | --- |
  | Sirius | 101.287, -16.716, 379.21 mas | 8.60 ly | -55, 275, -84 | BSC 06h45m08.9s -16d42m58s (= 101.287, -16.716); BSC parallax 0.375" |
  | Vega | 279.235, +38.784, 130.23 mas | 25.04 ly | 63, -385, 313 | BSC 18h36m56.3s +38d47m01s; BSC parallax 0.123" |
  | Betelgeuse | 88.793, +7.407, 6.55 mas | 497.95 ly | 47, 2212, 288 | BSC 05h55m10.3s +07d24m25s; BSC parallax 0.005" |
  | Alpha Centauri | 219.902, -60.834, 754.81 mas | 4.32 ly | -78, -65, -181 | IAU-CSN 219.902066, -60.833975; BSC parallax 0.751" |

  For Sirius: r = 100 sqrt(8.60) = 293, z = 293 sin(-16.716) = -84, x = 293
  cos(-16.716) cos(101.287) = -55.
- **Stick figures.** `constellation_line`: written for this project from the
  traditional figure of each of 22 constellations (which star joins which),
  not copied from any existing line file. A line exists only where both of its
  stars have a row. They are display only and belong to one constellation.
- **Names.** Only real astronomical names (IAU proper names, Bayer and
  Flamsteed designations, the common names of the IAU list such as "Barnard's
  Star"); nothing from fiction or games. Planets are "<star> II", resource
  bodies "<star> belt" and "<planet> moon" (a second of a kind takes a letter).
- **Star clusters** (task 162; the owner decided this on 2026-11-26).
  `star_cluster(id, name, x, y)` and `star.cluster_id`, generated by
  `cluster()` in `stars.py` from the curated positions; `generate.py` prints
  what it made. Display only: the cluster, quadrant and galaxy views
  (`world-model.md` "Five zoom levels"); nothing in the simulation reads them.
  - *Grouping.* Two stars link when they lie within a bound of each other:
    150 map units plus 10 % of the nearer one's map radius (`LINK_BASE`,
    `LINK_PCT`; the map is denser near the Sun). Linked pairs, nearest first
    (ties by Hipparcos numbers), join their clusters while the union holds at
    most 7 stars and no two of its stars lie farther apart than twice the
    pair's bound. A cluster of fewer than 3 stars then joins the nearest
    cluster with room whose nearest star lies within twice the bound, in
    order of lowest Hipparcos number until nothing changes; a pair still left
    takes the nearest star of a cluster of more than 3. A star with no other
    star within twice its bound stands alone. Integer arithmetic throughout,
    so the grouping is the same every time (a test re-derives it).
  - *Result.* 160 clusters: 13 of one star, 40 of three, 23 of four, 21 of
    five, 31 of six, 32 of seven. The single stars are Delta Ceti, Alpha
    Sculptoris, Omega Capricorni, Sigma Ophiuchi, Beta Scuti, Zeta
    Monocerotis, Eta Leonis, Mekbuda, Iota1 Scorpii, Iota Canis Majoris,
    Kappa Cassiopeiae, Rho Leonis and Nu Cephei (each 724 map units or more
    from any other star).
  - *Names.* A cluster most of whose stars belong to a traditional open
    cluster takes its name: "Pleiades" (Electra, Taygeta, Maia, Merope,
    Alcyone, Atlas) and "Hyades" (Prima and Secunda Hyadum, Kappa1 and
    Upsilon Tauri, Ain, Chamukuy). Every other one is "<star> group" after its
    leading named star: a star with an IAU proper name before one without,
    then the earliest Bayer letter (Bayer lettered a constellation's stars
    roughly by brightness, and the catalogue stores no magnitudes), then the
    nearer, then the lower Hipparcos number ("Sirius group", "Alpha Centauri
    group").
  - *Ids.* In order of the centre's distance from the Sun (the mean of the
    members' map positions): cluster 1 is the Altair group.
  - *The disk.* `x, y` is the cluster's place on the galactic disk in map
    units: its centre's galactic longitude (from the same IAU 1958 frame as
    `glon_mdeg`) at the centre's map distance from the Sun, its galactic
    latitude dropped; `x` toward longitude 0, `y` toward 90 degrees. Its
    footprint there is its stars' offsets from the centre, flattened onto the
    galactic plane and shrunk to 20 %, plus 25 map units of room
    (`DISK_SHRINK_PCT`, `DISK_ROOM`; the views draw it so). Clusters are
    placed in id order; one whose footprint overlaps one placed before it
    moves along its longitude ray by the smallest change of distance, farther
    before nearer. 42 moved: 41 outward (the most, Atria group 202 and Spica
    group 170) and one inward (Markab group, 12).
- **In a match.** `rom-mapgen` picks a connected region; `star_system` holds
  its stars' names, positions (`x`, `y`, `z`), `star_id`, `constellation` and
  `cluster_id` (task 162; NULL for the celestial home); `star_figure` holds
  the figure lines between them. Save format 6 stores the two tables; format 5
  saves are refused. Save format 25 (task 162) stores `cluster_id`; earlier
  files are refused.

## Ice and snow apart (task 115)

`mat.ice` (Ice: the ice brushes and every planet's polar caps) is a pale glacier blue, #b3d3e8.
Snowfields (`ter.snow`) and the snow on mountain peaks (`mat.mountain` `snow_material`) use the new
`mat.snow`, the old white #e8eef2. Appended at the end of `seed.sql`, so earlier definition ids stay.

## Sounds (task 125)

Presentation only: the simulation never reads these rows, and nothing in `rom-sim` knows about
sound. A `sound` value names a **sound family** of `app/assets/sounds/`: a file name without
`.ogg` and without its variation number. `unit.knight.fight` plays one of `unit.knight.fight.1`,
`.2`, `.3` at random; `voice.knight.select` is a file of its own. `docs/sound.md` says what plays
when.

- **`et_sound(entity_type_id, event, slot, sound, gain_pct)`**, primary key
  `(entity_type_id, event, slot)`: each unit's and building's own sound per event. Events: units
  `select`, `move`, `attack`, `build` (lines), `build_done`, `research_done` (lines), `fire` (a
  ranged weapon's shot; `slot` is its `et_weapon.slot`), `fight` (a melee blow), `moving` (a loop
  while it moves); buildings `select`, `active` (its work loop), `build_done`, `train_done`,
  `research_done` (cues), `fire`. A type without a row for an event is silent there. `gain_pct`
  (100 = the file's own level) is where the owner's changes after listening go.
  The rows are **generated**: `app/tools/sounds/assign.py` chooses every type's sounds from its
  data (class, tags, movement class, weapons, components, first age) and writes
  `tools/sounds/assignment.json`; `python3 tools/sounds/seed.py` (from `app/`) turns that file
  into the block between `-- BEGIN GENERATED SOUNDS` and `-- END GENERATED SOUNDS` at the end of
  `seed.sql`, with the gains of its `GAINS` table; `--check` reports a stale block. The rows name
  their type by key and fail the load if a key is missing. 763 rows for 140 types in 0.0.118.
  The proposed `voice_set` column was not needed: each type's lines are its own files, spoken by
  the voice `assign.py` chose for its kind of speaker, and only the person's own units speak.
- **`ambience(id, sound, view, layer, terrain_id, resource_id, light, gain_pct)`**: the ambience
  beds. `view` 'surface' beds play over a planet, each at the share of the ground in view its row
  selects (a `layer`, one terrain, or the ground near gather sites of one resource; none of the
  three: everywhere), on the day or night side (`light`), fading as the camera leaves the
  planet; 'space' beds play in the system and star-map views. Seed: wind everywhere, waves over
  `fluid`, birds by day and crickets by night over `ground`, the forest over `ter.forest`, the
  rustle of fields near `res.food` gather sites, and `ambient.space`.
- **`order_kind.line`** ('', `move`, `attack`, `build`): the `et_sound` event a unit says when
  given an order of that kind; '' says nothing (training and research are answered by the HUD's
  click). Set by `UPDATE`s in the seed's sounds section.

The schema change keeps both lint views empty; `et_sound` and `ambience` add no definitions. The
rules loader reads them into `EntityType::sounds`, `Rules::ambience` and `OrderKind::line`
(`rom-rules`); the rules fingerprint changes with them, so saves made with earlier rules are refused, as
after any rules change.


## Delete, attack ground and the spaceport's rally point (task 142)

Two order kinds (`order_kind` rows; no other rules data):

- **`delete`** (`needs` '', target none): an own unit or building leaves at once, at the next
  tick boundary, like `raze` and `set_rally` a setting rather than a queue entry. A construction
  site refunds its price × the share of the work not done × `economy.cancel_refund_pct` (100),
  rounded down per resource (`World::delete_refund`); a finished building or a unit refunds
  nothing. A producer's queue and pending orders are refunded as a cancel refunds them, workers
  inside a gather building come out, and a landed carrier sets its passengers down first; a
  carrier with units aboard that has not landed refuses (`PassengersAboard`), and so does a
  finished city (`CannotDelete`: it is captured, never removed). A unit is removed at once; a
  building falls with the dead of the tick, as a razed one: no stain, no killer, no kill or
  plunder. The price is the one the site was charged: its type's ramped price with the site
  itself left out of the ramp count. `raze` now refunds the same amount (before, it used the
  unramped base price; both agree for a player's first building of a type).
- **`attack_ground`** (`needs` weapon, target point, line `attack`): every weapon that throws a
  projectile fires at the point — `projectile_speed` > 0 (instant beams and railguns do not
  qualify: their hit is a ray at a target), a reach (`range_max` > 0), not `consumed`, and an
  `et_weapon_target` row for the layer of the ground there (`ground` on land, `fluid` on water).
  Drone operators send their drones at enemies and do not attack the ground. A ship in orbit
  uses its `orbit_only` weapons at the planet below; elsewhere the other weapons, on the unit's
  own body, never from a ship bound to its orbit. The unit walks (in orbit: moves its ground
  point) until the point is within the reach of one of them, then stands and fires its first
  ready weapon in slot order, every reload, until stopped or given another order; an immobile
  shooter out of reach fails with `out_of_range`. A shot is rolled like any other (accuracy,
  falloff, the layer's `hit_pct`); it lands on the enemy standing on the point (a unit within its
  radius, a building whose footprint covers it), and its splash, if any, reaches enemies around
  the point as from any hit. Splash never hurts own or allied units, so neither does this.
- **Spaceport rally point.** A ship a spaceport trains is launched into orbit before it is
  free to move, and the rally point was only applied to units that came out on the ground: it
  was skipped for every launched ship. A ship now goes on to the rally point when its launch
  ends: in orbit, it moves within the orbit to above the point (or the entity) on the ground and
  stays in orbit there. This holds for ships bound to their orbit and for those that can travel;
  none lands. Orbital craft and aircraft come out on the ground and walk or fly there as before.
- **PART B, save format 19.** `pending_hit` gains `ground_x, ground_y, ground_face, ground_body`
  (NULL for a hit on an entity); a shot at the ground has `target_id` 0, no entity's id.
  `pending_shot` is unchanged: such a shot flies at its `target_*` point. Formats 13 to 18 load
  without shots at the ground. The state hash adds the point only for shots at the ground, so
  other worlds keep their hashes.

## Support units, free repair and builders that do not stand idle (task 140)

No new table or column. Rules data:

- **Supply carts repair vehicles.** `unit.supply_wagon` and `unit.supply_truck` gain an
  `et_healer` row (target tag `mechanical`, range 3000, no energy; 4 hp a second for the wagon
  of the medieval age, 8 for the truck of the industrial age, as the medic and the repair drone)
  and the `healer` role tag. Medics and priests already healed every `biological` unit
  (soldiers and citizens alike); unchanged. A healer heals no enemy and never itself.
- **Attrition relief.** `et_supply.radius` of both carts is 8000 (was 6000). The attrition code
  (`rom-sim/src/attrition.rs`) already let an own supply unit cancel enemy-territory damage
  within that radius; it stays own-only: an ally's cart does not shield (as before, tested).
- **Two constants.** `citizen.auto_build_delay_ticks` (40, two seconds) and
  `citizen.auto_build_radius` (40000 milli-tiles, a city's border radius), both validated by
  `World::new`.
- **Mending is slow because it is free.** `et_builder.repair_pct` of citizens and worker drones
  is 25 (was the default 100, unused): a builder mends a building's full hit points in four of
  its build times (a 150-tick farm of 400 hp: 2 hp every 3 ticks; a city of 1200 hp: 2 hp a
  tick). At 100 a few citizens kept a besieged city whole, and a 20-seed duel ended in a draw.
  The automated colony keeps 100.

Simulation:

- **Mending a building.** A `repair` order on a finished, damaged own building (no longer
  `unsupported`) walks the builder to it and restores, each tick, the building's `max_hp` ×
  `et_builder.repair_pct` / 100 / its build time (`build_time` stat), with the remainder kept in
  the order's `build_remainder`: at 25 a builder mends a building from nothing in four build
  times; several builders add up. It costs nothing. The order is done when the building is whole; a
  repair of a whole building is done at once. A struck-down city keeps its own restore path, and
  units are not mended by builders.
- **Idle builders.** A builder that can move (citizens, worker drones) is now tracked as idle
  work even without a harvester component. Every `citizen.auto_build_delay_ticks` of standing
  idle it looks for the nearest own unfinished site, else the nearest own damaged building (not
  struck down, not being assimilated), on its body, within `citizen.auto_build_radius` of where
  it stands and in the stretch of ground its movement class can cross (`World::reachable`); ties
  go to the lower entity id. It is given a `repair` order to it (joining the site, or mending).
  Only then, after `citizen.auto_work_delay_ticks` as before, does it look for a gather slot. A
  unit with any order (a gather order included) is not idle and keeps it. So a player may place
  several sites and leave them to the citizens around. The game has no idle-worker counter; the
  computer player's `idle_workers` summary counts units without orders, so builders sent to a
  site no longer count.

Interface (`rom-app/src/selection.rs`): a box that holds armed soldiers keeps the units *of the
army*: armed units that are not workers (no builder, harvester or trader component) and the
unarmed movers with a healer or supply component that are not tagged `civilian` (medics, supply
carts, repair drones; priests are civilians). Citizens, worker drones and priests stay out
(citizens are armed, so before this they were boxed with soldiers). Ctrl keeps everything, a
box without soldiers keeps everything, control groups keep any mix. A double click on a soldier
takes the soldiers of its type and the support units on screen. An attack-move sends the
unarmed members of a selection as a move. A right click on a damaged own building with builders
selected gives them `repair` (before a gather order on a damaged gather building).

The computer player's naval manager no longer sends a healer that cannot reach ships on the
water (a supply cart now heals `mechanical`) to mend them.

## Militia: workers called up and stood down (task 141)

Owner (2026-11-23): citizens are called up as militia, with more attack and hit points, and
stood down again; free, in any age, near an own city.

- **PART A, new component table `et_call_up(entity_type_id, militia_type_id)`**: the worker
  (`unit.citizen`, `unit.worker_drone`) and the first step of the militia line it is called up
  into (`unit.militia_ancient`). `lint_unproducible` also accepts a type named by an
  `et_call_up.militia_type_id` (it is made from workers, never trained). The rules loader puts
  the row in `EntityType::call_up`; validation refuses a call-up from a type that is not a moving
  unit, or into a type that is not a moving, armed unit, or that builds, gathers or is called up
  itself; the reachability check counts the militia as made once its worker is.
- **The militia line** (`ui_group` `militia`, generated by `db/content/generate.py`,
  `militia()`): one step per step of the line infantry, in that step's age, each upgrading from
  the one before (`upgrades_from_id`). The advance to a step's age (`tech.age.<age>`) carries
  an `upgrade_type` effect from the previous step, so the militia standing change with the age,
  as the nuclear reactor becomes the fusion plant (task 146). Each step has the infantry
  step's weapon (damage class, reach, reload, projectile) at 60 % of its damage, 60 % of its hit
  points, armour 0 and the `flesh` class (no plate), the infantry's sight, the citizen's speed
  (1300) and population (1), tags `biological`, `military`, `infantry`, `passenger`. No
  builder, harvester, cost, production row or build time.
- **Two order kinds**, `call_up` and `stand_down` (`needs` '', target none). Like `delete`,
  they apply at once at the next tick boundary and are not queued. `call_up` takes a unit
  with an `et_call_up` row; it becomes the newest step of its militia line that its player has
  unlocked and its faction may use (from the first step, the lowest-id unlocked successor while
  there is one). `stand_down` takes a unit of a militia line (`Rules::militia_worker`: a line
  whose first step an `et_call_up` row names) and makes it the newest unlocked step of the line
  of those workers (a citizen, or a worker drone once that upgrade is researched). Either is
  refused (`MilitiaOutsideTerritory`) unless the tile under the unit belongs to its owner's
  territory; a unit in orbit is refused too. The unit keeps its id, place, facing and stance;
  its orders are cleared (it leaves the site it works at, pending orders of other kinds given
  in the same tick still follow); its reloads are cleared; `refresh_entity_stats` keeps its hit
  points' share of the maximum (rounded down). `CannotCallUp` and `CannotStandDown` refuse the
  wrong unit.
- **PART B: nothing new.** The type change is the entity's `entity_type_id`; the save format
  stays 19 and the state hash covers it as any type.

What follows from the data, not from new code: a militia has a weapon and neither builds nor
gathers, so it counts in the army (`World::army_size`), is boxed with soldiers
(`rom-app` `of_the_army`), is not idle work for the auto-gather and auto-build searches, is a
military unit for the computer player, and is left out of `rom fight-matrix` (it has no
production row and no price: a fight at equal cost does not apply to a free unit).


## Formations a player picks (task 143)

Owner (2026-11-23): "big ship formations ... are disposed in a circle which sometimes is not
convenient, lets have more ships and troops formations: square, compact, circle, line, double
line etc."

- **PART A, `formation.shape` CHECK** gains `compact`, `circle` and `double_line`, with three
  rows at the end of the seed: `form.compact` (spacing 70 %), `form.circle` and
  `form.double_line` (130 %). `rank_ratio_pct` of `square`, `sparse` and `mob` is now 100
  (square blocks), of `column` 25 (a block half as wide as deep). `spacing_pct` is 130 for
  every shape but compact (70), sparse (200) and mob (70, no `ranged_behind`): at 750
  milli-tiles the drawn soldiers stand shoulder to shoulder and the ranks do not read.
  `form.line` keeps its ratio 400: the computer players' advance reads it, and now marches
  30 % wider (`docs/computer-player.md`). `FormationShape` gains the three values.
- **New constant `formation.orbit_spacing_pct` (400).** Ships above a planet are drawn two to
  three times their size (`et_view_scale.planet_pct`); in orbit (or entering it) a member's
  spacing is this share of its `formation_spacing`, so the hulls do not overlap and the views do
  not fan them out. In a system's space the plain spacing holds.
- **What a shape lays out** (`rom-sim` `formation.rs`, `formation_ranks`): `line` one rank,
  `double_line` two, `wedge` one at the point then three, five, … a rank; `circle` (and
  `sphere`) one ring of up to twelve whose circumference gives each its spacing, rings a spacing
  apart (outer first) beyond; every other shape a block `ceil(sqrt(n × rank_ratio_pct / 100))`
  wide, `envelop` with its flanks bent forward, `refused` back, an echelon receding to its
  side. The spacing is the largest `et_movement.formation_spacing` of the members times
  `spacing_pct`, at least the widest collision diameter plus 50 milli-tiles. The centre stands
  on the goal; the ranks face from the group's centre to the goal (along the first member's
  facing when it stands there already).
- **Orders.** `move`, `attack_move`, `patrol`, now also `orbit` and `travel`, may name a
  formation as subject (anything else is `UnknownFormation`). When orders are applied, one
  player's orders of one tick naming one formation with one target make a group; with
  `ranged_behind` the members go close fighters first (no weapon reach), then ranged by reach,
  then unarmed, each set farthest-ahead first; each rank takes its members left to right as they
  stand. Each order's point becomes the member's place (`target_pos`), so the save and the
  state hash carry it as before. A place the member cannot stand on (water, a building, off the
  map) gives it the nearest free spot of the formation's lattice within three spacings (behind
  it first), or the goal when there is none. Ships in orbit are placed on the planet below; `orbit` of a habitable planet
  enters its orbit at the places about the order's point (the middle of face 0 by default);
  `orbit` of any other body keeps the marker as target and adds the ship's own place, half the
  holding reach out on the group's side and facing the body, within reach of it (only places
  that are get one): the ship walks there and holds within 250 milli-tiles of it, walking back
  when pushed off. `travel` places the ships about the point in the destination system, facing
  away from the edge they jump in by (an unexplored system still brings them in at its edge).
- **Pace.** A walking formation that `keeps_slowest` arrives together: the member with the
  farthest to go walks at the slowest member's speed, the others slower by how much nearer
  they are, never below a quarter of it (`order.pace`). Orders naming a formation that are not
  laid out together (the computer player's advance gives every member its own point) keep the
  old pace: one player's orders of one tick and formation at the slowest member's speed; the
  kind no longer separates them, so an attack move's unarmed members walk with it.
- **Passing through.** Members of one formation on the march (front orders of one player, tick
  and formation, not fighting) do not push each other apart, so a rank can pass through another
  to its place; avoidance between everyone else is unchanged.
- **PART B: nothing new.** The place is the order's point and the pace its `pace`; the save
  format stays 19.

## City names (task 144)

Owner (2026-11-23): "let the cities have names"; nation lists, renamable.

- **PART A: no new table.** The existing `faction_name` pool with `kind = 'city'` holds the
  lists: 20 names for each of the fourteen human nations (real names of each people's cities,
  in the people's own or period form where it reads well: `Roma`, `Athenai`, `Camulodunum`,
  `Waset`, `Kerma`, `Pasargadae`, `Chang'an`, `Heian-kyo`, `Pataliputra`, `Karakorum`,
  `Tenochtitlan`, `Cusco`, `Ngaruawahia`, …; ASCII only, so the HUD's font has every letter) and
  20 invented names in one sound for each brood and flight (`Kesharak` for Ashscale, `Glumrak`
  for Mirefang, `Ithral` for Sunstone, `Teeliri` for Highwind, `Rhusish` for Reedmarsh,
  `Krevai` for Stormcrest). `seq` 1 is the capital's. Blocs and races have no list.
- **New `order_kind` `rename_city`** (target `none`, no component): an own city (a site too)
  takes the name in the order's text at once. `Command` (and the network's `WireCommand`)
  gains `text`, used by this order only. The name is cleaned: control characters and invisible
  format marks (zero-width, direction overrides) dropped, every run of white space one space,
  ends trimmed, at most **`city.name_max_chars`** characters (new constant, 24). A name that
  cleans to nothing is refused (`EmptyName`), as are another player's city (`ForeignOwner`) and
  anything that is not a city (`NotACity`). Like a stance it is a setting: it is applied when
  submitted, takes an order id, has no cost and never enters a queue; the journal carries the
  command, so every machine renames alike.
- **Naming** (`rom-sim` `city_names.rs`). A city is named when it enters the city index (placed,
  as a site or finished, or spawned at setup): the first name of its owner's list that no city
  in the match carries, so two players of one nation never share a name and a renamed city's
  old name is free again. The list is the player's starting faction's (`origin_faction`), or
  the first faction up its ladder with one, then the present faction's: a bloc and united
  humanity go on with the nation's names. When the list runs out the names come round as
  "New <name>", then "New <name> 2", "New <name> 3", …; a faction without any list numbers its
  cities after the city type ("City 1", …). The capital keeps the first name; a captured city
  keeps its name; a city that is gone frees its own.
- **State.** `Territory::names` (city id → name), in the state hash after the city index, in the
  snapshot (`WorldSnapshot::city_names`) and in the save.
- **PART B, `city.name` (format 20).** The name beside the city's economic radius. Formats 13
  to 19 load with every city named on restore from its owner's list in id order (the capital,
  the lowest id, keeps the first name); `World::restore` names any city the snapshot leaves
  without one.

## Fortifications and minefields (task 147)

Owner (2026-11-23): palisades, then walls from the classical to the enlightenment age, then
defensive lines (trench, tank traps, barbed wire) from the industrial to the fusion age, dragged
as lines with gates; minefields from the industrial age, hidden from enemies, sparing own units.
Part 147a (this section) is the simulation, the data, the models and the sounds; the drag in the
interface, the drawing of joined segments and the computer player's use of them are part 147b.

- **PART A, two component tables.** `et_fortification(entity_type_id, gate, gate_type_id)`: a
  one-tile building laid along a line; `gate` = 1 lets the units of its owner's team through, a
  segment's `gate_type_id` names the gate laid in its line. `et_minefield(entity_type_id, damage,
  damage_class_id)`: a one-tile mine. The rules loader puts them in `EntityType::fortification`
  and `::minefield`; validation refuses either on anything but a building of one tile, both on one
  type, a segment whose gate is not a gate, a gate naming a gate, and an unknown damage class.
  New stat `mine_detection` (entity) and constant `mine.detection_radius` (4000), checked by
  `World::new` when the rules have a minefield; new tag `fortification`. Both lint views stay
  empty.
- **The content** (`db/content/generate.py`, `fortifications()`, written after every other
  definition so every earlier id stays). One type per kind, its look following its owner's age
  (`entity_visual`), page `fortification` of the builders' palette:

  | Kind | Segment | Gate | Ages | Hit points | Armour | Build ticks | Price |
  | --- | --- | --- | --- | --- | --- | --- | --- |
  | palisade | `bld.palisade` | `bld.palisade_gate` | ancient (with Military 1) | 500 | 2 | 60 | 10 timber |
  | stone wall | `bld.stone_wall` | `bld.wall_gate` | classical to enlightenment | 1500, +300 an age | 4, +1 an age | 100, +10 an age | 12 timber, 6 metal |
  | defensive line | `bld.defensive_line` | `bld.checkpoint` | industrial to fusion | 2600, +300 an age | 8, +1 an age | 90, +10 an age | 8 timber, 14 metal |

  A gate has 150 % of its segment's hit points, three times its price and twice its build time.
  All are `structure` armour (siege and artillery, `explosive` at 200 %, break them; arrows do
  next to nothing), 3 tiles of sight, no ramp, no limit, `in_own_territory` on plain ground. The
  later ages of a kind are `modifier` rows of the age advances (`hp`, `armor`, `build_time`, `add`,
  on the exact type). The advance into a kind's first age carries an `upgrade_type` from the
  kind before (palisade → stone wall at the classical advance, stone wall → defensive line at
  the industrial one; gates alike), so the standing ones change in place keeping their health's
  share, as the reactor becomes the fusion plant, and the old kind is superseded in the build
  menu. Each kind's types are one upgrade line (`upgrades_from_id`).
- **The minefield** `bld.minefield` (industrial): 100 hit points, armour 0, 40 build ticks,
  5 timber and 15 metal a mine, no sight. Damage 140 `explosive` (80 × f(5), the formula of
  `content.md`), +12 with each later advance to the fusion age (`attack` modifiers). **Mine
  Detectors** (`tech.mine_detectors`, industrial, at the barracks, 100 food and 150 metal, 400
  ticks; its button takes the next free place so no other moves) gives the `infantry` tag
  `mine_detection` +1: infantry, ranged foot, anti-armour teams and scouts then see enemy mines
  within `mine.detection_radius`.
- **Laying a line: the order is `construct`, one per site.** `Command` cannot carry a list of
  points (its shape is shared with the interface and the computer player), so a line is no new
  order kind: `World::line_sites(player, segment, points, gates)` takes the polyline a drag gives
  and returns its sites (`LineSite`: subject, tile centre, facing along the line, and the result
  of the same placement check `construct` makes): the tiles each straight stretch crosses,
  eight-connected, each once, in drag order, the site nearest each point of `gates` taking the
  segment's gate type. The interface orders one `construct` for each site it keeps (the first
  replacing, the rest queued on the builder); each site is laid when its order is applied, paid
  at submission like any building, and other builders join through the idle-builder search of
  task 140. `World::minefield_sites(player, mine, a, b)` does the same for the rectangle of tiles
  between two corners, one mine a tile, row by row. Both refuse a type that is not a segment or a
  mine (`NotALine`), more than `MAX_LINE_SITES` (256) sites (`LineTooLong`), a stretch across
  bodies (`OtherBody`) or into a cube corner's gap (`CrossesFaceEdge`).
- **Blocking and gates.** A segment's tile counts in the path occupancy like any building's, for
  its own side too. Eight-connected diagonal segments leave no gap: a path step never cuts a
  corner between two blocked tiles, and a straight walk through an exact corner touches both.
  A gate's tile is also in `Pathing::gates` (tile → its owner's team): the search, the straight
  walk, the waypoint smoothing and the pushes of a unit whose owner is of that team pass it;
  everyone else finds it shut. Path requests carry the team (`PathRequest::side`) and are shared
  only within it. A captured gate (`change_owner`) opens to its new team. The gate map is derived
  from the entities (rebuilt on restore, not hashed); a restored queued search takes the team of
  its first waiter.
- **Mines.** A mine occupies no tile (units walk over it; it is not pushed out of the way of a
  site), but stands in the building index, so nothing is built on it (an enemy placing a building
  on a hidden mine is refused too, as on any occupied tile). It is hidden from every
  player outside its owner's team (`World::visible_entities`, attack orders (`TargetHidden`),
  automatic targets, building memory and planet surveys) unless a unit of that player's team
  with `mine_detection` > 0 stands within `mine.detection_radius`; an ascended player sees every
  mine. Each tick, after movement, a finished mine with an enemy ground unit on its tile (a
  moving unit of another team, not in the air, not in orbit, not aboard; not the neutral player's)
  goes off: the unit of lowest id there takes `attack` (from `et_minefield.damage`) × the damage
  matrix − its armour (at least `combat.min_damage`), the mine leaves a blast for the views
  (`World::blasts`) and is removed (no wreck, no loss counted). Mines in id order. Splash and
  detected mines can be attacked like any building.
- **Not a town's building.** Segments, gates and mines do not widen a city's economic radius
  (`city.radius_per_building`).
- **PART B: nothing new in 147a.** Walls, gates and mines are ordinary buildings; the state
  hash covers them as any entity. Part 147b adds the facings below (format 21).
- **Part 147b (owner, 2026-11-25: "i want the gates wider ... 3 times wide ... problem will be
  with the orientation").** A building keeps a **facing** (`Entity::facing`, degrees clockwise
  from the face's x axis, a multiple of 45): `construct` carries it (`Command::facing`, snapped;
  `Order::facing`, hashed when not 0), the site and the building keep it, and so do the fog's
  memory of a building (`RememberedBuilding::facing`) and a wreck. The footprint turns with it
  (`rom_sim::turned_footprint`): a quarter turn swaps the sides, a diagonal covers the square of
  the longer side. Every use of a building's footprint takes its facing (placement and overlap,
  path occupancy and gates, approach, reach). The **gates are 3 × 1** (`GATE_WIDTH` in the
  generator; validation allows a gate of an odd width, one tile deep, segments and mines stay
  one tile): `World::line_sites` centres a gate on the line's tile nearest its point where the
  line runs straight for the gate's width (else on the nearest tile), faces it along the line
  and leaves out the segments its footprint covers; on a diagonal a gate covers 3 × 3 tiles.
  `World::check_placement_facing` and `place_turned_for_setup` are the turned forms of the
  preview and setup calls. **Save format 21**: `"order".facing` and `vision_memory.facing`;
  formats 13 to 20 load with both 0. The render turns a building's model by its facing and
  stretches a diagonal one by √2 along its x, so a diagonal line's segments meet.
  Measured: `rom playthrough --seed 1` hash `abee291cb764153a`, ascension `1ee55ed65a7c141b`
  (before: `acc4db242db8a622`, `91692088c29e53a3`); with the gates set back to one tile in the
  same build both come back exactly, so the move is the playthrough bot's gate standing three
  tiles wide. Rules fingerprint `f6ba40204370fc84` (gate footprints, gate models, the order
  table). The gate models keep the art direction's team colour share: a part widened across
  the gate is structure, with the team colour on its middle tile only.
- **The computer player (147b)**: see `computer-player.md`, "Fortifications and minefields".
- **The computer player (147a)** read none of this. Its construction manager builds the first of a
  building line it has none of at a low score when nothing better waits, so it may lay one
  segment, one gate and one mine in a match; no data column keeps a constructible building out
  of its catalogue without also keeping it from people. Part 147b teaches it lines and mines.
  Measured (20 seeds, balanced:moderate against balanced:easiest): 18–0 with 2 draws (seeds 5
  and 14), mean tick 52052; with the same rules less the construct rows of these seven types
  20–0, mean tick 52405, exactly as before the task, so the simulation changes alone move no
  duel: the two draws come from the computer players laying their one palisade and gate.
- **Looks** (`db/models/`): a model per kind and age, segments spanning their tile along x so
  neighbours join end to end (the interface turns them along the line): `bld.palisade@ancient`;
  `bld.stone_wall@classical`, `@medieval`, `@gunpowder`, `@enlightenment`;
  `bld.defensive_line@industrial` (earth parapet, sandbags, barbed wire), `@modern` (wire coils,
  concrete tank traps), `@information` (precast barrier, fence), `@automation` (composite panels,
  sensor post), `@fusion` (pylon and glowing field); the gates of each age; `bld.minefield@` the
  five ages from industrial. A palisade or wall still standing in a later age (one granted that
  age without its advance) wears the look of the kind built then, so every building has a look
  for each era (`bld.palisade@medieval` is the curtain wall's, `@industrial` the trench line's,
  and so on: 41 files in all). **Sounds**: the `post` family (`tools/sounds/assign.py`: walls, gates
  and mines are manned like a guard post), baked for the new types only.

## The space ages split by distance (task 155)

Owner (2026-11-25): "spaceports should also build space probes earlyer so that we can explore
and colonize the solar system, frigates should also be maybe interesting to have earlier than
starfaring age ... which honestly comes too soon. i'd rename it spacefaring age and the
expansion age i'd make into starfaring age." The own star system is reached first, the other
stars one age later. Rows only: no schema change and no code; both lint views stay empty.

- **Names.** `age.starfaring` is shown as the **Spacefaring Age** (10) and `age.expansion` as the
  **Starfaring Age** (11); their advances are "Advance to Spacefaring" and "Advance to
  Starfaring" (`AGE_NAMES` in the generator). With them (owner, GitHub issue #160: "Please
  rename the gunpowder age to the age of discovery") `age.gunpowder` is shown as the **Age of
  Discovery** (3), its advance "Advance to Discovery", its dialog subtitle "powder, sail and new
  coasts" (it was "the first age of discovery"). The keys stay, so saves, the server, scripts and
  the music keys are unchanged; the music's files and published titles stay too (`music.md`).
  The age dialog's texts (`assets/text/ages.toml`, `ages-story.md`) follow the new names.
- **Fusion (9): the home system.** The fusion age has been map-scale rank 1 since task 138, so
  its ships may launch and move within the home system (`World::space_age`) and nothing in the
  code changes. The spaceport (information age since task 130) now also trains the **space
  frigate** and the **transport ship** in the fusion age, and citizens may build the **ground
  battery**: the frigate bombards from orbit, and the battery is its answer on the ground (task
  71's rule), as the satellite (armed since task 134) is in orbit. Their `prerequisite` rows
  name `age.fusion`; their `first_age_id`, stats and looks stay those of the spacefaring age
  (`BUILDABLE_FROM` in the generator, which since this task holds units too, as the spaceport
  and the satellite did for task 130). A fusion transport carries settlers to another planet
  of the home system and they found a city there (task 54); a fusion frigate claims a resource
  body by presence like any starship with the claim tag; the computer player does both by data
  (`computer-player.md`, "Space").
- **The space probe** stays bound to its planet's orbit (`orbit_bound`, task 130). The other
  planets and bodies of the home system are explored by the frigate and the transport; a probe
  that leaves its orbit would need a code rule or a new unit type with its own model.
- **Spacefaring (10)**: corvettes, carriers, orbital craft, the space freighter, the Deep Space
  Survey, as before.
- **Starfaring (11): jumps.** A `travel` order (a jump along a route) is refused below map-scale
  rank 2 (`SimError::CannotEnterSpace`), and rank 2 is first seeded at `age.expansion`. So the
  research that opens the jumps is the age's own advance, **Advance to Starfaring**
  (`tech.age.expansion`, library, page `research_4`, column 0 row 1): 2 750 food and 2 750
  knowledge, 2 600 ticks, after the four track researches of level 11. A separate research
  would need a new rule key read by the travel check in `rom-sim`; it was not added. The
  **survey ship**, a picket for jump arrivals, moves to this age (its prerequisite; stats,
  `first_age_id` and look stay), and with it its **Survey Sensor Array** (spaceport research,
  now 850 ticks, 1 200 wealth and 600 knowledge; was 800, 1 100 and 550 in the spacefaring age).
  Enhanced Sensors and the jump inhibitor were already of this age, interstellar trade is of
  abundance; the jump detector (task 118) stays in the fusion age (it warns of foreign ships).
  The spaceport's research grid follows the order of the ages: Deep Space Survey moves to
  column 1, Enhanced Sensors to 2, the Survey Sensor Array to 3.
- **Balance.** The advance into the Spacefaring Age costs half as much again
  (`AGE_ADVANCE_PCT`): 3 750 food and 3 750 knowledge (was 2 500 each) and 3 600 research ticks
  (was 2 400). No other price or time changes.
- **Tests** (`rom-sim/tests/space_ages.rs`, each run twice for equal hashes): a fusion-age
  player builds a spaceport with citizens, trains a probe, a frigate and a transport (the
  corvette and the survey ship are refused), may not jump, carries three citizens to the
  neighbour planet of its system and founds a city there; a spacefaring frigate flies within its
  system but may not jump until the library's Advance to Starfaring is done, then jumps.
  Two tests followed the data: the rally-point test of task 142 trains a corvette where it
  trained a survey ship, and the descent's food in the ascension test is 53 415 (was 52 290: 90 %
  of the 1 250 more food the advance costs).
- **Measured.** `rom playthrough --seed 1` hash `3a9c0ad61891fb60`, ascension `6cd649b9b902f276`
  (before: `abee291cb764153a`, `1ee55ed65a7c141b`). The ages up to fusion are reached on the same
  ticks (fusion 33 416); spacefaring 38 904 (was 38 184), starfaring 43 720 (43 000), abundance
  48 536 (47 816), galactic 53 304 (52 584): the bot spends 720 more ticks on the dearer advance
  (its science research shortens the 1 200 extra ticks). With the advance's old price and time
  in the same build every age tick comes back exactly; the hash still moves (`bd16b5ba7bd25279`)
  with the other rows. Fight matrix 270 strengths, 0 unanswered (its rosters follow
  `first_age_id`, which did not move); duel balanced:moderate against balanced:easiest 20–0, mean
  tick 46 979 (unchanged: those matches end long before the space ages). Screenshots (universe seed 1,
  `--players 2 --ai none --sun fixed`, scripts `tasks/155-*-script.txt`):
  `img/155-spacefaring-dialog.png` (the library's advance from the fusion age: the new-age dialog
  names the Spacefaring Age), `img/155-fusion-spaceport.png` (a fusion-age spaceport has trained
  a probe, a transport and a frigate through its Train tab) and `img/155-fusion-system.png` (the
  same ships in the system view: the frigate and the transport sent to the home planet's moon,
  which they claim, the probe still in its planet's orbit).

## City and fort shields (task 157)

Owner (chat, 2026-11-25): "cities and forts (their upgrades in their line) should have their own
shield gained through research, in the freedom age they can research missile shield, the upgrade
to laser shield and energy shield", "make the shields expensive to research"; with task 158: a
city's shield covers its city area. **Schema change (authorised by the card):** one component
table, four stats, one save table; both lint views stay empty and the seed loads.

- **`et_shield(entity_type_id, cover, recharge_delay_ticks, recharge_period_ticks, recharge_pct,
  recharge_resource_id, recharge_cost)`**: a building its owner's researches may give a shield.
  The rules loader puts it in `EntityType::shield`; validation refuses it on anything but a
  building, a period below one tick and a recharge resource that is not a resource. The
  generator (`shields()`, written after every other definition so every earlier id stays)
  gives it to every type tagged `city` or `fort` (`SHIELD_COVER`): the city, the automated colony
  (a city, tag `city`) and the whole fort line (fort, star fort, fortress, citadel, found by the
  tag their upgrades carry). The tower line (bastion, bunker, defence turret) is not the fort
  line and holds none.
- **What it covers**: what stands within `cover` milli-tiles of its footprint (forts: **4 000**,
  four tiles round them) and, for a city type, its whole economic radius (the city area, which
  grows with its buildings whatever task 158's option says). A target is measured by its
  position (a building by its centre); in orbit or aboard, nothing is covered.
- **Stats (entity scope)**: `intercept.explosive`, `intercept.bullet`, `intercept.kinetic` — the
  percent of the projectiles of that damage class thrown at whatever of the shield's owner's it
  covers that the shield shoots down; the simulation reads `intercept.<damage class key>`, so a
  new class is shot down by adding its stat and modifiers, never by code. `shield_pool` — the
  percent of the building's (effective) hit points its pool holds.
- **The researches** (kind `upgrade`, at the city: research page, columns 2 to 4, the next free
  places; each needs the one before). Their modifiers target the tags `city` and `fort` and add
  up:

  | Research | Age | Price | Ticks | Adds | The advance into its age |
  | --- | --- | --- | --- | --- | --- |
  | Missile Shield `tech.missile_shield` | Freedom (`age.information`) | 2 000 metal, 2 000 knowledge | 2 000 | `intercept.explosive` 35 | 1 750 food, 1 750 knowledge, 1 800 ticks |
  | Laser Shield `tech.laser_shield` | Automation | 2 500 metal, 2 500 knowledge | 2 400 | `intercept.explosive` +20 (55), `intercept.bullet` 35, `intercept.kinetic` 30 | 2 000, 2 000, 2 000 |
  | Energy Shield `tech.energy_shield` | Fusion | 3 000 metal, 3 000 knowledge, 800 energy | 2 800 | `shield_pool` 50 | 2 250, 2 250, 2 200 |

  Each is dearer and longer than the advance into its age, and the first two also than the
  advance out of it (Automation 2 000/2 000/2 000 ticks, Fusion 2 250/2 250/2 200).
- **Interception** (`rom-sim/src/shields.rs`, `World::intercepted`, called in `fire_shots` for
  each shot that struck true, in the shots' geometric order): a hit of a weapon that throws a
  projectile (`projectile_speed` > 0, a reach, not consumed: the rule `attack_ground` already
  uses) at an entity of a player (or, a shot at the ground, at a point) that a working shield
  of that player (of any enemy of the shooter, for the ground) covers is rolled against the largest
  `intercept.<class>` of the covering shields (the nearest of those shields, by the distance to
  its footprint, shoots) with the simulation's one generator; on a success it is
  not queued. Light and rail slugs (beams, railguns: `projectile_speed` 0) hit at once and cannot
  be shot down, so the space frigate's and the space cruiser's bombardment from orbit pass every
  shield; the orbital bomber's shells and the battleship's plasma are shot down. Shots at a
  drone of a grid are not. No roll is made where no covering shield has a share, so worlds
  without the researches draw the same numbers as before. A working shield is a finished,
  standing building of a shield type, not neutralised, on a surface.
- **The pool**: `World::absorb`, called in `damage_entity` (every hit and splash): the charged
  pools of the target's owner's shields that cover it, the nearest first (by the distance to the
  shield's footprint, then by id: a fort standing in a city area takes the blows on itself
  before the city's dome does), take the damage before hit points; the blow still counts as an attack (alerts, retaliation). Every covering pool, empty or
  not, restarts its recharge delay. `recharge_shields` (each tick after the dead are removed):
  each shield, in id order, at a multiple of its `recharge_period_ticks` (**20**, a second), with
  a pool now (`shield_pool` × its hit points), not hit for `recharge_delay_ticks` (**200**, ten
  seconds), gains `recharge_pct` (**5**, empty to full in 20 s) of its full pool, paying
  `recharge_cost` (**10**) of `recharge_resource_id` (**energy**) per 100 points, both rounded up;
  without that much in store it does not recharge. A pool starts empty and charges the same way.
- **State**: `World::shield.pools` (shield → pool points and the tick of its last blow), hashed
  once not empty (other worlds keep their hashes) and saved: **save format 23**, table
  `entity_shield(entity_id, pool, hit_tick)`; formats 13 to 22 load with no pool (one guard, no
  other compatibility code). `World::interceptions()` and `World::shield_hits()` are reports for
  the views, kept 60 ticks (after the would-be landing), not hashed, not saved;
  `World::shield_view(id)` gives the pool, its full size and the reach a dome is drawn to.
- **Views** (`rom-app`, `rom-render`): a shot shot down is drawn flying its whole way's time
  (`ShotView::cut`) and stopped three quarters along (`INTERCEPT_AT`) with a white-blue burst and
  sparks; an Energy Shield's dome (`HudShield`, `overlay/shields.rs`) spans what it covers, a
  translucent cap 0.4 of its reach high (at most 9 tiles, but 1.2 times the building under it
  and never above its reach), as bright as its pool is full, shimmering, drawn without writing
  depth (`OverlayGeometry::glass`) so what is inside shows; a blow on it flares on the side facing
  where it came from (`HudShieldHit`: `ShieldHit::from`, the attacker's position, from the struck
  point 0.6 of the way to the rim toward it, up on the dome). Sounds: `docs/sound.md`.
- **Computer players**: nothing new; they weigh the researches by their usual scores (`rom-ai`,
  `research::worth`: modifiers of an entity stat by the army share, less a twentieth of the
  price), so the dear shields come late.
- **Screenshots** (the game's own paths, `--script`): `img/157-rockets.png`
  (`tasks/157-rockets-script.txt`: rocket batteries fire on a city with the Missile and Laser
  Shields; shot-down rockets burst three quarters of the way) and `img/157-dome.png`
  (`tasks/157-dome-script.txt`: fusion tanks attack a citadel standing in its city's area; its
  Energy Shield dome flares on the side facing them).
