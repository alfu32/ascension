# Game database

Files: [`../db/schema.sql`](../db/schema.sql) (90 tables, 4 views),
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
| `body_kind` | rules | the vocabulary `star`, `habitable`, `resource` |
| `map_style` (universe knobs) | rules | neutral systems, habitable planets and resource bodies per system (each a `_min`/`_max` pair), planet face size `planet_n_min/median/max` (64..=512), `route_extra_pct`, `star_map_radius` |
| `map_style_feature` with `region_kind = 'resource_body'` | rules | what resource bodies yield: a `node` type with `claim_mode = 'presence'`; `chance_pct` weights the kinds, and a `per_player` row puts `clumps_min` bodies of its kind in every home system |
| `star_system` | state | name and star-map position (integers, display only) |
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
in slot 0. The hand-written rows follow the generated seed block.

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
mountain tiles within 6 tiles of it for its slots). Missing tiles are grown
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
means 100 and 100. The seed gives the starships two to five times their size
above a planet and a quarter to a half of the reference on the plane, by
class (survey ship smallest, battleship largest); the rows are appended after
the generated content, keyed by definition key, so no id moves. The renderer
takes them through `rom-app`'s `looks::ship_scales` and never looks at a key.
The same hull is drawn at every level; between the planet and the plane its
size and place blend smoothly with the camera's distance.

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
| `unit.orbital_bomber` | airfield, starfaring | 520 HP, armour 4 vehicle, speed 4000, climb 120 ticks; slot 0: 30 explosive, 4 tiles, 40 ticks, at `orbit`, `space`; slot 1 (`orbit_only`): 90 explosive, 8 tiles, 60 ticks, at `ground`; 260 metal 100 energy, 450 ticks |
| `unit.carrier` | spaceport, starfaring | 2000 HP, armour 6 shield, speed 2400 (starship), `pop_cost` 2; 25 energy, 6 tiles, at `space`, `orbit`; container 10 population, accepts `orbital_craft`, `launches`; 700 metal 300 energy, 500 ticks |
| `bld.ground_battery` | built by citizens and worker drones, starfaring, military page, own territory, plain | 2400 HP, armour 5 structure, 3 × 3, sight 16 tiles; 150 explosive, 14 tiles to the subpoint, every 30 ticks, at `orbit`; 400 metal 200 energy |

`et_view_scale` rows: carrier 450 / 45, fighter 300 / 25, bomber 320 / 25. A
ground battery contests a landing: a carrier descending onto ground within its
range is in orbit until it touches down, so it is shot at all the way down.

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
