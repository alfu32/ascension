# World model: round planets, star systems, continuous zoom

Decided by the owner on 2026-11-13. Supersedes the "one flat map with planets
set in open space" model (`database.md` section 6, `research/03-timeline.md`
"Reaching space").

## What the player experiences

- Every planet is a **sphere**. A planetary-tier player's camera stays around
  the home planet, limited by `age.view_range`; zero allows the whole globe.
- Interplanetary-tier players can zoom continuously from their planet to the
  home star system. Interstellar-tier players can also reach the star map,
  visit known systems and zoom in on their planets. Each player unlocks these
  tiers separately through `age.map_scale`, using the tier's numeric rank.
- Ground, sea and air play happen on planet surfaces. Space play happens in
  star-system space and on the routes between systems.

## Scale

Reference: 800–4,000 large cities fit around the Earth. In game terms a
planet's circumference is **about 100–200 buildings, median 150**, with
roughly 50 % land.

| Quantity | Median | Range |
| --- | --- | --- |
| Building pitch (footprint + spacing) | 5 tiles | — |
| Circumference | 768 tiles (≈ 154 buildings) | 512–1,024 tiles |
| Cube face size *N* (circumference = 4 *N*) | 192 tiles | 128–256 |
| Tiles per planet (6 *N*²) | 221,184 | 98,304–393,216 |
| Land tiles at 50 % | ≈ 110,000 | — |
| Planet radius | ≈ 122 tiles | 81–163 |

Home planets use the median size for fairness; other habitable planets vary.
A 4-tile building on a 122-tile-radius globe makes the curvature visible from
a modest zoom — that is intended.

## Planet surface: a cube sphere

The simulation keeps working on **integer tiles**. A planet surface is a
*cube surface*: six square grids of *N* × *N* tiles joined along their edges.

- **Simulation:** positions are `(face, x, y)` in milli-tiles. Distance,
  neighbours, movement and range checks use the tile metric; across a face
  edge the neighbouring face is unfolded into the current face's plane. At
  the eight cube corners three faces meet instead of four, a small local
  anomaly that gameplay can ignore. All integer and deterministic, as now.
- **Generation:** each tile's centre is mapped to a point on the unit sphere
  (equal-angle cube mapping, computed with deterministic integer
  arithmetic); terrain comes from 3D noise sampled at that point, so there
  are no seams. Sea level by percentile (default 50 %).
- **Rendering:** the same mapping places the tile grid on a sphere of radius
  *R*; each face is split into chunks; sea is a sphere at sea level. Tiles
  near cube corners appear slightly smaller on screen than tiles at face
  centres (about 1.3 ×); gameplay does not see this.

The planet's axis is the `rom-geo` sphere y axis: north is +y, south is
−y. Latitude is zero at the equator; the prime meridian passes through the
centre of face 0 (+z), with positive longitude toward +x (face 1).
Each polar cap covers the poleward 20 % of **latitude** in its hemisphere,
so its default boundary is |latitude| ≥ 72°, perturbed by seamless,
low-frequency 3D integer noise within ±3°. Ice replaces both land and sea,
is not buildable, and does not block sight. Ground movement classes that
cross plain cross ice at 50 % speed; ships cannot cross it.

`map_style.polar_cap_pct` controls that latitude share (0–50; zero disables
caps); `polar_terrain_id` selects the cap terrain by reference. Caps are
applied after sea level and before biomes and starts. Biome coverage counts
only non-ice land, and biomes, resource nodes and start clearings never
replace ice. Player starts must lie within ±60° of the equator by default,
controlled by `start_latitude_max_mdeg` (0–90,000). Existing start spacing
and local fairness rules still apply. All terrain and deposits are generated
before play.

A flat rectangular map remains available as a topology for tests, small
skirmish maps and the existing scenarios.

## The universe

| Level | Contents | Coordinates |
| --- | --- | --- |
| Star map | star systems, routes between them | system positions (display only) |
| Star system | a star, habitable planets, resource bodies (moons, asteroids, small planets) | continuous integer plane per system |
| Orbit | ships and stations near a body | body + slot |
| Surface | tiles of one habitable planet | `(face, x, y)` |

- **Habitable planets** have a full surface: cities, borders, ground and naval
  battles.
- **Resource bodies** have no surface play. They stay small objects that a
  player owns while a starship stays beside them (the earlier decision).
- Each **race's home planet** is in its own star system. Players of the same
  race share that planet.
- An entity has exactly one **location**: on a surface, in orbit around a
  body, in a system's space, or in transit between systems. Moving between
  levels is an order (launch from a spaceport, land at a site, leave orbit,
  depart along a route), with a duration.

## Camera and visibility by age

| Tier rank | Camera and reach | First seeded age |
| --- | --- | --- |
| 0, planetary | home planet, positive `view_range` limits ground half-width; zero shows the globe | ancient; fusion first has zero range |
| 1, interplanetary | home system, ships may move and orbit within that system | starfaring |
| 2, interstellar | star map and route travel between systems | expansion |

