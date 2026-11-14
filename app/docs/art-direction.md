# Art direction: buildings and units

Owner decisions (2026-11-14): models are **built from parts inside the game**
(procedural, data-driven), in a **stylised, low-detail** style, with **one look
for every faction** to start; regional families come later as data.

## Style

- Clean, chunky shapes with flat-shaded faces. Surface detail (stone courses,
  brick, roof tiles, planks, thatch) comes from the material shaders, not from
  geometry and not from image files.
- Each model reads at gameplay distance (60° tilt, 30–80 tiles across the
  screen) by its **silhouette** first and its colours second.
- Soft contact shading: parts darken slightly where they meet the ground or
  another part (baked into the vertex colour), so models do not look pasted on.
- Natural materials carry most of the surface. **Team colour** covers about
  15–25 % of what is visible: roof trim or roof, banners and flags, unit tunics
  and shield faces. Never the whole model.

## Scale

- The tile is the unit of length. A standing figure is **1.0 tile** tall,
  about twice "real" scale next to buildings, so units stay readable.
- A building covers 85–95 % of its footprint; the gap shows the ground between
  neighbours. Wall height for an ordinary building is about 1.6–2.2 tiles;
  landmarks (temple, tower, city centre) go higher.
- Mounted figures about 1.4 tiles tall; vehicles, ships and aircraft are sized
  by class, not by real dimensions.

## Telling buildings apart

Each building has one **signature element** that names its job at a glance:

| Building | Signature |
| --- | --- |
| City | a cluster: central hall plus houses, highest roof in the city |
| Farm | crop rows (done) and a small barn |
| Woodcutter's camp | log piles and a saw-horse under an open shed |
| Library | stacked roof or dome over a solid block, few windows |
| Market | open stalls with striped awnings in team colour |
| Granary | round silos |
| Temple | columns and a pediment on a stepped base |
| Barracks | walled yard with a gate and a tall banner |
| Tower | tall and slim, crenellated top |
| Dock | pier reaching over the water, a crane |
| Stable | long low barn with a fenced paddock |
| Mine | headframe over a shaft, spoil heap |
| Smelter / factory | chimneys (smoke later) |

Units: silhouette by role — spear (long vertical line), bow (curve), shield
(disc or rectangle), mount (horizontal body), siege (frame and wheels).

## Ages

One family of materials and shapes per era; the same building changes look
when its owner advances.

| Era | Ages | Materials and shapes |
| --- | --- | --- |
| Early | ancient | mud brick, thatch, timber posts, palisades |
| Classical | classical | limestone, columns, terracotta roof tiles |
| Medieval | medieval | rough stone, timber framing, slate, crenellations |
| Early modern | gunpowder, enlightenment | plaster, tile, bastions, brick, sash windows |
| Industrial | industrial | red brick, iron, chimneys, sawtooth roofs |
| Modern | modern, information | concrete, glass, flat roofs, antennas |
| Future | automation and later | white composite, solar panels, domes, light strips |

## Motion

Units move (task 39): figures walk (legs and arms swing, the body bobs), strike,
thrust, throw, draw a bow or whirl a sling as their `attack` knob says, swing a
tool or raise both arms at work, and breathe when idle; beasts trot, wheels
turn, oars sweep and hulls ride the water. Each unit is at its own point of the
cycle, so a group does not move in step. Motion is presentation only.

## Budgets

| Model | Near | Far |
| --- | --- | --- |
| Building | ≤ 1,500 triangles | ≤ 300 |
| Unit figure | ≤ 300 | ≤ 80 |
| Vehicle, ship | ≤ 800 | ≤ 160 |

Construction: a foundation plate first, then the model rises from the ground
as work progresses.

## How models are reviewed

Every model has a **model id**: `<entity key>@<age>`, for example
`bld.temple@classical` or `unit.hoplite@classical` (`@any` when the look does
not depend on the age). Every part has a name inside its model, so a part is
`bld.temple@classical/roof`. Ids are printed under each model in the gallery
and on the review page; use them in instructions.

1. **Gallery in the game.** `--gallery <age|key|units|all>` lays the models
   out in a grid on plain ground, labelled with their ids (units turned
   three-quarters to the camera); `--model <id>` shows one model from four
   angles; `--parts` labels each part; `--pose <idle|move|attack|work>` sets
   what the units do and `--anim-time <seconds>` stops the motion at that
   moment of every cycle. All work with `--screenshot`.
2. **Review page.** The orchestrator publishes a private page with every
   model: its pictures, part list, parameters and status (draft, needs work,
   approved). Comment per model there or in chat, using the ids.
3. **Directions become data.** A model is a list of named parts in
   `mesh_part` rows (shape, material, size, position, knobs), so most
   directions ("steeper roof on `bld.temple@classical/roof`", "all early roofs
   thatch") are edits to the seed, rebuilt into the gallery in minutes.
   Sketches or marked-up screenshots dropped into `app/docs/art/notes/` are
   read as part of the brief.
4. **Approved models are locked.** An approved model's geometry fingerprint is
   recorded in `app/docs/art/approved.toml`; a test fails if it changes, so
   later work cannot silently alter what was signed off. Changing an approved
   model means updating its fingerprint deliberately, with the owner's say.
