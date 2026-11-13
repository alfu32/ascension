# Task 19 — Live simulation on the globe

Crates: `rom-app`, `rom-render`.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/world-model.md`.
- `app/crates/rom-sim/src/lib.rs`, `world.rs`, `geometry.rs` — `Pos` now has a
  `face`; `World::from_sphere`, `spawn_nodes`, `pos_of`, `offset`,
  `distance`, `topology()`, `check_placement`, `territory_owner_on`,
  `is_visible_on`, `is_explored_on`, `place_for_setup`.
- `app/crates/rom-mapgen/src/sphere.rs` — `SpherePlanet`.
- `app/crates/rom-render/src/globe.rs`, `orbit.rs`, `pick.rs`, `entities.rs`, `hud*.rs`.
- `app/crates/rom-app/src/` — all of it (match setup, sim thread, selection, demo, script, terrain).

## Build

1. **Sphere match setup.** With `--sphere <n>` build the world with
   `World::from_sphere` from the generated planet (terrain, nodes for player 0,
   factions and teams exactly as the flat setup), place each player's city
   with `place_for_setup` at its start (on the cube a start too close to a
   face edge for the city footprint moves inward the minimum needed — the
   simulation's territory command does the same; reuse that logic or call a
   shared helper) and the five builders around it using `World::offset`, so
   positions near edges are valid. Start the simulation thread as in flat mode.
   Remove the "simulation not available in sphere mode" restrictions on
   `--demo`, `--script` and `--stress-units`.
2. **Snapshots on the sphere.** `EntityView` carries the face; the render
   thread places each entity on the globe at `radius + terrain height` along
   the local up, oriented by its facing in the face's local frame (convert
   once with the same mapping the globe mesh uses). Interpolation between
   snapshots happens on the sphere (interpolate the surface position via the
   unfolded vector, or slerp the unit directions — no jumps across face edges).
   Buildings are boxes standing on the local tangent plane; health bars and
   selection rings follow the local up.
3. **Selection and orders on the sphere.** Pick events already carry a
   `SurfacePos`; the controller uses it for the world point of every order,
   for placement (`check_placement` with the face) and for box selection
   (projected entity positions, as now). The placement footprint preview is
   drawn on the globe surface, conforming to the curvature (a small grid of
   quads over the footprint tiles).
4. **Fog and borders on the globe.** Darken tiles the player has never
   explored and dim tiles not currently visible, using `is_explored_on` /
   `is_visible_on` (update a per-tile texture or vertex attribute on the
   render thread from a compact visibility snapshot published at most every
   5 ticks); draw national borders as tinted tiles along territory edges
   (`territory_owner_on`). Enemy units outside vision are not drawn
   (the snapshot already filters by what player 1 can see — make sure it does).
   Apply the same to the flat path (it has neither today).
5. **Minimap** (sphere mode): the equirectangular image with entity dots,
   fog darkening, and clicks that move the orbit camera to that longitude/latitude.
6. **HUD:** live values in sphere mode as in flat mode.

## Tests to add (no window, no GPU)

- `sphere_setup_places_city_and_builders_on_valid_tiles` (every player, n = 128).
- `entity_on_the_globe_sits_on_the_surface_at_its_tile` (render placement).
- `interpolation_across_a_face_edge_has_no_jump`.
- `orders_on_the_sphere_use_the_picked_face` (right-click on another face moves the unit there).
- `placement_on_the_sphere_matches_check_placement`.
- `snapshot_hides_enemies_outside_vision`.
- `fog_texture_matches_visibility` (sampled tiles).
- Existing tests keep passing.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1672x941 --after 30 --screenshot target/globe-live.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1672x941 --script docs/tasks/15-demo-script.txt --after 12 --screenshot target/globe-select.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x720 --orbit 0,10,330,0 --demo --after 30 --screenshot target/globe-fog.png --no-hud
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --stress-units 3000 --seed 1 --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

(Adapt the demo script if its relative positions are not valid on the sphere;
keep the flat script working.) Look at the screenshots: cities and builders
standing on the curved surface with the HUD live; a selection with rings and a
green footprint on the globe; the whole planet with explored areas lit, the
rest dark, and borders around the cities. Copy them to `app/docs/img/`
with the same names. Benches at or above 55 fps; report the real numbers.

## Rules that matter most

- Thread ownership unchanged; `rom-render` may use `rom-geo` but not `gpui`,
  `rom-sim`, `rom-rules`, `rom-mapgen`; `rom-app` does not call raylib.
- Do not modify `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or
  the seed. Missing simulation API → work around it in `rom-app` and list it.
  (A parallel task is changing `rom-rules`, `rom-mapgen`, the schema and the seed.)
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--screenshot`
  or `--bench`, or by ending the process you started. `env -u WAYLAND_DISPLAY`.
  No `sudo`. Stage files by name.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, bench lines, what each screenshot
shows, files changed, missing simulation API, deviations and why, anything you could not verify.
