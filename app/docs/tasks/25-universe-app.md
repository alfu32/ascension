# Task 25 — The app on the universe world: ships, claims and travel on screen

Crates: `rom-app`, `rom-render`.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/world-model.md`; `app/docs/hud.md`.
- Simulation API: `app/crates/rom-sim/src/universe.rs` (`World::from_universe`,
  `UniverseSetup`, `add_surface`, `bodies`, `systems`, `routes`, `home_body`,
  `visibility_tiles`, `territory_tiles`, `find_city_site`, `city_type`),
  `space.rs` (`space_of`, `body_position`, `claims`, `transits`, the `orbit`
  and `travel` order kinds — travel's destination is `target_pos.body` = the
  destination system's star body), `surface.rs`, `world.rs`.
- `app/crates/rom-app/src/` — all of it (`match_setup`, `sim_thread`,
  `selection`, `universe_view`, `terrain`, `shell`), and
  `app/crates/rom-render/src/` (`zoom`, `universe`, `scene/space`, `globe`,
  `entities`, `pick`, `vision`, `hud*`).

## Build

1. **One world for the whole universe.** In `--universe` mode build the
   simulation with `World::from_universe` from the generated universe (every
   home planet as a surface; other habitable planets registered and added
   with `add_surface` when the camera first zooms in on them — generate them
   with `Universe::expand` on the simulation thread). Every player starts on
   its own race's home planet (`find_city_site` + `place_for_setup`, builders
   around it). `--sphere` and flat modes keep their current single-surface setup.
2. **Snapshots per body.** The simulation thread publishes entity snapshots
   for the body the camera is looking at **and** for the current system's
   space (ships, markers), filtered by player one's vision on that body. The
   fog publisher uses the bulk queries (`visibility_tiles`, `territory_tiles`)
   for the viewed planet. The HUD shows player one's totals across all bodies.
3. **Space on screen** (system level): ships as small hull shapes in owner
   colours with selection rings and health bars; body markers are the planets
   and resource rocks already drawn; a claimed resource body gets a ring in its
   claimant's colour; ships in orbit cluster around their body. At star-map
   level, ships in transit are drawn as moving points along their route
   (interpolated from the transit's progress) when the transit belongs to
   player one or is visible to them (otherwise not drawn).
4. **Orders in space.** At system level, selecting ships works as on planets
   (click, box, control groups). Right-click on a planet or resource rock →
   `orbit`; right-click on empty space → `move`; right-click on an enemy ship →
   `attack`. At star-map level, right-click on another system with ships
   selected → `travel` (the simulation finds the route). The palette's ORDER
   tab shows "Orbit" and "Travel" buttons for selected ships.
5. **Building ships.** A spaceport (built on a planet once the age allows)
   trains ships from its palette; the new ship appears in system space and the
   event line says so. Add the event line text for new ships, claims won and
   claims lost (player one).
6. **Sun on the player's side.** The sun direction of each planet is chosen so
   that player one's home start is in daylight when the match starts (fixed
   direction in planet space from that start; no day–night cycle yet).
7. **Camera cap** as today (globe before the space age; star map after), but
   the cap now follows player one's real age in the universe world.
8. **Options:** `--universe` builds the universe world; keep `--zoom-all`,
   `--view`, `--screenshot`, `--bench`, `--script`. Add a script command to
   grant player one an age and resources (test only) so a screenshot can show
   ships: e.g. `grant-age <age key>` and `grant <resource key> <amount>`
   (keys typed in a script are input, not behaviour).

## Tests to add (no window, no GPU)

- `universe_world_puts_each_race_on_its_home_planet`.
- `snapshot_contains_the_viewed_body_and_its_system_space_only`.
- `fog_uses_the_viewed_planets_bulk_queries`.
- `right_click_on_a_body_orders_orbit_and_on_a_system_orders_travel`.
- `claimed_body_ring_takes_the_claimants_colour` (render model).
- `transit_point_moves_along_its_route`.
- `sun_lights_the_home_start` (the dot product of sun and start normal is positive).
- `zooming_into_an_unexpanded_planet_adds_its_surface` (headless sim thread).

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --size 1280x720 --after 15 --screenshot target/u-home.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --zoom-all --script docs/tasks/25-space-script.txt --view system,home --size 1280x720 --after 40 --screenshot target/u-ships.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --zoom-all --script docs/tasks/25-space-script.txt --view starmap --size 1280x720 --after 90 --screenshot target/u-transit.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --universe --seed 1 --zoom-all --view system,home --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10
```

Write `docs/tasks/25-space-script.txt`: grant player one the space age and
resources, build a spaceport, train two ships, orbit a resource body, then
travel to a neighbouring system. Look at the screenshots: the home town in
daylight with a live HUD; ships around a claimed resource body with a ring in
the player's colour; a ship in transit on the star map. Copy them to
`app/docs/img/`. Benches at or above 55 fps; report the real numbers.

## Rules that matter most

- Thread ownership unchanged; `rom-render` may use `rom-geo` but not `gpui`,
  `rom-sim`, `rom-rules`, `rom-mapgen`; `rom-app` does not call raylib.
- No branching on definition keys or names in non-script code.
- Do not modify `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or
  the seed. A parallel task is changing `rom-sim` (victory, capture,
  attrition). Missing simulation API → work around it and list it.
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--screenshot`
  or `--bench`, or by ending the process you started. `env -u WAYLAND_DISPLAY`.
  No `sudo`. Stage files by name.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, bench lines, what each screenshot
shows, files changed, missing simulation API, deviations and why, anything you could not verify.
