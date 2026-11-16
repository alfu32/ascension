# Task 69 — Ship and body models in the system view

Crates: `rom-render` (space scene), `rom-app` (looks handed to the space
pass), tests, docs. No change to `rom-sim`, `rom-ai`, `rom-rules`, schema or
seed, and no change to any model file: every model is locked in
`app/docs/art/approved.toml` and the test `approved_models_are_unchanged` must
stay green. Implementer: Claude (Opus). Starts after task 51 is merged.

## Why

Task 51 built the starships so they are told apart by silhouette: survey
ship (dish and boom), corvette (dart), frigate (outrigger nacelles), cruiser
(hammerhead bow), battleship (stepped wedge, spinal cannon). In the planet
view they are drawn with their composed models. In the system view every ship
is still task 53's generic box hull (`Hull::push` in
`rom-render/src/entities.rs`, called from `rom-render/src/scene/space.rs`),
told apart only by `et_view_scale.system_pct`. Space resource bodies are drawn
as spheres in their resource colour with an icon; node types have no
`entity_visual`. See "Not done" in `app/docs/tasks/51-space-age-models.md`.

## Read first

`AGENTS.md`; `app/docs/world-model.md` (system view, zoom from surface to
star map); `research/02-rendering-ui-stack.md` (threading, shaders);
`app/docs/models-in-meshcraft.md`; `app/crates/rom-render/src/scene/space.rs`,
`entities.rs` (`Hull`, `push_orbital_ship`, `ModelGpu`), the world scene's
model pass; `rom-app`'s looks (how a unit kind maps to its model per age);
`app/docs/tasks/68-*-script.txt` and `51-orbit-script.txt` for scenes with
ships in a system.

## Build

1. **Ships drawn as their models in the system view.** The space pass draws
   each ship with the same composed model (near or far level by screen size)
   the planet view uses, from the same baked meshes (share the world scene's
   `ModelGpu` or upload the same meshes once; all raylib objects on the render
   thread). Lit by the system's star from its direction, with team colour and
   the emissive materials (`mat.glow`, `mat.light_strip`, `mat.plasma`,
   `mat.laser`) self-lit as in the planet view. Size keeps following
   `et_view_scale.system_pct` so ships stay readable at system zoom.
2. **The morph from planet orbit to system view** (`morph_hull`) keeps
   working: a ship must not jump or swap shape abruptly as the camera zooms
   out; blend scale and position, keep the model.
3. **Fallback.** A ship kind with no model keeps the box hull, so nothing
   disappears.
4. **Resource bodies stay spheres.** The owner decided (2026-11-16): "not
   proper 3D models, just bigger on the starmap" (meaning the system view,
   "the solar plane map"). No models for resource
   bodies; item 5 makes them bigger.
5. **Bigger resource bodies (owner, 2026-11-16: "planetoids are a tad too
   small, let's make them bigger in spite of the physical reality").** Moons,
   asteroids and small planets are drawn at 15 × (1 + size) system units
   (`body_radius` in `rom-render/src/universe.rs`), a tenth to a fifth of a
   median planet (300). Draw them about three times larger, roughly a third
   to a half of a median planet, but always smaller than the smallest
   habitable planet (`HOME_PLANET_RADIUS` × 96 / 192), in the system view and
   on the minimap, and clearly bigger at every zoom of the system view,
   whole-system zoom above all (a screen-size floor per kind, rocks about 55–75 % of the smallest planet
   on screen), not only close up. Keep them readable and
   apart: moons must not touch their planet (widen their small ring as
   needed), belt bodies must not overlap, and hover, double-click and the
   claim ring still pick the right body. Presentation only: no change to
   simulation positions or claim range. Screenshots before and after.
6. **Frame rate.** Bench a system view with at least 40 ships
   (`--bench`, twice, with `uptime` load) before and after; the system view
   must not lose more than 10 % of frame rate on this machine.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules/playthrough unchanged); screenshots in
`app/docs/img/` of the system view with all five starship classes of two
teams side by side, the zoom morph at three steps, and a fleet at system
zoom; bench table; commit on the branch (stage by name), do not push or
merge; final message with commands and real output, files changed,
deviations, anything not verified.

## Result (2026-11-16)

