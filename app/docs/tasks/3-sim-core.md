# Task 3 — Simulation core: state, tick loop, order book, straight-line movement

Crate: `rom-sim` (and `rom-cli` for one subcommand).

## Read first

- `AGENTS.md` — all of it. Hard rules 1, 2, 3 and 4 are the point of this task.
- `app/docs/database.md` — "How the database is used", "The economy is an order book".
- `app/db/schema.sql` — table `order_kind` in PART A, and in PART B the tables
  `match`, `player`, `entity`, `"order"` (these describe the state you model in memory).
- `app/crates/rom-rules/src/model.rs` and `lib.rs` — the `Rules` API you build on
  (`DefId`, `EntityType` and its components, `order_kinds`, `constant`,
  `movement_class_terrain`).

## Build

`rom-sim` depends on `rom-rules` only. No files, clocks, threads or floats.

1. **Fixed-point and random numbers.**
   - Positions are `i32` milli-tiles (`1000` = one tile). A `Pos { x, y }` type
     with integer distance helpers (squared distance in `i64`; an integer
     square root for exact distances).
   - `SimRng`: a small self-contained generator (for example PCG32 or
     SplitMix64) with `next_u32`, `range(lo, hi)`; state is part of the world
     and of the hash. Do not use the `rand` crate.
