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
| Star map | star systems, routes between them | 3D system positions (real stars, integers) |
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
  depart along a route), with a duration. A route is charted by its first voyage and a jump
  along it is charged first (task 70, "Routes, jumps and inhibitors").

### The real sky (task 59)

The universe is a piece of the real sky in three dimensions. Every system is a
real star (`star` table, `app/docs/database.md` "Star catalogue") at its real
direction and distance from the Sun, with its real constellation and name
("Vega", "Sirius", "Epsilon Eridani"). Its planets are named after it in orbit
order, "Vega II"; resource bodies "Vega belt" and "Vega II moon".

- **Frame.** x toward right ascension 0h, y toward 6h, z toward the north
  celestial pole, the Sun at 0, 0, 0 (the equatorial frame; right-handed). The
  renderer's frame has y up: it takes x, z and -y, so seen from the Sun the
  constellations are not mirrored.
- **Radial scale.** The real distances run from 4 to over 2 600 light-years.
  The map radius of a star is `100 x sqrt(distance in light-years)`
  (Proxima Centauri about 205, Sirius 293, Vega 500, Deneb 5 100). The square
  root keeps the order of distances and keeps near neighbours apart; a
  logarithm would crowd the nearest hundred stars into a shell, and a linear
  scale would leave the nearby stars in one point. Directions are exact, so
  every constellation keeps its true shape seen from the Sun. Neighbours in
  the map are only approximately neighbours in space (the compression bends
  distances along the line of sight, never across it).
- **A match uses a region.** A seeded centre star and the nearest stars to it
  (3D, in map units) until the match has its systems; the region must lie
  within the style's `star_map_radius` of the centre. Routes join real
  neighbours: a spanning tree of the nearest pairs, two routes for every home
  system, then the shortest extra routes between true neighbours (a route is
  refused when another system lies nearer to both its ends than they are to
  each other, or when it passes within 40 units of a system). Route length is
  the integer 3D distance; travel time follows from it as before.
- **Fair homes.** Home systems are chosen among the region's stars whose local
  density (the distances to their three nearest neighbours) is within 70 to
  135 percent of the median, by the farthest-point rule, over 24 seeded
  placements: the smallest spread of that density, then the widest gap, wins.
  After the routes are drawn the universe must also pass: every system
  reachable, two routes per home, equal habitable counts, resource-body value
  within two routes at least 80 percent of the richest home's, and the two
  shortest routes of the poorest home at least 70 percent of the richest
  home's. Layouts failing this are redrawn (up to 64 centres).
- **Seeing the sky.** F2 switches the "Sky from the Sun" preset: the eye
  goes to the Sun and looks toward the middle of the systems; arrows or a drag
  turn the look; the constellation stick figures are drawn between the member
  stars that are in the match (F3 hides or shows them anywhere on the star
  map). Each constellation is named once, faintly, over its member systems.
  The star map is an ordinary 3D scene: orbit with the heading and the tilt
  (the tilt reaches 89 degrees below the plane, task 74), systems drop faint lines to
  the equatorial plane's rings, distant ones fade.

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
facing. Since task 68 the orbit is geosynchronous: the ship holds over its
subpoint and moves only when ordered (below). The system-plane position stays
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
ship moves within its orbit toward the target until it is in range, then fires. A short
beam uses the actual shot's target position. Surface weapons reach an
orbital ship only when they have an `orbit` target layer (task 58, below).

### Ships in orbit (task 68)

- **Geosynchronous.** A ship or craft in a planet's orbit stays above the same
  ground point; turning the planet in the view carries it along.
- **Right-click in orbit moves within orbit**: the ship goes to the orbit point above
  the clicked ground point along the great circle, at its own speed, and never
  descends. Attack-move works the same way.
- **Patrol** (order panel, then a click): in orbit the ship circles the planet on the
  great circle through where it is and the clicked point, fighting like an
  attack-move, until given another order; the path is drawn while it is selected.
- **Land** (order panel, for carriers, freighters and craft in orbit) comes straight
  down onto the point below, or says why it cannot (water, rough ground, a building,
  an enemy city). Lift off keeps its button.
- **Boarding only on the ground**: troops board and leave a ship only once it has
  landed; craft keep boarding and launching from a carrier in orbit.
