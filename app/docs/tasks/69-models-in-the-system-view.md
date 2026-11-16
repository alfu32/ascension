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
   proper 3D models, just bigger on the starmap". No models for resource
   bodies; item 5 makes them bigger.
5. **Bigger resource bodies (owner, 2026-11-16: "planetoids are a tad too
   small, let's make them bigger in spite of the physical reality").** Moons,
   asteroids and small planets are drawn at 15 × (1 + size) system units
   (`body_radius` in `rom-render/src/universe.rs`), a tenth to a fifth of a
   median planet (300). Draw them about three times larger, roughly a third
   to a half of a median planet, but always smaller than the smallest
   habitable planet (`HOME_PLANET_RADIUS` × 96 / 192), in the system view and
   on the minimap, and clearly bigger at whole-system zoom and further out
   (a screen-size floor per kind, rocks about 55–75 % of the smallest planet
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