2. **World state** — `World`:
   - `tick: u64`, `rng: SimRng`, `rules: Arc<Rules>`, `map: MapGrid`,
     players, entities, orders, `next_entity_id`, `next_order_id`.
   - `MapGrid { width, height, terrain: Vec<DefId> }` (row-major, one terrain
     definition per tile) with `terrain_at(tile_x, tile_y)` and bounds checks.
   - `Player { id, team, faction: DefId, age: DefId }` — enough for ownership.
   - **Entities:** stable `EntityId(u32)` values that are never reused. Store
     them so that lookup by id is O(1) and iteration is always in ascending id
     order. Fields now: `id`, `type_id: DefId`, `owner: PlayerId`, `pos`,
     `facing` (degrees, `u16`), `hp`, `move_remainder` (see 5).
   - `spawn(type_id, owner, pos) -> Result<EntityId, SimError>` (type must be
     an entity type; `hp` starts at the type's `hp`), `despawn(id)`.
3. **Spatial grid.** Buckets of 4×4 tiles, kept up to date when entities move,
   spawn or despawn. `entities_in_radius(center, radius) -> Vec<EntityId>`
   returns ids in ascending order and uses exact squared-distance tests.
4. **Order book.**
   - `Command { player, executor: EntityId, kind: OrderKindKey, subject: Option<DefId>,
     target_entity: Option<EntityId>, target_pos: Option<Pos>, queue: bool }`.
     `queue = false` replaces the executor's queue; `true` appends.
   - `world.submit(Command) -> Result<OrderId, SimError>` validates **now**:
     executor exists and belongs to the player; `kind` exists in the rules'
     `order_kind` table; the executor's type has the component named by
     `order_kind.needs` (map the text values `movement`, `weapon`, `builder`,
     `harvester`, `healer`, `trader`, `container`, `producer`, `energy` to the
     component fields; empty = no requirement); the target shape matches
     `order_kind.target` (`none`, `point`, `entity`, `def`, `entity_or_point`).
     Valid commands are stored in a pending list and **applied at the start of
     the next tick in submission order**.
   - Each entity has an order queue. An `Order` has `id`, `kind`, targets,
     `status` (`Queued`, `Active`, `Blocked(reason)`, `Done`, `Cancelled`,
     `Failed(reason)`), `progress_ticks`, `issued_tick`. The front order is the
     active one; when it finishes the next becomes active.
   - Implement the behaviour of exactly two kinds in this task:
     - `stop`: clears the executor's queue.
     - `move` with a point target: see 5. (`move` with an entity target and
       every other kind: accepted by validation, then finished immediately
       with `Failed("unsupported")` when they become active. Later tasks
       replace this.)
   - Finished orders are removed from the queue and counted in
     `world.stats()` (`orders_done`, `orders_failed`); keep no unbounded history.
5. **Straight-line movement** (path search comes in a later task).
   - Per tick an entity with an active `move` order advances toward the target
     by `speed × terrain_speed_pct / 100 / ticks_per_second` milli-tiles, where
     `speed` is `et_movement.speed`, the terrain percentage comes from
     `movement_class_terrain` for the tile it stands on, and
     `ticks_per_second` is the rules constant `sim.ticks_per_second`.
     Carry the division remainder in `move_remainder` so no distance is lost
     to rounding over time.
   - Move along the straight line using integer arithmetic (scale the
     direction vector; no trigonometry). Update `facing` from the direction
     with an integer `atan2` approximation or a table — any deterministic
     integer method is fine.
   - If the next position would be on a tile whose terrain has no
     `movement_class_terrain` row for the unit's class, or outside the map,
     the order becomes `Failed("no_path")` and the unit stays where it is.
   - Arrival: when the remaining distance is within one step, snap to the
     target and mark the order `Done`.
   - No collision between units in this task.
6. **Tick.** `world.step()` in this fixed order: apply pending commands →
   advance each entity's active order in ascending entity id → update the
   spatial grid → `tick += 1`.
7. **State hash.** `world.hash() -> u64`: a hand-written stable hash (FNV-1a
   64 is fine) over tick, random state, next ids, every player, every entity
   and every queued order, in id order, field by field in little-endian bytes.
   Do not use `std::hash::Hash`/`DefaultHasher` (not stable across versions).
8. **CLI.** `rom run --db <rules.db> --seed <n> --ticks <t> --units <u>`:
   - Loads the rules, picks — without looking at keys or names — the first
     entity type in id order that has a movement component, and the first
     terrain in id order that its movement class can cross.
   - Builds a 128×128 map of that terrain, one player, spawns `u` units at
     random positions, orders each to move to a random point, runs `t` ticks.
   - Prints `tick=<t> entities=<u> orders_done=<n> hash=<16 hex digits>`.

## Tests to add (in `rom-sim`; build rules in tests from the real schema + seed through `rom_rules::build_rules_db` and `Rules::load`)

- `same_seed_same_hash`: the CLI scenario as a library call, 500 units, 600
  ticks, run twice → identical hash; `different_seed_different_hash`.
- `hash_changes_when_any_entity_moves`.
- `unit_reaches_target_in_expected_ticks`: a citizen (speed 1300, plain
  terrain 100 %) moving 13 tiles arrives after exactly 200 ticks.
- `terrain_speed_is_applied`: the same walk through forest (70 %) takes proportionally longer.
- `no_distance_lost_to_rounding`: a speed that does not divide evenly still
  covers the exact distance (use a hand-built map and assert the arrival tick).
- `impassable_terrain_fails_the_order` and `leaving_the_map_fails_the_order`.
- `commands_apply_next_tick_in_submission_order`.
- `queue_flag_appends_and_replace_clears`; `stop_clears_the_queue`.
- `submit_rejects_unknown_executor`, `…_foreign_owner`,
  `…_missing_component` (a building ordered to move), `…_wrong_target_shape`,
  `…_unknown_kind`.
- `unsupported_kinds_fail_when_activated`.
- `entity_ids_are_never_reused`; `iteration_is_in_ascending_id_order`.
- `spatial_query_matches_brute_force` over 1,000 random entities and 50 queries.
- `despawn_removes_from_spatial_grid`.

## Rules that matter most

- **Determinism:** integers only; no `HashMap`/`HashSet` anywhere in state or
  iteration; no `std::time`; no threads; no `rand`.
- **No branching on definition keys or names** in non-test code. The only
  text compared is `order_kind.needs` / `order_kind.target` and order kind
  keys, which are vocabulary, not content.
- Do not change the schema, the seed, `rom-rules`, `rom-render`, `rom-app`,
  or the workspace `[patch]` section. If `rom-rules` lacks an accessor you
  need, **list it in your final message and work around it**; if the schema
  lacks something structural, stop and report.
- Public items get a one-line doc comment. Typed errors with `thiserror`.
- Keep modules small: `math`, `rng`, `map`, `entity`, `spatial`, `order`, `world`, `hash`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 1200 --units 1000
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 1200 --units 1000
```

The last two commands print the same line.

## Out of scope

Path search, unit collision, combat, economy, production, fog of war,
saving, map generation, anything graphical.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands run with their real output (including the two `run`
lines), files changed, accessors you missed in `rom-rules`, and anything you
could not verify.
