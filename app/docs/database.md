# Game database

Files: [`../db/schema.sql`](../db/schema.sql) (124 tables, 4 views),
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
     │                 ├─ et_healer        heals            (priest, medic, repair drone)
     │                 ├─ et_trader        runs city-to-city delivery routes
     │                 ├─ et_keeper        keeps carried nodes
     │                 ├─ et_container     garrison / transport / carrier
     │                 ├─ et_producer      takes train + research orders
     │                 ├─ et_territory     city, borders
     │                 ├─ et_energy        ability pool
     │                 ├─ et_supply        supply radius
     │                 ├─ et_gather_site   farm, mine, camp, university
     │                 ├─ et_view_scale    drawn size in the space views (presentation)
     │                 ├─ et_inhibitor     jump inhibitor: holds enemy ships in its system
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
| `map_style` (universe knobs) | rules | `star_region` (which catalogue stars a match takes, task 89: `nearest`, `single`, `quadrant`, `all`), neutral systems, habitable planets and resource bodies per system (each a `_min`/`_max` pair), planet face size `planet_n_min/median/max` (64..=512), `route_extra_pct`, `star_map_radius` |
| `map_style_feature` with `region_kind = 'resource_body'` | rules | what resource bodies yield: a `node` type with `claim_mode = 'presence'`; `chance_pct` weights the kinds, and a `per_player` row puts `clumps_min` bodies of its kind in every home system |
| `star_system` | state | the star's name, 3D star-map position (x, y, z integers), catalogue star and constellation |
| `star_figure` | state | constellation stick-figure lines between this match's systems (display only) |
| `constellation`, `star`, `constellation_line` | rules | the real star catalogue: 745 stars in 86 constellations with published position, computed map coordinates and galactic longitude, and stick figures ("Star catalogue") |
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
Information Age, 450,000 in Automation, and zero from Fusion on. These are
the ground limits passed to the camera; task 41 supersedes the earlier
separate whole-globe-only age.

**Map styles (task 89).** `map_style.star_region` says which catalogue stars a
universe takes; everything else about a system (habitable planets, resource
bodies, routes) stays in the per-system knobs. The code reads the region kind,
never a style key.

| Style (key) | `star_region` | Systems |
| --- | --- | --- |
| One World (`map.one_world`) | `nearest` | one home system per race, nothing else: a quick match |
| Cluster (`map.many_worlds`) | `nearest` | a seeded star and its nearest neighbours: the homes plus `neutral_systems_min..max` (2 to 4), within `star_map_radius` (the key stays: saves and scripts name it) |
| Solar (`map.solar`) | `single` | one seeded catalogue star whose system holds every home planet (with separate worlds, each race its own planet), `habitable_per_system_min..max` − 1 more planets, and the resource bodies; no routes |
| Quadrant (`map.quadrant`) | `quadrant` | every catalogue star in one galactic quadrant (`star.glon_mdeg` / 90 000; the seed chooses it and keeps it through the layout's retries) |
| Galaxy (`map.galaxy`) | `all` | every catalogue star: 745 systems. Held back by task 89 (the match world grew by about 12 MB a system and was killed at 8.4 GB) and put back by task 99, appended last as before: system space now costs little until it is used (`world-model.md` "Scale") |

`neutral_systems_*` and `star_map_radius` bound the `nearest` region only (the
other rows carry 0 and the catalogue's extent, 8 300). In Quadrant (and Galaxy) the
systems beyond the homes are mostly empty: 0 to 2 habitable planets and 2 to 6
resource bodies each. `habitable_per_system_min` may therefore be 0; a home
system always holds its home planet. In Solar every home planet has the
`per_player` resource bodies as its own moons (one ore asteroid each in the seed).
In the `quadrant` and `all` regions a home candidate's density is measured over its eight
nearest neighbours (three in the nearest region) and up to 256 layouts are tried.

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
size as the ship comes down to land), and `system_pct`, its size on a star
system's plane as a percentage of the readable reference size there (the
size task 53 drew every ship at, which keeps its size on screen). No row
means 100 and 100. Since task 72 the seed gives the starships two to three
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
- Galactic Empires (13) is the last age, but the match has no end: a player that has
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
aims it instead. The early weapons of the city, tower and fort shoot arrows: they have
`et_weapon.projectile_speed` 25 tiles per second (the column existed), so hits land
after their flight and the arrow is drawn in flight; the gun-age variants hit at once
and draw a tracer. (The city's weapon is `pierce` in every age in the data, so a city
shoots arrows in every age.)

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
- **Satellite** (`unit.satellite`, spaceport, starfaring): launched into orbit above
  its spaceport, no weapon, 150 HP, sight 96 tiles. The new tag `orbit_bound` keeps it
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

