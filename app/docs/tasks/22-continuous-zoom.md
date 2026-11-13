# Task 22 — Continuous zoom: globe, star system, star map

Crates: `rom-render`, `rom-app`. (World model step 5.)

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/world-model.md` — all of it, especially "Camera and visibility by
  age" and "Rendering notes" (one frame per level, log-distance camera,
  cross-fade, level of detail).
- `app/crates/rom-mapgen/src/universe.rs` — `Universe`, `StarSystem`, `Body`,
  `Route`, `generate_universe`, `Universe::expand`; `app/docs/img/universe-starmap.png`.
- `app/crates/rom-render/src/` (globe, orbit, worker, scene, hud*) and
  `app/crates/rom-app/src/` (args, terrain, shell, sim_thread, match_setup).

## Build

1. **Universe scene** (`rom-render`, plain data, no `rom-mapgen` dependency):
   `UniverseScene { systems: Vec<SystemView { id, name, x, y, star_colour }>, bodies: Vec<BodyView { id, system, kind, orbit_radius, orbit_angle, n_or_size, home_race_colour }>, routes: Vec<(u32, u32)> }`,
   published once. `rom-app` builds it from the generated universe when
   started with `--universe` (style, seed, factions as today; default style
   many-worlds). In universe mode the home planet of player 1 is the globe
   that is drawn up close (reuse the sphere path: same planet scene, same
   live simulation as `--sphere` on that planet — the simulation itself still
   has one surface; another task is extending it).
2. **Levels and frames.** Four presentation levels: *surface* (street level),
   *globe*, *system*, *star map*. Each has its own coordinate frame:
   the planet frame (planet centre at origin, 1 unit = 1 tile), the system
   frame (star at origin, orbit radii scaled so the whole system fits a few
   thousand units; planets drawn with a radius that is **not** to scale — a
   habitable planet a few hundred units wide when seen at system level), and
   the star-map frame (systems spread over a few tens of thousands of units).
   The camera keeps one **log-distance** value; crossing a threshold switches
   the active frame and cross-fades over a short range so there is no pop.
   Each frame is rendered with its own near/far planes to keep precision.
3. **Drawing per level.**
   - Globe → as now.
   - System: the star as an emissive sphere with a glow sprite; orbits as thin
     rings; habitable planets as small lit spheres coloured from their
     generated planet when available (home planets) or a neutral palette
     otherwise; resource bodies as small rocks or moons; the planet the camera
     came from is the one the zoom is centred on.
   - Star map: systems as points of light with a soft halo, routes as faint
     lines, names in the HUD font next to systems in view; home systems tinted
     with their race colour once known.
   - Level of detail: at system level the home globe is replaced by a sphere
     mesh of a few hundred triangles textured with a small equirectangular
     image of the planet; at star-map level systems are sprites.
4. **Camera controls across levels.** The wheel zooms continuously through all
   levels. At system and star-map level, panning moves across the frame; a
   double-click on a planet (system level) or a system (star-map level) flies
   the camera to it and zooms in to the next level. `Home` returns to the
   player's home globe. Clicking a planet that has no generated surface yet
   shows it as a plain sphere (no zoom to surface).
4. **Limit by age.** A `max_level` parameter caps the zoom: before the
   player's space age the cap is the globe (as today); from the space age on
   it is the star map. `rom-app` sets it from player 1's age and the rules
   (the space age is the first age whose `map_scale` is not `planetary` —
   find it by data, not by key). Add `--zoom-all` to lift the cap for testing.
5. **HUD.** The minimap shows the current level: the planet image at surface
   and globe level, a small system diagram at system level, the star map at
   star-map level. A level indicator (four small pips) sits next to the minimap.
6. **Options:** `--universe`, `--zoom-all`, and `--view <level>,<target>` for
   reproducible screenshots (`globe,home`, `system,home`, `starmap`).
7. **Record** bench lines for each level in `app/docs/performance.md`.

## Tests to add (no window, no GPU)

- `log_distance_maps_to_levels_monotonically` and `crossfade_weights_sum_to_one`.
- `frame_switch_keeps_the_view_centre` (the point under the screen centre stays put when the frame changes).
- `max_level_caps_zoom` and `space_age_is_found_by_data`.
- `system_layout_fits_every_orbit` and `star_map_layout_contains_every_system`.
- `double_click_targets_the_body_under_the_cursor`.
- `universe_scene_matches_the_generated_universe` (`rom-app`).

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --zoom-all --view globe,home --size 1280x720 --screenshot target/zoom-globe.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --zoom-all --view system,home --size 1280x720 --screenshot target/zoom-system.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --zoom-all --view starmap --size 1280x720 --screenshot target/zoom-starmap.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --size 1280x720 --view starmap --screenshot target/zoom-capped.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --zoom-all --view system,home --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10
```

Look at the screenshots: the home globe; the home system with its star, orbit
rings, planets and resource bodies; the star map with systems, routes and
names; and the capped run still showing the globe (the cap holds before the
space age). Copy them to `app/docs/img/` with the same names. Benches at or
above 55 fps; report the real numbers.

## Rules that matter most

- Raylib only on the render thread; `rom-render` may use `rom-geo` but not
  `gpui`, `rom-sim`, `rom-rules`, `rom-mapgen`; `rom-app` does not call raylib.
- Do not modify `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or
  the seed. A parallel task is changing `rom-sim` (several planets in one
  world); use only today's simulation API.
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--screenshot`
  or `--bench`, or by ending the process you started. `env -u WAYLAND_DISPLAY`.
  No `sudo`. Stage files by name. No names of other games or fiction.

## Out of scope

Ships and travel, simulation on other planets, planet rotation and orbital
motion, atmosphere, nebula rendering.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, bench lines, what each screenshot
shows, files changed, deviations and why, anything you could not verify.
