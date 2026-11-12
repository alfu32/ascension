# Task 8 — Show a generated map in the window

Crates: `rom-render`, `rom-app`.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `research/02-rendering-ui-stack.md` — sections 2, 3 and 7.
- `app/docs/performance.md` — what the frame budget looks like on this machine.
- `app/crates/rom-render/src/` and `app/crates/rom-app/src/` — all of it (the stack proof you extend).
- `app/crates/rom-mapgen/src/lib.rs` — `GeneratedMap`, `BaseTerrains`, `Region`, `generate`.
- `app/crates/rom-rules/src/lib.rs` — `build_rules_db`, `Rules::load`; `model.rs` — `Terrain`, `Layer`.

## Build

1. **Rules and map at start-up (`rom-app`).** Build the rules in memory from
   the schema and seed embedded with `include_str!`
   (`rom_rules::build_rules_db` → `Rules::load`), then generate a map with
   `rom_mapgen::generate`. Command-line options: `--style <key>` (default: the
   first map style in id order), `--seed <n>` (default 1), `--players <n>`
   (default 4), `--map-size <W>x<H>` (default 256x256). Looking the style up
   by the key typed on the command line is fine.
2. **Scene description (`rom-render`, no dependency on `rom-mapgen`,
   `rom-rules` or `rom-sim`).** Define a plain `TerrainScene` in `rom-render`:
   width, height, per-tile `height: u8`, per-tile `kind: TileKind`
   (`Land`, `ShallowSea`, `DeepSea`, `Space`, `Feature(u16)`), a colour table
   for feature ids, `starts: Vec<(f32, f32)>`, `nodes: Vec<(f32, f32)>`.
   `rom-app` converts the `GeneratedMap` into it (features: any terrain that
   is not one of the four base terrains; colour derived from its id) and
   publishes it once through a new `RenderBridge::publish_terrain(Arc<TerrainScene>)`.
3. **Terrain mesh (render thread).** Build static meshes once per published scene:
   - one vertex per tile corner, elevation from the height field (average of
     the adjoining tiles), scaled so the highest land is a few tiles tall;
     split into chunks of at most 64×64 tiles so each mesh stays within
     16-bit index limits;
   - vertex colours: land shaded green→brown→pale by height, feature tiles
     tinted with their colour, sea-floor tiles dark;
   - flat-shaded lighting from one directional sun using normals computed
     from the height field (per-vertex normals stored in the mesh are fine);
   - **no tiles are drawn for `Space`** — open space is simply not meshed.
4. **Sea.** A translucent blue quad per planet region at sea level (a single
   large quad clipped by the terrain is enough); deep tiles should look darker
   than shallow ones because of the sea floor beneath.
5. **Space backdrop.** Clear to near-black and draw a deterministic star
   field (a few thousand small points on a large sphere or as a far quad
   layer, positions from a fixed integer hash). Stars must not swim when the
   camera pans; they may ignore parallax.
6. **Markers.** Player starts as tall thin coloured pillars; nodes as small
   yellow boxes. (Placeholders until real entities are drawn.)
7. **Camera.** Start centred on the first player start. Clamp the camera
   target to the map bounds. Raise the far zoom limit so the whole home
   planet can be seen; add a key (`Home`) that frames the whole map.
8. **Keep the proof.** `--bench` still works and now measures this scene.
   Add `--screenshot <file.png>`: render at the requested `--size`, wait
   until the terrain is built and one second has passed, write the current
   frame to a PNG **from the frame pixels already read back** (convert
   BGRA→RGBA and save with the `image` crate), then exit 0. Add
   `--camera <target_x>,<target_y>,<distance>,<yaw_deg>,<pitch_deg>` to place
   the camera for reproducible screenshots. Remove the 5,000 moving boxes from
   the default scene; keep them behind `--boxes <n>` for stress testing.
9. **Record.** Update `app/docs/performance.md` with bench lines for the
   terrain scene at 1280×800 and at the largest window this display allows,
   scale 100, and the mesh build time.

## Tests to add (no window, no GPU)

- `rom-render`: `terrain_chunks_cover_every_non_space_tile_once`,
  `chunk_index_counts_fit_16_bits`, `vertex_height_is_average_of_adjoining_tiles`,
  `space_tiles_produce_no_triangles`, `star_field_is_deterministic`.
  (Put mesh generation in pure functions that return vertex/index vectors so
  they can be tested without raylib.)
- `rom-app`: `generated_map_converts_to_terrain_scene` (tile kinds match the
  base terrains; starts and nodes are carried over in tile units).

## Rules that matter most

- All raylib calls on the render thread; `rom-render` must not depend on
  `gpui`, `rom-mapgen`, `rom-rules` or `rom-sim`; `rom-app` must not call raylib.
- Floats are fine here (this is presentation, not simulation).
- Do not touch `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or the seed.
- **Never use `xdotool`, `wmctrl` or similar to close or control windows.**
  End runs with `--bench` or `--screenshot` (they exit by themselves) or by
  ending the process you started. Run with `env -u WAYLAND_DISPLAY`.
- `unsafe` only where raylib's mesh upload requires it, each with a one-line reason.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 1 --size 1280x800 --screenshot target/shot-home.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --style map.many_worlds --map-size 384x384 --seed 1 --size 1280x800 --screenshot target/shot-worlds.png --camera 192,192,420,0,70
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --bench 10 --size 1280x800 --scale 100
```

Both PNG files exist and show terrain (look at them yourself before
reporting: land with relief, sea, black space with stars, start pillars).
The bench reports at least 55 fps.

## Out of scope

Shadows, procedural materials, textures, trees, entities from the
simulation, fog of war, borders, HUD, minimap.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, the bench lines, files changed,
deviations and why, anything you could not verify.