- **Ships as their models (1–3).** `ShipView` carries the composed model of
  its owner's faction and age (`space_view::dress_ships`, the same
  `ModelBook::resolve` the planet view uses). The system pass draws it from
  the world scene's `ModelGpu` (no second upload): near or far mesh by how
  large it looks (`fit_model`: the distance at which the planet view would
  show it as large), team colour in the spare matrix cells, the idle cycle of
  rigged parts, emissive programs self-lit. The model shader gained a star
  mode (`starLight`): light from the star at the frame's origin, the loaded
  planet's shadow (`starShade`, a ball) and a little light from above the
  plane out in space (`starFill`, faded in with the space fade so the
  hand-over from the globe matches). On the plane a model is 1.6 × the hull's
  length (`MODEL_ON_PLANE`; the hull is as wide as it is long, a model is
  slender), still by `et_view_scale.system_pct`. A ship in orbit of the
  loaded planet morphs from the size the globe drew (the model at its own
  size) to its place on the plane, keeping the model. A ship without a model,
  or whose model did not upload, keeps the box hull.
- **Resource bodies as models (4): dropped by the owner** (2026-11-16: "not
  proper 3D models, just bigger" in the system view), replaced by item 5.
- **Bigger resource bodies (5).** Radius 85 + 20 × size (105–145 units, was
  30–60), always under the smallest planet (150). A second moon's ring clears
  the first by a rock's width; any rock closer than 60 units to another body
  is turned along its ring (presentation only). Minimap dots by drawn size
  (rocks 4.75–5.4 px, planets 6.5, star 8).
- **Follow-up after review (item 5 at whole-system zoom).** The single
  2.5-pixel floor made the bigger rocks look the same as before at the zoom
  the owner uses. Floors are now per kind out in space (`least_pixels`,
  `shown_pixels` in `universe.rs`): star 12 px, habitable planet 8 px, rock
  65 % of the smallest planet drawn at that moment (5.2 px at whole-system
  zoom; 55–75 % asserted in a test), fading to 2.5 px for every body seen
  from the ground. The globe hands over to the system's sphere at the planet
  floor, so it does not jump. The layout reserves each body's whole-system
  size (`PlacedBody::room`, at 56 units a pixel, a 720-pixel-high picture):
  moon rings and the rock spreading use it, and a body is never drawn wider
  than its room (`drawn_pixels`), so zoomed out past the whole system bodies
  shrink with it instead of touching. Within the system view (up to the
  whole-system zoom) the cap does not bind on a picture at least 720 pixels
  high; on a smaller picture (or a render scale under 100 %) rocks at
  whole-system zoom are drawn a little under the floor so they keep apart.
  Hover and double-click (`markers`) and the claim, colony and hover rings
  use the drawn size.
- **Frame rate (6).** 40 ships of two teams (`69-fleet-script.txt`),
  `--bench 12`, 1280×800, alternating binaries:

  | run | load (1 min) | fps | frame ms avg | frame ms max |
  | --- | --- | --- | --- | --- |
  | before (0.0.65) | 7.27 | 53.5 | 9.65 | 19.88 |
  | after | 7.87 | 58.7 | 9.46 | 19.37 |
  | before (0.0.65) | 8.87 | 52.6 | 9.62 | 19.08 |
  | after | 8.98 | 50.1 | 9.01 | 19.71 |
  | before (0.0.65), follow-up | 9.97 | 59.1 | 9.72 | 17.14 |
  | after, follow-up | 10.22 | 56.9 | 8.92 | 33.03 |
  | before (0.0.65), follow-up | 10.54 | 52.6 | 9.30 | 20.34 |
  | after, follow-up | 10.29 | 59.2 | 9.37 | 17.05 |

- **Screenshots** (`app/docs/img/`): `system-ship-classes.png` (the five
  classes of two teams, `69-classes-script.txt`), `system-ship-morph.png`
  (task 51's orbit scene 12, 15 and 18 wheel steps out,
  `69-morph-script.txt`), `system-fleet.png` (the 40-ship fleet before and
  after), `system-rocks.png` (a moon close up, the whole system with the
  minimap, and zoomed out past the whole system; before and after).
- **Found, not changed:** in the planet view a ship with a model is drawn at
  its own size while its pick box, health bar and hull length grow with
  `et_view_scale.planet_pct` (`scale_ship_view` scales `ModelRef.size`, which
  `ModelGpu::place` does not read).