Research updates the existing camera's limit immediately without moving it.
Starting zoom-out and mouse-wheel zoom obey the same limits. The code reads
ranks and range, never age or tier names.

### Launch, orbit and ground fire (task 43)

Training a space-only mover on a habitable planet queues a `launch` order
against its producing spaceport. After `space.launch_ticks`, it enters that
planet's orbit. An orbit has a body, current integer surface subpoint and
facing. Its footprint advances across cube-face edges at
`space.orbit_speed` milli-tiles per second. The system-plane position stays
at the body's marker for system interactions; the orbit record determines
its actual location, ground visibility and combat footprint. Moving in
system space or travelling leaves orbit; an orbit order to another habitable
body enters its orbit when the ship arrives. Resource-body orbit orders
continue holding the ship near the resource marker.

The planet view places these ships eight tiles above their subpoint, with
raised hulls, selection rings and hit boxes. The ship's ground sight follows
that subpoint. `et_weapon.orbit_only` weapons can hit their declared surface
layers only while orbiting that same body; their ranges are measured from
the subpoint to the target footprint. The seeded corvette and later combat
ships have a separate ground bombardment weapon. The survey ship does not.
Select an orbital ship and right-click an enemy ground unit or building to
attack; the attack hotkey followed by clicking that entity also works. The
ship keeps orbiting and fires as targets pass within its footprint. A short
beam uses the actual shot's target position. Surface weapons cannot hit an
orbital ship; anti-space batteries remain a later task.

### What a star system looks like (task 56)

- **Orbits grow geometrically.** The generator spaces orbit slots from the
  star outward, each 1.4 to 1.9 times the previous (seeded), the innermost
  at about 1500 to 2000 system units. Habitable planets take the slots in
  order of size, small ones near the star and large ones far out. The
  simulation still lays the plane out within 240 tiles of the outermost
  orbit, so the inner planets lie 35 to 90 tiles from the star.
- **Belt and moons.** Resource bodies are either **moons** (about 40 %,
  at most two per planet) or **belt bodies**. A moon lies 10 to 12 tiles
  from its planet along the planet's orbit radius, at a fixed 0.6 degree from
  its angle; belt bodies share one slot's orbit with a spread of one sixth of
  its radius, and every belt body keeps at least 1.4 degrees from any other
  body's angle. The renderer tells a moon from its angle and radius alone (no
  parent column exists); claim range (3 tiles) never reaches a neighbour.
  Every home system gets the same planets, belt slot and moons; only the
  radii and angles differ.