- **Space freighter** (spaceport, starfaring): right-click a city on another planet of
  the system to run a route there; it lands beside each city in turn, and each
  arrival pays wealth by the planets' distance in the system plane.
- **Satellite** (spaceport, starfaring): launched into orbit above its spaceport,
  unarmed, sees 96 tiles (an eighth of a median planet's circumference); it moves only
  within its orbit and cannot land.
- **Sight from orbit** grows with altitude and hull size (16 to 28 tiles, the survey
  ship more); see `database.md`, "Sight".

Screenshots (scripts `tasks/68-*-script.txt`): `img/orbit-hold-before-turn.png` and
`img/orbit-hold-after-turn.png` (ships over the city before and after the view turns),
`img/orbit-patrol.png` (a patrol's great circle round the planet),
`img/orbit-landing.png` (a transport coming straight down), `img/orbit-freighter-route.png`
(a freighter between the home planet and the colony), `img/orbit-satellite-sight.png` and
`img/orbit-satellite-before-after.png` (the fog before and after a satellite goes up).

### Routes, jumps and inhibitors (task 70)

Ships fly freely inside a star system. Between systems they follow the routes, and three things
shape that (values and tables in `database.md`, "Routes and jumps"):

- **Charting.** A player knows a route once its ship has flown it. The first voyage along a route
  takes five times as long, charting it; the next is quick. Unknown routes are not drawn on the
  player's star map: a faint stub leaves each system it has seen. In the expansion age the
  spaceport's **Enhanced Sensors** reveal every route leaving a system where the player has a ship
  or a planet and remove the delay; in the galactic age **Automated Route Survey** reveals them all.
- **Charging a jump.** A ship sent along a route flies to the edge of its system and charges (6 s
  for a corvette to 12 s for a battleship); it cannot move or fire meanwhile and can be hit, and a
  new order cancels the charge. Ships sent together leave together, when the slowest has charged.
  A charging ship wears a ring that fills, in the system and on the star map; its card says
  "Charging jump to Sirius: 4 s".
- **Jump inhibitors.** A building of the space age, on a planet: enemy ships in its star system
  may leave only back the way they came, charge half as long again, and are told why when another
  route is refused. Several do not stack; destroying the building frees them at once. A system so
  held shows a six-sided ring on the star map and a faint boundary at its rim; hovering it explains.
- **Interstellar trade.** From the age of abundance (**Interstellar Trade**), a space freighter
  runs a route to an own or allied city in another star system: lift off, cross to the edge,
  charge, jump, cross, land, deliver, and back; it pays by the whole path's length.

Screenshots (scripts `tasks/70-*-script.txt`, universe seed 1, `--ai none`):
`img/jump-fleet-charging.png` (a fleet at the edge, amber charge rings filling),
`img/jump-charge-card.png` (one ship: "Charging jump to Tarf: 3 s"), `img/jump-fleet-in-transit.png`
(the fleet on its way after the charge), `img/jump-chart-before.png` and `img/jump-chart-after.png`
(a first voyage: the minimap's stub becomes a route, "Route charted between ..."),
`img/jump-routes-sensors.png` and `img/jump-routes-survey.png` (the star map with Enhanced Sensors
and with Automated Route Survey), `img/jump-interstellar-trade.png` (a freighter's route card:
"Gamma Canis Minoris → Tarf, 1 jump (1 to chart)"), `img/jump-inhibitor-refused.png` (a route
refused under an enemy inhibitor, the boundary at the rim, the help line) and
`img/jump-inhibitor-star-map.png` (its mark), `img/jump-inhibitor-model.png` (the first-look model).

### Orbital craft, carriers and ground batteries (task 58)

The airfield trains **orbital fighters** and **orbital bombers** from the
starfaring age. They take off beside the airfield like other aircraft;
**Climb to orbit** (or `lift`) takes them up to their planet's orbit by
themselves, drawn climbing. In orbit they sit beside the ships there: fighters
engage ships and craft in orbit (and aircraft while in the air), bombers strike
ships in orbit and, like a starship's bombardment, the ground below. **Land**
(since task 68; right-click moves them within the orbit) brings them back down
into the air below them. They
never leave that orbit on their own: the **carrier** (spaceport, starfaring)
takes ten craft aboard — they board it in the orbit of their planet, climbing
first if they are in the air — carries them between planets and systems, and
**Launch craft** sends them out into the orbit it holds. The transport and the
other starships take ground units only; the carrier takes craft only (container
tags).

The **ground battery** (starfaring building) fires on enemy ships and craft
orbiting above it within fourteen tiles of their ground point, so it shoots at
a carrier descending onto ground near it all the way down; the laser battery
fires up as well, less accurately. Shots between the ground and orbit, and
between craft and ships in orbit, are drawn as orange tracers.

### Carrying, landing and colonies (task 54)

Starships carry ground units: the **transport ship** (starfaring age, unarmed)
takes twenty population, every other starship one or two squads. Units board
by walking up to a ship that has landed (since task 68 only then: a ship in
orbit takes nobody aboard and sets nobody down). Selecting a carrier lists who
is aboard (portraits, as workers inside a building) and its load ("Garrison 3 /
20"); **Unload** sets them down beside a landed ship.

To land, select a carrier in orbit, move it within its orbit above open ground
(right-click) and press **Land** (task 68; the right-click landing of task 54 is
gone): any buildable ground tile without a building, outside every enemy city's
economic radius. The ship descends for `space.land_ticks` (drawn coming down from its
orbital height), lands, sets everyone down on free tiles around it and stays
as a landing zone: units can walk up and board it, right-clicking ground with
it selected sends its passengers there, and **Lift off** returns it to orbit.
On the ground it can be shot by surface weapons.

A planet that is not generated yet is generated when a ship is ordered into
its orbit (as when the camera first visits it), so the ship can enter orbit
and land. Workers set down on a planet where their player has no city may
found one outside any border (never inside another player's); it grows its
own borders, economy and production, and the planet becomes part of the
player's territory. Troops set down there fight with the normal combat and
capture or raze cities as on the home planet; the conquest victory counts
every unit and city, aboard or not. The system view rings each planet with
cities in its owners' colours, and the star map rings each system with them.
Pressing the focus key on a ship that orbits a planet whose globe is not shown
flies the camera down to that planet.

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
  fiftieth of the disc, instead of a tenth). Resource bodies are drawn far
  larger than true (owner, task 69): 85 + 20 × size units in radius (105 to
  145, a third to a half of a median planet, always under the smallest
  planet's 150). Orbit radii keep their ratios
  (linear map from the generator's units to the frame). A moon is drawn on a
  small ring about its planet, a few planet radii away, not at its true
  distance; a second moon's ring is wide enough to clear the first. Rocks
  that would touch another body are turned along their ring (about the star,
  or a moon's about its planet) until 60 units apart: presentation only, the
  simulation's positions and claim range do not change. On screen every body
  has a floor by kind out in space: a star 12 pixels in radius, a habitable
  planet 8, a resource body 65 % of the smallest planet drawn at that moment
  (so at whole-system zoom a rock is a small body beside the planets, never a
  dot); from the ground every body is a small disc of at least 2.5 pixels.
  The layout keeps bodies apart at the size they are drawn at whole-system
  zoom on a picture 720 pixels high (56 units a pixel, `PlacedBody::room`);
  zoomed out farther a body is never drawn wider than that room, so it
  shrinks with the system instead of running into its neighbours. Below 6
  pixels a body also wears a soft halo. Hover, double-click and the claim,
  colony and hover rings use the drawn size. Orbit rings are faint one-pixel lines: one per planet, one
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
a focus point, a heading, an elevation and the distance. Every level answers
the same input (task 74):

| Input | Planet | System and star map |
| --- | --- | --- |
| Wheel | zoom, one gliding step per notch | zoom toward the point under the cursor |
| Middle drag | turn the globe under the camera | turn: orbit the camera about the focus (left–right the heading, up–down the elevation) |
| Shift + middle drag | turn the heading | pan (the plane follows the pointer) |
| Arrows, pointer at the edge | pan | pan, relative to the heading |
| Double-click | — | fly to a planet, star or system |
| Home | home view | fly back to the home planet |
| Space / Shift + Space | fly to / follow the selection | the same, at the level it is seen at |

- **No seams.** At the planet's hand-over the space camera takes the planet
  camera's exact pose (carried into the system frame), so the picture goes
  on unbroken (since task 57 nothing is cross-faded at all: one camera, one
  scene, below); above it the view eases to the plane's own pose
  (heading kept, no roll). The star map shares the system's heading and
  elevation, and the two frames meet exactly at their hand-over. Zooming
  out and back in without other input returns the planet camera to where it
  was; a turn made in space turns the planet camera about the planet's axis.
- **Zooming in lands where you look.** Over a body, the wheel anchors on it
  (keeping the picture) and comes down to it; a loaded planet is entered,
  another habitable planet is asked for and entered once its globe is
  loaded. Past the star map's hand-over the wheel enters the system nearest
  the focus. Visiting another system is: zoom out, pan, zoom in.
- **One way to turn (task 74).** The middle button turns the view at every
  level and Shift + middle does the other job. The setting "Middle drag: turn /
  pan" (`--middle-drag pan`, launcher "Camera and light") swaps the two
  everywhere. Turning moves the eye about the focus, never the focus about the
  eye: dragging right swings the camera so the scene follows the pointer (near
  side right), dragging up lowers the eye toward the plane. "Invert horizontal
  turn" and "Invert vertical turn" (`--invert-horizontal-turn`,
  `--invert-vertical-turn`) reverse either direction. The flat test camera and
  the planet's heading turn follow the same rule. From the Sun (F2) a drag grabs
  the sky (it follows the pointer both ways) and the arrows look the way they
  point.
- **A low angle in space (task 74).** The space camera has one elevation for
  the system view and the star map. The first hand-over from the planet eases
  to `DEFAULT_ELEVATION`, 12 degrees above the plane (orbits as long ellipses,
  the star to the side); after that the elevation is the player's, kept for the
  match. It reaches 89 degrees above and below the plane; from below the
  picture keeps its up (the plane's normal points up the screen), so names and
  marks are never upside down, and picking and orders on the plane work from
  either side.
- **The focus stays on the plane (task 74).** In a system the focus always
  lies on the system plane: panning moves it within the plane (the ray from the
  pointer to the plane, from above or below), zoom to the pointer holds the
  plane point under the cursor (or anchors on the body under it), turning
  orbits about it. Near the horizon a pointer ray that meets the plane farther
  than four camera distances from the eye's foot, or misses it, stands for the
  point at that distance in its direction, so panning and zooming stop there
  instead of shooting off (the minimap's outline of the view is cut there too).
  On the star map the focus lies on the map's reference plane (the sky's
  equatorial plane through the Sun, which the rings and drop lines stand on),
  except where it sits on a system: after a flight to a system or a ship, while
  following, or after zooming out of a system. Any pan on the map (drag, keys,
  edge, a minimap click) glides it back onto the reference plane; zooming into
  a system glides it onto that system's plane; the wheel over a system of the
  map zooms toward the system itself, wherever it lies in depth. This star-map
  rule is provisional and lives in one place (`ZoomCamera::onto_reference_plane`).
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

### One camera, one scene (task 57)

Task 53's hand-overs swapped frames and cross-faded them; the owner saw a
jump. Now there is one perspective camera from the street to the star map
(`ZoomCamera::one_view`): the planet camera while it decides the picture,
then the space camera that continues it exactly. The planet frame, the
system frame and the star map only nest the coordinates; the same camera is
expressed in each and everything is drawn into one picture with one
projection. What changes with the distance is only what is drawn:

- **The loaded globe** is the same mesh at every distance, at its true place
  in its system; it is drawn as itself while it is more than 2.5 pixels in
  radius, then as the small textured sphere of the system view.
- **The sky from the surface.** The star and the other bodies stand in the
  sky at their true directions as small discs (the star 7 px with its glow,
  planets 3.5 px, rocks 2.5 px, each with a soft halo); they grow to their
  places as the camera leaves the planet. The directional light that
  represents the sun orbits the planet orb, and the star is where it comes
  from. Other systems are faint points of light in their true directions, as
  bright as the sky lets the stars be.
- **Space around the planet** (orbit rings, claims and colony rings, ships on
  the plane, names) fades in between the planet camera's range and the plane's
  own pose; each system's own detail (bodies, rings, ships) goes with how wide
  the system looks: all of it down to a quarter of the whole-system view, none
  at a twelfth, so the system shrinks onto the star map and then its star's
  glow swells into the map's halo. The same rule holds for every system, so
  changing the anchored system never changes the picture. The map's routes,
  names and ship marks fade in over three wheel steps past the whole system.
- **Ships** keep one look at every level. A ship with a composed model is
  drawn as that model in the system view too (task 69: the model of its
  owner's faction and age, near or far mesh by how large it looks, lit by the
  star from its direction, in the loaded planet's shadow when behind it, with
  a little light from above the plane out in space); a ship without one keeps
  task 53's hull. Above a planet hulls are drawn two to five times their true
  size by class (models at their own size); on a system's plane a quarter to
  a half of task 53's size (a model 1.6 times that length, as it is slender);
  in between, a ship in orbit of the loaded planet slides from its place
  above the ground to its place on the plane while its size blends in log
  space, starting from the size the globe drew. The sizes come from
  `et_view_scale`.
- **Wheel steps (task 74).** One notch of the wheel is one step: the window
  used to send three steps per notch (the toolkit reports three lines), so a
  notch left the low orbit, crossed the system or reached the star map. Notches
  within 80 ms of the one before (`WHEEL_GESTURE_MS`) belong to the same gesture
  and are dropped, so a misfiring or free-spinning wheel moves one step;
  fractional notches (touchpad, smooth scrolling) zoom continuously. Each step
  glides: the distance eases to its goal, 95 percent of the way in 0.2 s
  (`WHEEL_GLIDE_SECONDS`), and a step during the glide moves the goal on (or,
  turning back, starts from where the camera is); the point under the cursor is
  held frame by frame. The step is a factor of 1.2 near the ground, growing
  smoothly from one log unit below the planet camera's limit to a factor of 1.4
  by the planet's hand-over. On the test scale (a planet of 122 tiles) street →
  whole globe takes 21 notches (was 9: 28 steps of three per notch), street →
  whole system 32 (was 13), whole globe → whole system 12 (was 3 or 4) and
  whole system → whole star map 12 (was 3 or 4). The levels change at the same
  distances as before; the minimap names the level and the help line says
  where the camera is and how to turn and pan there.
- **Zoom to the cursor on the planet too:** the point of the ground under the
  cursor stays under it.
- **Ready for a 3D star map (task 59).** Star-map distances, the system
  nearest the focus and picking use all three coordinates; Shift + middle
  drag up or down tilts the camera about the focus, so a map with systems at
  different heights can be seen from any angle (since task 74 the plain middle
  drag turns, from 89 degrees above the plane to 89 below it). Task 59 fills the map with
  real stars in 3D (see "The real sky").

Screenshots: `img/camera-zoom-strip.png` (street to star map, four wheel
steps apart), `img/camera-handovers.png` (the old hand-overs, two steps
apart), `img/camera-sky.png` and `img/camera-sky-planet.png` (the star and a
planet in the evening sky), `img/camera-ship-planet.png` and
`img/camera-ship-system.png` (a transport above its spaceport, and on the
system plane). Task 74: `img/camera-system-12deg.png` (the home system at the
default 12 degrees, `tasks/74-system-script.txt`, `--view system`),
`img/camera-system-below.png` (the same from about 43 degrees below the plane,
`tasks/74-below-script.txt`), `img/camera-star-map-12deg.png` (the star map at
the default angle, `--view starmap`), `img/camera-wheel-steps.png` (six
consecutive notches out of orbit from `--view globe`) and
`img/camera-wheel-to-map.png` (0, 5, 10, 15, 20 and 24 notches: low orbit to the
star map).

## Rendering notes

- One camera for every level; each frame keeps its own coordinates near its
  origin (the planet frame centred on the planet, the system frame on its
  star, the star map) and the camera is carried into each by the similarity
  transforms that nest them, composed in doubles at draw time, so 32-bit
  floats stay precise from a building to the whole map. The layers are drawn
  far to near into one picture, each in its own depth range: the sky, the
  star map (no depth test), each system in view (the anchored one split
  around the loaded globe by distance, so bodies and ships in front of the
  planet are drawn after it), the globe.
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
