# Task 21 — One world, several planets

Crates: `rom-sim`, `rom-cli`. (World model step 4b, part 1.)

## Goal

Today a `World` holds one surface (a flat map or one cube-sphere planet). The
universe has several habitable planets. Make one `World` hold **every
habitable surface of the match**, with each entity on exactly one body, so
that the next task can add orbit, system space and travel between them.

## Read first

- `AGENTS.md` — all of it. Hard rules 1–5.
- `app/docs/world-model.md` — "The universe", "Camera and visibility by age".
- `app/docs/database.md` — section 6 (star systems, bodies, routes; `entity.body_id` + `face`).
- `app/crates/rom-mapgen/src/universe.rs` — `Universe`, `Body`, `StarSystem`,
  `Route`, `recipes`, `planets`, `starts`, `Universe::expand`.
- `app/crates/rom-sim/src/` — all of it, especially `geometry.rs` (topology
  dispatch), `map.rs`, `world.rs`, `pathing.rs`, `spatial.rs`, `territory.rs`,
  `vision.rs`, `placement.rs`, `hash.rs`, `scenario.rs`.

## Build

1. **Bodies in the world.** `World` holds a list of surfaces keyed by a
   `BodyId(u32)` (the universe body id). Each surface has its own topology
   (flat or cube), terrain, path-cost grids, spatial grid, territory grid and
   per-player vision. A single-surface world (every existing constructor and
   scenario) is a world with one body, id 0 — behaviour and results unchanged
   apart from the hash format.
2. **Positions.** Add the body to positions (`Pos { body, face, x, y }` or an
   equivalent — state your choice). Everything that compares, measures or
   steps two positions must treat different bodies as unreachable / out of
   range (no path, no attack, no healing, no aura, no avoidance, no
   splash across bodies). Orders whose target is on another body are rejected
   at submit with `SimError::OtherBody` for now.
3. **Players and bodies.** A player's territory, vision, explored memory and
   remembered buildings are per body. `reveal_terrain(player, body)` takes the
   body. Each player has a **home body**; match setup reveals the home body's
   terrain only (the world model: nothing else is known before the space age).
4. **Universe construction.** `World::from_universe(rules, systems, bodies, routes, planets: BTreeMap<BodyId, (CubeSurface, terrain, nodes)>, seed, players, homes)`
   (or a small `UniverseSetup` struct — `rom-sim` cannot depend on
   `rom-mapgen`, so pass plain data). It stores the star systems, bodies
   (kind, system, orbit) and routes for later tasks, creates a surface for
   every planet given, and records which bodies are habitable but not yet
   generated (surfaces can be **added later** with `world.add_surface(body, …)`,
   since non-home planets are generated on demand — test that adding a
   surface mid-match keeps determinism).
5. **Queries for the interface:** `world.bodies()`, `world.systems()`,
   `world.routes()`, `world.surface(body)` (topology, terrain), per-body
   variants of `terrain_at`, `territory_owner_on`, `is_visible_on`,
   `is_explored_on`; and the bulk queries the app is missing today:
   `world.visibility_tiles(player, body) -> Vec<u8>` (0 unexplored, 1 explored,
   2 visible, dense tile order) and `world.territory_tiles(body) -> Vec<u32>`
   (owner per tile, 0 = none). Also `world.find_city_site(player, body, near)`:
   the nearest centre within a radius where a city passes every placement rule
   except territory (the app currently searches for this itself).
6. **Per-body work per tick** stays proportional to what is on each body: a
   body with no entities costs nothing.
7. **Hash** covers bodies, systems, routes and every per-body structure.
8. **CLI:** `rom universe … --simulate <ticks>` builds the world from the
   generated universe (home planets only), places each player's starting city
   and builders on its home body with `find_city_site`/`place_for_setup`, runs
   the given number of ticks with the existing scenario behaviour (random moves
   on each home planet) and prints `world bodies=<surfaces>/<habitable> entities=<n> tick=<t> hash=<16 hex>`.
   `rom run`, `rom battle`, `rom territory` keep working.

## Tests to add

- `single_body_world_behaves_as_before` (the flat and sphere scenarios give
  the same outcomes — survivors, kills, orders done — as on main).
- `entities_on_different_bodies_never_interact` (two armies at the same
  coordinates on two bodies: no combat, no avoidance push, no spatial hits).
- `order_to_another_body_is_rejected`.
- `vision_and_territory_are_per_body`.
- `reveal_terrain_reveals_only_that_body`.
- `adding_a_surface_mid_match_is_deterministic` (add at tick 300 in two runs → same hash).
- `bulk_visibility_matches_per_tile_queries`, `bulk_territory_matches_per_tile_queries`.
- `find_city_site_passes_placement_and_is_nearest`.
- `universe_world_same_seed_same_hash` (via the CLI path as a library call, 600 ticks).
- `idle_body_costs_nothing` (an extra empty surface does not change tick time by more than noise — or measure and report instead; state which).

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- universe --db target/rules.db --style map.many_worlds --seed 1 --factions nation.romans,nation.egyptians,nation.ashscale,nation.highwind --simulate 1200
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000 --sphere 192
cargo run --release -p rom-cli --locked -- battle --db target/rules.db --seed 3 --ticks 2400 --per-side 200 --sphere 192
```

The universe line twice gives the same output. Report timings of the two
`run` commands before and after (each must not get more than 15 % slower).

## Rules that matter most

- Integers only; no floats, no `HashMap`/`HashSet`, no clocks, threads or `rand`; everything new in the hash.
- No branching on definition keys or names in non-test code.
- Change only `rom-sim` and `rom-cli`. If `rom-app` or `rom-mapgen` stop
  compiling because of your API change, make the smallest compatible fix there
  and list it. A parallel task is editing `rom-render` and `rom-app` — keep any
  `rom-app` change minimal.
- Do not run the GUI; no window tools; no `sudo`. Stage files by name.

## Out of scope

Orbit, system space, transit, launching and landing, claiming resource bodies,
star or planet motion, the app's use of several bodies.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output and timings, your position-type
choice, files changed (and any change outside rom-sim/rom-cli), deviations and
why, anything you could not verify.