- **Proportions in the system view.** The disc holds 12 000 system units;
  the star is 600 and a median planet 300 units in radius (a thirtieth to a
  fiftieth of the disc, instead of a tenth). Orbit radii keep their ratios
  (linear map from the generator's units to the frame). A moon is drawn on a
  small ring about its planet, a few planet radii away, not at its true
  distance. Every body is at least 2.5 pixels in radius on screen and wears
  a soft halo below 6 pixels, so it can still be seen, hovered and
  double-clicked. Orbit rings are faint one-pixel lines: one per planet, one
  small ring per moon, one through the middle of the belt.
- **Zoom.** The planet-to-system hand-over is unchanged (the planet is drawn
  with `HOME_PLANET_RADIUS`); the whole system now fits at 48 000 units of
  camera distance, four times as far, which adds about 1.4 to the log
  distance (seven wheel steps in space), and the star map hand-over keeps
  the system at the same size on the map.
- **Resource bodies say what they hold.** Each is tinted with its resource's
  colour, has the resource's HUD icon beside it at every zoom of the system
  view and a coloured dot on the minimap. The tooltip reads
  "Tar Moon · Oil +42 / 30 s" and "Unclaimed" / "Claimed by you" / "Claimed
  by player N" / "Contested"; the yield is what `World::body_yield` reports
  (the node's income with the player's modifiers, the same term the economy
  pays), the state `World::body_claim`. The event line says "Tar Moon
  claimed: Oil +42 / 30 s", "Tar Moon lost" or "Tar Moon contested", and the
  card of a ship holding a claim says "Holds Tar Moon: Oil +42 / 30 s".

### Moving between levels (task 53)

One orbit-style camera serves the planet, the star system and the star map:
a focus point, a heading, an elevation that rises with the distance, and
the distance. Every level answers the same input:

| Input | Planet | System and star map |
| --- | --- | --- |
| Wheel | zoom | zoom toward the point under the cursor |
| Middle drag | pan | pan (the plane follows the pointer) |
| Shift + middle drag | turn the heading | turn the heading about the focus |
| Arrows, pointer at the edge | pan | pan, relative to the heading |
| Double-click | — | fly to a planet, star or system |
| Home | home view | fly back to the home planet |
| Space / Shift + Space | fly to / follow the selection | the same, at the level it is seen at |

- **No seams.** At the planet's hand-over the space camera takes the planet
  camera's exact pose (carried into the system frame), so the cross-fade
  shows one picture; above it the view eases to the plane's own pose
  (heading kept, no roll). The star map shares the system's heading and
  elevation, and the two frames meet exactly at their hand-over. Zooming
  out and back in without other input returns the planet camera to where it
  was; a turn made in space turns the planet camera about the planet's axis.
- **Zooming in lands where you look.** Over a body, the wheel anchors on it
  (keeping the picture) and comes down to it; a loaded planet is entered,
  another habitable planet is asked for and entered once its globe is
  loaded. Past the star map's hand-over the wheel enters the system nearest
  the focus. Visiting another system is: zoom out, pan, zoom in.
- **Every change of place is a flight** of short legs (out of the planet, up
  to a star-map height that fits the way, across, down), interrupted by any
  input. The camera never cuts.
- **Ships are findable at every level.** In orbit above the planet (task
  43), on the system plane, at the plane's rim toward the other end of their
  lane while between systems, and on the star map: a mark beside each system
  that has ships, an arrowhead on the lane for each ship in transit, its lane
  ahead lit and its destination ringed. A ship in transit stays selected;
  its departure and arrival are event lines, and clicking the event line
  flies the camera to it.
- **Discoverability.** Hovering a body, system or ship rings it and names it
  beside the pointer with what can be done; the pointer turns into a hand
  over what a double click enters; the help line explains the level; the
  first time a player's reach widens, the event line says how to see it.
  The minimap names the level, shows the ships, outlines the part of the
  plane in view and glides there on a click.
- The tier caps still bound everything: wheel, flights and focus requests
  (an interplanetary player cannot leave the home system's view).

## Rendering notes

- One scene per level with its own coordinate frame (surface/globe frame
  centred on the planet, system frame, star-map frame). The camera works in
  log-distance; crossing a threshold switches the active frame and
  cross-fades, so 32-bit floats stay precise at every level.
- Level of detail: surface chunks near the camera → a coarse globe mesh →
  a shaded disc → a point of light.
- The star of the current system is the sun light; the shadow map follows
  the visible part of the surface.

## Migration plan

Each step keeps the game working and every existing test passing.

1. **Topology in the simulation.** All geometry (neighbours, distance,
   stepping, circle and footprint iteration, unfolding) goes through one
   surface abstraction with two implementations: the current flat map, and
   the cube sphere. Existing scenarios keep the flat one.
2. **Sphere map generation.** One habitable planet as a cube surface from 3D
   noise, with starts, features and fairness; a flattened six-face image for review.
3. **Sphere rendering and the orbit camera.** Draw the planet as a sphere,
   place entities on it, orbit and zoom from the surface to the globe, and pick
   points on the sphere.
4. **Universe model.** Star systems, bodies and entity locations in the
   schema, generation and simulation: launch, land, move in system space,
   claim resource bodies by presence, travel between systems.
5. **Continuous zoom.** System and star-map views, frame switching, level of
   detail, and per-player camera limits by age.

Territory, fog of war, selection, victory and the computer opponent continue
in parallel on top of whichever topology is current.

## Defaults pending the owner's confirmation

| Question | Default |
| --- | --- |
| Travel between systems | fixed routes (lanes) between systems, with travel time by length — gives chokepoints to defend |
| Match size | one home system per race present, plus 2–4 neutral systems |
| Contents of a system | 1–3 habitable planets, 3–8 resource bodies |
| Planet sizes | home planets at the median; others vary within the range |
| Positive view range | camera stays on the home planet within the age's ground limit |

## Day and night (decided 2026-11-13)

The directional light that represents the sun orbits the planet orb by
default: one full orbit every `world.day_ticks` (5 game minutes by default
for now; the owner will tune it after playing), so every player gets day and
night in turn. Night is darker but stays playable. The sun direction is a
function of simulation time only, identical for every player; it is
presentation, with no gameplay effect for now. The planet's axis has no tilt by
default (`world.axial_tilt_mdeg = 0`), so there are no seasons yet.

The sky follows the light: blue by day, a short dusk and dawn, starry at
night; from orbit, space is black with stars, and every habitable planet
shows an atmosphere halo around its limb, brightest on the day side.

## Running a match (task 41)

`rom-app --sphere N` builds the same universe as `--universe`, with every
home planet's face size set to N (64–512). Other planet sizes keep the map
style's range, extended when needed to include N. `--age <ordinal>` starts
every non-neutral player in that age in universe, single-sphere and flat
matches; an explicit ordinal takes precedence over a script's `grant-age`.
`--single-planet --sphere N` preserves the isolated sphere (16–512) for
measurements and screenshots; `--single-planet` alone uses N = 192.
No arguments still selects the existing flat test/skirmish setup.

Resource terrain is discovered per player using `resource.first_age_id` and
the gather site's terrain requirements. App knowledge updates on research;
the simulation never changes the generated terrain. The current renderer
has baked terrain, without a tile-identity update channel: oil remains drawn
at every age pending that renderer work. See `tasks/41-verification.md` for
the exact remaining rendering requirements.
