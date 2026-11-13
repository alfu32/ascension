# Task 18 — Render a sphere planet with an orbit camera

Crates: `rom-render`, `rom-app`. (World model step 3.)

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/world-model.md` — all of it (scale, cube sphere, camera by age, rendering notes).
- `app/crates/rom-geo/src/lib.rs` — `CubeSurface`, `Tile`, `SurfacePos`, `sphere_point`, `tile_at_point`.
- `app/crates/rom-mapgen/src/sphere.rs` — `SphereRequest`, `SpherePlanet`.
- `app/crates/rom-render/src/` and `app/crates/rom-app/src/` — all of it
  (terrain chunks, sea, stars, camera, picking in `pick.rs` — `View::terrain_point`
  is the single ray-to-ground function — HUD, minimap, screenshot and bench modes).

## Build

`rom-render` may depend on `rom-geo` (pure geometry, no game data). It must
still not depend on `gpui`, `rom-sim`, `rom-rules` or `rom-mapgen`.

1. **Planet scene.** `PlanetScene { n, kind per tile, height per tile, feature colours, starts: Vec<SurfacePos>, nodes: Vec<SurfacePos> }`
   published once (like `TerrainScene`). `rom-app` builds it from a
   `SpherePlanet` when started with `--sphere <n>` (map style, seed and
   factions as today); the flat path stays the default and keeps working.
2. **Globe mesh.** Planet radius `R = 4n / (2π)` world units (1 unit = 1 tile).
   For each face, chunks of at most 64 × 64 tiles; vertices at tile corners
   placed with the equal-angle mapping (convert `sphere_point` to floats here —
   presentation only), displaced radially by height (same vertical scale as
   the flat terrain); vertex colours as the flat terrain; normals from the
   displaced surface. Chunk edges on neighbouring faces must meet without
   cracks (shared corner positions computed identically). Sea: a sphere mesh
   at sea level, translucent, darker over deep tiles as now.
3. **Orbit camera** (`rom-render`, pure maths, testable): state = a surface
   direction (unit vector the camera looks at), heading around the local up,
   tilt, and distance above the surface. Zoom is logarithmic and continuous
   from about 8 units above the ground (street level, tilted toward the
   horizon) to about 3 R (the whole globe in view, looking at the centre);
   tilt eases toward straight-down as you zoom out. Panning (arrows, edge
   scroll, drag) rotates the direction along great circles at a speed
   proportional to the distance; heading rotates with middle-drag; `End`
   resets to the first start; `Home` shows the whole globe. **The maximum
   distance is a parameter** (`max_distance`) — the world model limits it to
   the globe before the space age; for now 3 R.
4. **Lighting:** the sun is a direction fixed in planet space; the shadow map
   (when one exists for the flat scene) is fitted to the visible part of the
   surface; at globe zoom, the night side is darker but still readable.
5. **Picking on the sphere:** a sphere-mode implementation of the single
   ray-to-ground function: intersect the ray with the displaced surface
   (sphere intersection, then refine against height along the ray) and return
   a `SurfacePos` (face, x, y in milli-tiles) via `tile_at_point`. Pick events
   carry this position. On the flat map the function returns face 0 and the
   current x/y.
6. **Markers:** starts as pillars standing along the local up; nodes as small
   boxes; placed with the same mapping. Live entities stay on the flat path
   in this task: in `--sphere` mode the simulation thread is not started (the
   next task connects the simulation on the sphere). The HUD is still drawn;
   the minimap shows an equirectangular image of the planet with the camera
   direction marked.
7. **Options:** `--sphere <n>`, and keep `--camera`, `--screenshot`, `--bench`,
   `--no-hud`. Add `--orbit <lon_deg>,<lat_deg>,<distance>,<heading_deg>` for
   reproducible sphere screenshots.
8. **Record** bench lines for the sphere at n = 192 at street, mid and globe
   zoom in `app/docs/performance.md`, plus mesh build and upload time.

## Tests to add (no window, no GPU)

- `globe_vertices_lie_on_radius_plus_height` (sampled).
- `neighbouring_chunks_share_edge_vertices` (across a face edge and at a cube corner).
- `chunk_index_counts_fit_16_bits`.
- `orbit_camera_zoom_is_monotonic_and_clamped`, `pan_moves_along_great_circles`,
  `globe_zoom_looks_at_the_centre`, `street_zoom_looks_toward_the_horizon`.
- `ray_from_camera_hits_the_surface_at_the_view_direction` and
  `pick_returns_the_tile_under_the_cursor` (round trip: tile centre → screen → pick → same tile).
- `rom-app`: `sphere_option_builds_a_planet_scene_from_the_generated_planet`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x720 --orbit 0,10,12,30 --screenshot target/globe-street.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x720 --orbit 0,10,90,0 --screenshot target/globe-mid.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x720 --orbit 0,10,330,0 --screenshot target/globe-full.png --no-hud
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

Look at the three screenshots: street level with visible curvature toward
the horizon, a mid view of a large region, and the whole round planet in
space with continents and sea. No cracks at face edges. Copy them to
`app/docs/img/globe-street.png`, `globe-mid.png`, `globe-full.png`. Both
benches at or above 55 fps; report the real numbers.

## Rules that matter most

- Raylib only on the render thread; crate boundaries as above; `rom-app` does not call raylib.
- Do not touch `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or
  the seed. A parallel task is moving the simulation onto the sphere.
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--screenshot`
  or `--bench`, or by ending the process you started. `env -u WAYLAND_DISPLAY`. No `sudo`.
- Stage files by name.

## Out of scope

Live simulation entities on the sphere, star systems and zooming beyond the
globe, atmosphere scattering, clouds, per-age camera limits (only the parameter).

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, bench lines, what each screenshot
shows, files changed, deviations and why, anything you could not verify.
