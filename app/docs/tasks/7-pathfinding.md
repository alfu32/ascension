# Task 7 — Path search and local avoidance

Crate: `rom-sim` only. One of the hard tasks: correctness, determinism and a
bounded cost per tick all matter.

## Read first

- `AGENTS.md` — all of it. Hard rules 1–5.
- `app/db/schema.sql` — `movement_class`, `movement_class_terrain`, `et_movement`, `entity_type` (`radius`, footprint).
- `app/crates/rom-sim/src/` — all of it. Movement today is a straight line in
  the order-advance code; construction and gathering reuse it to walk to a target.
- `app/docs/tasks/STATUS.md` — follow-up notes on tasks 3 and 5.

## Build

1. **Cost grids.** For each movement class in use, a per-tile cost derived
   from `movement_class_terrain.speed_pct` (no row = impassable), plus tiles
   blocked by **building footprints** (complete or under construction;
   classes with `flies = 1` ignore terrain and buildings). Keep them up to
   date incrementally when a building appears or disappears; never rebuild
   every tick.
2. **Path search.** A* on the 8-connected tile grid for one movement class:
   - integer costs only (straight 10, diagonal 14, scaled by the destination
     tile's slowness); no cutting corners past a blocked tile;
   - deterministic tie-breaking (stable ordering by f, then h, then tile index);
   - if the goal tile is blocked, path to the nearest reachable tile to it;
   - returns a list of waypoints after a line-of-sight smoothing pass (drop a
     waypoint when the straight segment between its neighbours crosses only
     passable tiles), still integer-only.
3. **Per-tick budget.** Path requests go into a FIFO queue. Each tick the
   simulation expands at most `path.nodes_per_tick` A* nodes in total
   (a new rules constant read by name; use 20,000 if the constant is absent),
   continuing an unfinished search on the next tick. A unit waiting for its
   path stays in place with its order `Active`. The search state that
   persists across ticks is part of the world and of the state hash.
4. **Following.** A `move` order (and the walking phase of `construct`,
   `repair` and `harvest`) requests a path, then follows the waypoints with
   the existing integer stepping and terrain speed. If the next step is
   blocked (a building was placed on the path), request a new path. If no
   path exists the order ends `Failed("no_path")` as today. When the straight
   line to the target is entirely passable, skip the search (fast path).
   Replace the old straight-line-only behaviour; keep its tests meaningful
   (update them where the behaviour legitimately changed).
5. **Group moves.** Many units ordered to the same point in the same tick
   share one search: units in the same 4×4-tile bucket with the same class and
   goal reuse the result. They should not all converge on the exact same
   milli-tile: give each a small deterministic offset around the goal so
   they end up spread within a radius that grows with the group size.
6. **Local avoidance** between ground units (not flyers): each tick, after
   moving, units whose circles (`entity_type.radius`) overlap are pushed apart
   by a small integer amount along the line between their centres
   (deterministic: iterate pairs in id order via the spatial grid; when
   centres coincide, separate along a direction derived from the two ids).
   A push never moves a unit onto an impassable tile. Units standing in a
   gather slot or working on a site still take part. Keep it simple and
   stable; no steering behaviours.
7. **Performance follow-up from the ledger.** Income and idle-worker logic
   currently scan every entity every tick. Make both incremental (maintain
   per-player rate totals when a worker takes or leaves a slot or a site
   completes or disappears; keep a small set of idle-capable entities) so the
   no-economy benchmark returns close to its earlier speed. Results must be
   identical to the current behaviour — the existing economy tests must pass unchanged.
8. **Hash.** Include paths, waypoint indices, pending searches and avoidance-relevant state.
9. `rom run` keeps working. Add the option `rom run --obstacles <percent>`
   (in `rom-cli`, the one change allowed outside `rom-sim`), which scatters impassable tiles (pick, by
   property not name, a terrain the chosen movement class cannot cross) so the
   command exercises path search.

## Tests to add

- `path_goes_around_a_wall` and `…_through_the_only_gap`.
- `no_corner_cutting`.
- `unreachable_goal_paths_to_nearest_reachable_tile`; `enclosed_unit_fails_no_path`.
- `path_prefers_fast_terrain_when_cheaper` (plain vs forest detour).
- `smoothing_removes_redundant_waypoints_but_never_crosses_blocked_tiles`.
- `search_is_spread_over_ticks_under_a_small_budget` (set the budget low via an extra seed row; the unit still arrives, later).
- `flyer_ignores_terrain_and_buildings`.
- `new_building_on_the_path_triggers_repath`.
- `builder_walks_around_obstacles_to_its_site`; `worker_reaches_farm_behind_a_wall`.
- `group_shares_one_search` (count searches) and `group_spreads_at_goal` (no two centres identical; all within the expected radius).
- `overlapping_units_are_pushed_apart` and `push_never_enters_impassable_tile`.
- `same_seed_same_hash_with_obstacles` (1,000 units, 20 % obstacles, 1,200 ticks, twice).
- Existing economy and core tests pass.

## Rules that matter most

- Integers only; no `HashMap`/`HashSet`; no clocks, threads or `rand`; iteration in id order.
- The open list may be a `BinaryHeap` with a total, deterministic ordering.
- Do not change the schema or `rom-rules`. You may add the constant
  `path.nodes_per_tick` to `app/db/seed.sql` (one `game_constant` row) — that
  is the only allowed data change. Do not touch `rom-render`, `rom-app`, `rom-mapgen`.
- New modules: `pathing` (grids, search, queue) and `avoidance`. Keep `world.rs` under ~500 lines.
- Running the application is not needed. Do not use window tools.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 1200 --units 1000 --obstacles 20
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000
```

The fifth command twice gives the same line. Report the wall-clock time of the
last two commands (`time …`); the last one should be well under 10 seconds.

## Out of scope

Formations, flow fields, hierarchical search, naval/space transport, combat.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output and timings, files changed,
deviations and why, anything you could not verify.
