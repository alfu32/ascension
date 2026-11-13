# World model: round planets, star systems, continuous zoom

Decided by the owner on 2026-11-13. Supersedes the "one flat map with planets
set in open space" model (`database.md` section 6, `research/03-timeline.md`
"Reaching space").

## What the player experiences

- Every planet is a **sphere**. Before the space age a player can only move
  around their own planet: the camera orbits it, from street level up to a
  view of the whole globe, and no further.
- After reaching the space age the camera zooms out **continuously**: from the
  surface to the globe, to the star system (planets around their star), to
  the star map (systems as points of light). The player travels by zooming
  out, moving to another system or planet, and zooming in on it, where they
  can look and act.
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

| Before the player's space age | From the space age |
| --- | --- |
| camera bound to the home planet: orbit, zoom from surface to globe | zoom out continuously to system and star map; zoom in on any known planet |
| nothing beyond the home planet is visible | other bodies become visible as they are explored (fog of war per level) |

Players reach space one by one; the first to arrive finds the rest unclaimed.

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
| Before the space age | the whole home planet can be seen from orbit; nothing else |

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
