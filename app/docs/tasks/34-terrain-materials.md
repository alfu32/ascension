# Task 34 — Terrain materials up close, map colours far out; forests, mountains and farms as objects

Crates: `rom-render`, and `rom-app` only to pass material data into the
render scene. Data: `material` and `mesh` rows in `app/db/seed.sql`
(hand-written part); new shader files under `app/assets/shaders/`.

## Owner's report (2026-11-13)

1. At the **lower zoom levels** the landscape is drawn with **grass, foliage,
   stone, sand and water shaders**; only the upper zoom levels show the
   terrain like a map, with colour and little detail.
2. Features that hold resources are hard to see: **forests** and **mountains**
   must be drawn as objects — forests as foliage volumes with some **trunks
   visible at the forest edge** (two volumetric shader objects: one for the
   edge, one for the centre); mountains more graphically suggestive (rock
   massifs with ridges and snow on the high ones, not tinted flat tiles).
3. **Farms** are drawn as **rows of tall grass with wide blades**.

The camera at low zoom will be tilted about 60° from the vertical (task 35,
running in parallel), so these objects are seen from the side as well as from above.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `research/02-rendering-ui-stack.md` §3–5 (shadows, shader materials,
  shader-defined objects, **bake once before the match**).
- The sibling project `../meshcraft` (read-only reference; port from it rather
  than re-deriving): `assets/shaders/procedural.fs`/`.vs` (host template),
  `procedural_tree.glsl`, `procedural_branching.glsl`, `scene.fs`/`.vs`,
  `shadow.fs`; `docs/procedural-materials.md`, `docs/material-library.md`,
  `docs/shader-objects.md`, `docs/branching-shaders.md`, `docs/standard-shaders.md`;
  `apps/desktop/src/gpui_materials.rs`, `gpui_object_bake.rs`, `gpui_lighting.rs`.
- `app/db/schema.sql` (`material`, `mesh`, `terrain.material_id`,
  `entity_visual`), the material rows in `app/db/seed.sql`, `app/docs/database.md`.
- `app/crates/rom-render/src/` — `globe.rs` (chunked sphere terrain, per-tile
  colours, fog), `terrain.rs` (flat), `scene.rs` and `scene/` (passes, shadow,
  sky), `shadow.rs`, `sun.rs`, `entities.rs` (how buildings are drawn; farms are
  buildings with a gather site), `zoom.rs`/`orbit.rs` (distance, levels),
  `worker.rs` (startup, baking, frame loop), `stats.rs` (bench).
- `app/docs/performance.md`.

## Build

1. **Material programs.** Port meshcraft's procedural host and the built-ins
   needed: grass (wind sway), foliage, rough stone, sand, water (animated),
   snow/ice. The seed's `material.program` picks one, `params` (JSON) gives
   colours and scales. Map every land terrain to a material (`ter.plain` →
   grass, `ter.coast`/`ter.deep` → water, `ter.mountain` → stone, `ter.forest`
   → forest floor, `ter.ice` → snow, beaches = land tiles next to water → sand,
   chosen by a `material` row, not by name in code). Add the rows.
2. **Level of detail.** Within a near-zoom band the terrain uses the material
   shaders; across a blend band (tied to the camera distance, named constants)
   it cross-fades to today's flat map colours; beyond it only map colours are
   drawn. Fog of war, borders, graticule, overlays and shadows keep working in
   both. Sphere and flat.
3. **Feature objects, baked before the first tick** (`mesh.baked = 1`; nothing
   generated during a match): after map generation, bake a small set of
   variants per feature and instance them on the feature tiles:
   - **Forest:** a centre canopy volume (merged foliage clumps) on interior
     forest tiles, and an edge volume on forest tiles bordering non-forest,
     showing trunks under the canopy. Use meshcraft's tree/branching shader
     to bake a few tree variants into meshes, or volumetric foliage blobs with a
     foliage material — your choice; keep the instance count bounded (merge per
     chunk).
   - **Mountain:** rock massif meshes (ridged noise, a few variants) per mountain
     clump, height from the terrain, snow material above a height threshold.
   - **Farm:** the farm building's footprint drawn as parallel rows of tall,
     wide-bladed grass (instanced blades or baked strips, wind sway in the vertex
     shader). The visual comes from `entity_visual`/`mesh` data for the farm
     type (add `mesh` rows: `forest_edge`, `forest_centre`, `rock_massif`,
     `crop_rows` generators), never from the type's key.
   Objects cast and receive shadows; they fade out with the material LOD
   band (far out, the map colours carry the meaning).
4. **Budget.** Bake time on the sphere n=192 under 3 s (report it). Bench at or
   above 55 fps at 1280×800 on `--sphere 192 --demo`, at the home view **and**
   at the closest zoom over a forest. Add `--bench-at forest` or an equivalent
   orbit argument to place the camera.

## Tests to add (no window, no GPU)

- `material_rows_cover_every_land_terrain`, `lod_weight_blends_by_distance`,
  `forest_edge_and_centre_tiles_are_split_correctly`,
  `crop_rows_follow_the_footprint_and_facing`, `feature_instances_are_bounded_per_chunk`,
  `bake_is_deterministic_for_a_seed`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
sqlite3 /tmp/rom.db < db/schema.sql && sqlite3 /tmp/rom.db < db/seed.sql   (fresh file; lint views empty)
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --after 20 --screenshot target/materials-street.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --orbit <over a forest and a mountain, close> --no-hud --after 2 --screenshot target/forest-mountain.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --orbit 0,10,330,0 --no-hud --after 1 --screenshot target/materials-globe.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

Look at the screenshots: grass, sand, water and stone up close; forests with
trunks at their edge; mountains as massifs; the farm's crop rows (train or
script a farm if the demo has none); the globe in map colours. Copy them to
`app/docs/img/`. Report real bench and bake numbers.

## Rules that matter most

- All raylib objects on the render thread; nothing generated during a match.
- No branching on definition keys or names: materials and meshes from rows.
- `rom-render` must not depend on `gpui`, `rom-sim`, `rom-rules`, `rom-mapgen`;
  `rom-app` passes plain data.
- A parallel task (35) edits the camera tilt and zoom cap (`orbit.rs`,
  `zoom.rs`), the minimap (`hud_gpu.rs`, `hud_minimap.rs`) and the palette.
  Do not edit those; expect a merge in `worker.rs`.
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--screenshot` or
  `--bench`, or by ending the process you started. `env -u WAYLAND_DISPLAY`.
  No `sudo`. Stage files by name.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, bake time, bench lines, what each
screenshot shows, files changed, deviations and why, anything you could not verify.
