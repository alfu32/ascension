# Task 28 — Orbiting sun light, shadow mapping, latitude lines

Crates: `rom-render`, `rom-app`. Data: two `game_constant` rows in the seed.

## Owner's decisions (2026-11-13)

Terminology (owner): say "the directional light that represents the sun
orbits the planet orb" — never that the sun or star orbits the planet.

- "The light should definitely orbit the planets by default. It doesn't matter
  that there might be night on one side as long as everyone gets their share
  of daylight; it is part of the game."
- Shadow mapping from the sun (it is missing today).
- Mark the equator, the prime meridian, the tropics and the polar circles on
  the minimap and on the planet model.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/world-model.md` (planet axis = `rom-geo` +y, longitude 0 through face 0, polar caps).
- `research/02-rendering-ui-stack.md` section 3 (sun and shadow map) and the
  sibling project `/home/alfu64/Development/meshcraft` (read only):
  `apps/desktop/src/gpui_lighting.rs`, `assets/shaders/scene.fs`, `scene.vs`,
  `shadow.fs`, `mesh.vs` — port the shadow technique from there (RGB-packed
  depth in an ordinary render texture, 3×3 PCF, slope-scaled bias and normal offset).
- `app/crates/rom-geo/src/lib.rs` (`latitude_mdeg`, `longitude_mdeg`, sphere mapping).
- `app/crates/rom-render/src/` (globe, orbit, scene, terrain, entities, zoom,
  universe, space, hud_minimap, worker) and `app/crates/rom-app/src/` (sim_thread, shell).

## Build

1. **Day and night are a function of simulation time.** New seed constants
   `world.day_ticks` (default 6,000 = 5 game minutes per rotation — owner's choice for now, to be tuned after playing) and
   `world.axial_tilt_mdeg` (default 0; the sun stays in the equatorial plane —
   no seasons yet). The sun's direction in planet space at tick `t` is
   longitude `phase − 360° × t / day_ticks`, latitude = tilt; `phase` is chosen
   so that at tick 0 the sun is over player one's home start (keeps today's
   first impression). The render thread computes it from the snapshot tick, so
   it follows game speed and pause and is identical for every player. A
   setting `--sun fixed` keeps today's fixed sun (for screenshots and tests).
   Read the constants in `rom-app` and pass them in the planet scene.
2. **Night stays playable.** The night side is darker but readable: ambient
   floor high enough that terrain, units and borders remain visible; a soft
   terminator band (no hard line). Unit and building colours and the HUD are
   unaffected by night. City lights are not required.
3. **Shadow map** on the globe (and on the flat map with a fixed sun): a
   directional light camera fitted to the part of the surface in view (not the
   whole planet), 2048² RGB-packed depth, 3×3 PCF, bias as in the sibling
   project; casters are terrain chunks, buildings and units; receivers are
   terrain and entities. At globe zoom (whole planet visible) use the sphere
   terminator only and skip the shadow pass. Re-render the shadow map when the
   sun moves more than a small angle or the view changes, not every frame if
   it can be avoided; keep the frame budget.
4. **Latitude and longitude lines** (graticule): equator, prime meridian,
   tropics (±23.44°) and polar circles (±66.56°). On the globe: thin lines
   drawn just above the surface (and sea), each with its own subtle colour
   (equator and prime meridian slightly stronger), fading out at street-level
   zoom; on the minimap (sphere modes): the same lines on the equirectangular
   image. A toggle key (`G`) and an option `--graticule off`. Default on.
5. **System view:** planets in the system view are lit by the system's star
   (light from the star's position), so the day side faces the star; the
   globe's sun direction at the same tick must agree with it (the planet's
   rotation phase from step 1 is defined relative to the star direction).
6. **Sky** (owner request, added while running): at street-to-mid zoom a sky
   gradient behind the terrain — blue by day, a short warm dusk and dawn, dark
   navy then black at night — with the star field fading in as the local sky
   darkens; driven by the same sun direction. From globe zoom outward, space
   stays black with stars (optional thin atmosphere rim on the day-side limb),
   blended smoothly with altitude. Same sky on the flat map. Tests:
   `sky_is_blue_at_noon_and_black_at_midnight`, `stars_fade_in_after_dusk`,
   `sky_blends_to_space_with_altitude`. Screenshots `sky-day.png` and `sky-night.png` at street level.
7. **Record** bench lines in `app/docs/performance.md` for street, mid and
   globe zoom with shadows on.

## Tests to add (no window, no GPU)

- `sun_direction_completes_one_rotation_per_day` and `sun_starts_over_the_home_start`.
- `fixed_sun_option_keeps_the_direction_constant`.
- `night_ambient_floor_keeps_terrain_visible` (shading function output for a point opposite the sun is above a threshold).
- `light_camera_covers_the_visible_surface` (frustum contains the projected view footprint).
- `graticule_lines_lie_on_their_latitudes` (sampled vertices of each line within 0.1° of its latitude; prime meridian at longitude 0).
- `minimap_graticule_rows_match_latitudes`.
- `system_view_and_globe_agree_on_the_sun` (direction from planet to star equals the globe's sun direction at the same tick).

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x720 --orbit 0,10,330,0 --no-hud --after 1 --screenshot target/sun-globe.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --speed 800 --size 1280x720 --orbit 0,10,330,0 --no-hud --after 90 --screenshot target/sun-later.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x720 --after 20 --screenshot target/shadows-street.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --zoom-all --view system,home --size 1280x720 --screenshot target/sun-system.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

Look at the screenshots: the globe with a day side, a soft terminator and a
readable night side, and the graticule lines; the same view later with the
terminator moved; buildings and units casting shadows at street level; the
system view with the home planet lit from its star. Copy them to
`app/docs/img/`. Benches at or above 55 fps; report the real numbers.

## Rules that matter most

- Raylib only on the render thread; `rom-render` may use `rom-geo` but not
  `gpui`, `rom-sim`, `rom-rules`, `rom-mapgen`; `rom-app` does not call raylib.
- The sun is presentation: do not change `rom-sim` (gameplay effects of night
  are out of scope). Only the two seed constants change in data.
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--screenshot`
  or `--bench`, or by ending the process you started. `env -u WAYLAND_DISPLAY`.
  No `sudo`. Stage files by name.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, bench lines, what each screenshot
shows, files changed, deviations and why, anything you could not verify.
