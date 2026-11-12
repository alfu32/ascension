# Task 10 — Run the simulation in the application and draw its entities

Crates: `rom-render`, `rom-app`.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `research/02-rendering-ui-stack.md` — sections 2 and 7 (three threads: simulation → render → GPUI).
- `app/crates/rom-app/src/` and `app/crates/rom-render/src/` — all of it.
- `app/crates/rom-sim/src/lib.rs`, `world.rs`, `entity.rs`, `order.rs`,
  `scenario.rs` — the public simulation API (`World`, `Command`, `step`, `entities`, `spawn`, `submit`).
- `app/crates/rom-mapgen/src/lib.rs` — `GeneratedMap` (`grid`, `starts`, `nodes`).
- `app/crates/rom-rules/src/model.rs` — `EntityType` and its components.

## Build

1. **Match setup (`rom-app`, new module `match_setup`).** From the rules and
   the `GeneratedMap`, build a `rom_sim::World` on the generated grid with one
   player per start (teams all different), and for each player spawn — chosen
   by **properties, never by key or name**:
   - one building of the first entity type (id order) whose territory
     component has `is_city`, centred on the start;
   - five units of the first entity type that has a builder component, placed
     around it;
   - grant each player 500 of every base resource.
   Nodes from the map become entities owned by player 0 (create that neutral
   player on team 0).
2. **Simulation thread (`rom-app`, new module `sim_thread`).** Owns the
   `World`. Runs `20 × speed_pct / 100` ticks per real second (speed from
   `--speed <percent>`, default 100) using a fixed-step accumulator; never
   more than 5 ticks in one loop turn (drop the backlog instead of spiralling).
   After each tick it publishes a **render snapshot** and drains a command
   channel (commands from the UI or a script are submitted to the world).
   Clean stop when the window closes.
3. **Render snapshot (`rom-render`, plain data, no `rom-sim` dependency).**
   `EntitySnapshot { tick, entities: Vec<EntityView> }` with
   `EntityView { id: u32, kind: u16, owner: u8, x: f32, y: f32 /* tiles */, facing_deg: f32, hp_frac: f32, building: bool, footprint: (u8, u8), radius: f32, under_construction: Option<f32> }`.
   `RenderBridge::publish_snapshot(Arc<EntitySnapshot>)` keeps the **last two**
   snapshots; the render thread interpolates positions and facing between
   them by wall-clock time since the newer one arrived (clamped), matching
   entities by id, so motion is smooth at 60 fps from 20 ticks per second.
4. **Drawing (render thread).**
   - Units: instanced upright capsules or boxes sized from `radius`, standing
     on the terrain surface (sample the terrain height at the position),
     coloured by owner from a fixed palette of 9 colours (owner 0 grey),
     a small darker wedge or stripe showing facing.
   - Buildings: boxes covering the footprint, height by `kind` variation,
     owner-coloured roof; a site under construction is drawn translucent or
     lowered in proportion to progress.
   - Node entities: small yellow boxes (replace the map-node markers; keep the start pillars off by default).
   - A thin health bar above any entity with `hp_frac < 1`.
   - Instancing must handle 5,000 entities without dropping below the frame budget.
5. **Demo script (`rom-app`, `--demo`).** So there is something to watch, a
   tiny scripted driver on the simulation thread (it only submits orders):
   every player orders one builder to construct the first gather-site
   building type (id order) it can afford on valid ground near its city, and
   sends the other builders on random `move` orders within 12 tiles of the
   city every few seconds. No AI logic beyond that.
6. **Overlay.** The GPUI stats overlay also shows the simulation tick, ticks
   per second achieved and entity count.
7. **Options.** `--demo`, `--speed <percent>`, and keep `--bench`,
   `--screenshot`, `--camera`, `--boxes`. `--screenshot` gains
   `--after <seconds>` (default 1) so a shot can be taken once the demo has moved.
8. **Record.** Add bench lines to `app/docs/performance.md` for the default
   demo and for a stress case (`--demo` on a 256×256 map plus `--boxes 0`,
   with 5,000 extra wandering units spawned by a `--stress-units <n>` option).

## Tests to add (no window, no GPU)

- `rom-render`: `interpolation_is_linear_between_snapshots_and_clamped`,
  `facing_interpolates_across_the_wraparound`, `entity_missing_from_older_snapshot_appears_at_its_position`,
  `instance_transforms_sit_on_terrain_height`.
- `rom-app`: `match_setup_spawns_city_and_builders_per_player` (types chosen
  by property; counts and owners correct; nodes owned by player 0),
  `snapshot_reflects_world_state`, `fixed_step_accumulator_caps_backlog`,
  `demo_orders_are_valid` (run the demo driver headless for 600 ticks: at
  least one construction site exists and no submit returned an error other
  than a placement or price rejection).

## Rules that matter most

- Three threads, three owners: simulation thread owns the `World`; render
  thread owns every raylib object; GPUI thread owns the window. They share
  only plain snapshot data and channels/mailboxes.
- `rom-render` must not depend on `gpui`, `rom-sim`, `rom-rules`, `rom-mapgen`; `rom-app` must not call raylib.
- No game logic in `rom-app` beyond the demo driver, which only submits orders.
- Do not touch `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or the seed.
  If the simulation API lacks something you need, **list it and work around it**.
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--bench` or
  `--screenshot`, or by ending the process you started. Run with `env -u WAYLAND_DISPLAY`. No `sudo`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --screenshot target/shot-demo.png --after 20
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --stress-units 5000 --seed 1 --size 1280x800 --bench 10
```

Look at the screenshot yourself: a city, builders around it, a building site
or finished building, all standing on the terrain in an owner colour. Both
benches should report at least 55 fps; report the real numbers either way.
Copy the screenshot to `app/docs/img/demo.png`.

## Out of scope

Selection and player input to units, HUD, minimap, fog of war, shadows,
procedural meshes or materials, sound, computer opponents.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, bench lines, what the screenshot
shows, files changed, missing simulation API, deviations and why, anything you could not verify.