**Charging a jump.** A ship ordered along a route flies to the edge of its system toward the
next system and charges there for its stat `jump_charge` (new entity stat): the new nullable
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

**Jump inhibitor** (`bld.jump_inhibitor`, starfaring, 3 × 3, 3 000 hp, 500 metal and 300 energy,
built by citizens and worker drones in own territory on plain ground; no weapon). New component
table `et_inhibitor(entity_type_id, reach, charge_pct)`: `reach = 'system'` (the only value)
covers the whole star system of the planet it stands on; `charge_pct` 50. While a complete,
active one stands, ships of its owner's enemies in that system may depart only back along the
route they arrived by (the new PART B table `entity_arrival` records it; a ship built there may
not leave at all) and charge `charge_pct` longer. Several do not stack (the largest applies).
Own and allied ships are free. A refused route is `SimError::HeldByInhibitor` at submission; a
queued or running travel order ends `failed: inhibited` (a freighter's trade order likewise).
Destroying it lifts the effect on the next tick; it has hit points like any building, so
bombardment from orbit or troops on the ground answer it (task 71's rule: it has no weapon, so
it is no strength that needs a counter).

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
  and 10 000 wealth) at the library's projects page, age Galactic Empires. `order_kind`
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
  New definitions (Ships of the Line, Dreadnoughts, their upgrade researches, the two building
  upgrades, the refits Fire Control and Armour Belts) are written after every other generated
  definition (the generator's `late` steps).
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
- **Flight time.** A weapon with `projectile_speed` (the city's, tower's and fort's arrows) is
  drawn landing exactly when the simulation lands it. Every other weapon hits at once in the
  simulation; its projectile is drawn flying at a speed per kind after the shot, so its impact
  shows a little after the damage (a bullet within a tick, an arrow about 0.4 s over five
  tiles, a shell about half a second). Giving those weapons a `projectile_speed` would make the
  flight real, but changes the simulation and its hash: a balance decision for the owner.
- `torpedo` is in the vocabulary and drawn, but no weapon uses it yet (there is no submarine).

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
values from primary catalogues, never invented:

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
  Epsilon Eridani (IAU name Ran). Toliman, the B component, shares
  Alpha Centauri's place and has no row.
- **Map coordinates.** Computed once in the generator, in `position()`:
  `r = 100 * sqrt(distance in light-years)`, then
  `x = r cos(dec) cos(ra)`, `y = r cos(dec) sin(ra)`, `z = r sin(dec)`,
  rounded to integers. The map generator and the simulation read the integers
  only.
- **Galactic longitude** (task 89). Computed once in the generator, in
  `galactic_longitude()`, from the stored right ascension and declination in the
  IAU 1958 galactic frame (J2000: north galactic pole at RA 192.859, Dec +27.128;
  galactic longitude of the north celestial pole 122.932):
  `l = 122.932 - atan2(cos(dec) sin(ra - 192.859), sin(dec) cos(27.128) -
  cos(dec) sin(27.128) cos(ra - 192.859))`, in `glon_mdeg` (milli-degrees,
  0..359 999). It numbers the galactic quadrants as astronomers do: I 0-90,
  II 90-180, III 180-270, IV 270-360 degrees (173, 163, 184 and 225 stars). Hand
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
- **In a match.** `rom-mapgen` picks a connected region; `star_system` holds
  its stars' names, positions (`x`, `y`, `z`), `star_id` and `constellation`;
  `star_figure` holds the figure lines between them. Save format 6 stores the
  two tables; format 5 saves are refused.
