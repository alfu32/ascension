# Task 14 — National borders, placement rules and fog of war

Crate: `rom-sim` only (plus small additions to `rom-cli`).

## Read first

- `AGENTS.md` — all of it. Hard rules 1–5.
- `research/01-reference-data-model.md` — section 2 (territory) and section 5.
- `app/docs/database.md` — sections 6 and 7 (one map, tiers, players reach space one by one).
- `app/db/schema.sql` — `et_territory`, `placement_rule`, `entity_type.los`,
  `entity_type.limit_scope` (`per_city`), `age.map_scale`, `map_scale`,
  `game_constant`, PART B `entity.city_id`.
- `app/crates/rom-sim/src/` — all of it, especially `construction.rs`
  (the placement hook left for this task), `economy.rs`, `world.rs`, `map.rs`.

## Build

1. **Territory grid.** One owner per tile (`PlayerId` or none), recomputed
   **incrementally** when a territory emitter (an entity whose type has
   `et_territory` with `border_radius > 0`) is completed, destroyed, or
   changes owner, and when `border_radius` changes through modifiers (stat
   `border_radius`, evaluated with `rules.entity_stat`). Rule: a tile belongs
   to the owner whose emitter has the greatest *strength* at that tile, where
   strength = `effective_border_radius − distance` (milli-tiles), counting
   only tiles within that radius; ties go to the emitter that was completed
   first (lower completion tick, then lower id). Tiles of the `space` layer
   are never owned. Expose `world.territory_owner(tile)` and per-player
   tile counts.
2. **Cities and per-city limits.** A building placed inside the
   `economic_radius` of one of its owner's cities belongs to the nearest such
   city (`entity.city_id`). Enforce `limit_scope = per_city` at submit:
   at most `limit_count` of that type (existing + sites + queued) per city.
   Enforce the player city limit: constructing a building whose type is a
   city is rejected when the player's cities (complete + sites) reach
   `rules.player_stat("city_limit", base, …)` with base from constant
   `player.city_limit.base`.
3. **Placement rules** (replace the hook in `construction.rs`): every
   `placement_rule` row of the building type must pass:
   `in_own_territory` (every footprint tile owned by the player),
   `in_city_radius` (footprint centre within a city's `economic_radius`),
   `on_terrain` / `adjacent_terrain` (footprint on / touching the referenced terrain),
   `near_type` / `min_distance_type` (distance to the nearest entity of the
   referenced type, any owner for `min_distance`, own for `near`),
   `min_distance_enemy_city`, `on_node` (centre on a node entity of the
   referenced type), `on_layer` (footprint tiles of that layer).
   **The first city:** a player with no city may place a city anywhere
   buildable that satisfies its other rules (the starting town is placed by
   match setup and is exempt from rules entirely — add a world method for
   setup placement). City spacing: constant `city.spacing` minimum distance
   between any two cities (all owners). Each failure is a distinct `SimError`
   variant naming the rule.
4. **Visibility (fog of war).** Per player, two bit grids: `explored` (ever
   seen) and `visible` (seen now). Each entity reveals tiles within its
   effective `los` (stat `los`) around its position — circle, no
   line-of-sight occlusion yet. Allies (same team) share visibility. Update
   **incrementally**: only entities that moved into a different tile or
   whose los changed, plus spawns and deaths, change the counts — keep a
   per-player per-tile counter of viewers so removal is exact. Expose
   `world.is_visible(player, tile)`, `world.is_explored(player, tile)`, and
   `world.visible_entities(player)` (own and allied entities always; others
   only when their position tile is visible; buildings once seen stay known
   as a remembered snapshot of type/owner/position until seen again — keep
   this memory per player).
5. **Targeting respects visibility.** Automatic acquisition and `attack`
   submission consider only entities visible to the attacker's owner.
   (Projectiles already in flight still land.)
6. **Tier reach.** A player's current age has a `map_scale` with a rank.
   Regions (from the map's region grid, which the world now stores — add it
   to `World` construction and keep `MapGrid` compatible) belong to tiers:
   planet regions are tier 0 (`planetary`), space is tier 1 (`interstellar`).
   Until a player's age reaches the space tier: (a) no tile of the space layer
   counts as visible or explored for that player, (b) `move`, `construct`
   and every walking order whose target lies in a region of a higher tier is
   rejected at submit with `SimError::BeyondReach`. The world reports
   `world.reach_rank(player)`. (Nothing can cross space yet anyway; this sets
   up the per-player reveal.)
7. **Hash** covers territory, city membership, visibility counters, explored
   bits, remembered buildings and the region grid.
8. **CLI.** `rom run` and `rom battle` keep working (their scenarios place
   units with the setup method where needed). Add `rom territory --db <rules.db>
   --seed <n> --players <n> --size <W>x<H> [--ppm <file>]`: generates a map
   with `rom-mapgen`, sets up one city per start through the setup method,
   runs one tick, prints `territory players=<n> owned=<per-player tile counts>
   hash=<16 hex>`, and with `--ppm` writes the map coloured by territory owner
   over dimmed terrain. (`rom-cli` may depend on `rom-mapgen` already.)

## Tests to add

- `border_grows_from_a_completed_city_only`; `site_does_not_emit_border`.
- `contested_tile_goes_to_stronger_emitter_and_ties_to_earlier`.
- `religion_extends_temple_border_on_the_grid` (modifier through `entity_stat`).
- `destroying_an_emitter_shrinks_the_border` and `incremental_equals_full_recompute`
  (random sequence of 200 spawns/deaths/completions; compare with a full recompute each step).
- `space_tiles_are_never_owned`.
- `farm_outside_city_radius_rejected`, `barracks_outside_territory_rejected`,
  `second_library_in_same_city_rejected` (per-city limit), `city_limit_enforced`,
  `cities_too_close_rejected`, `first_city_may_be_placed_without_territory`.
- `each_placement_rule_kind_is_enforced` (hand-written seed rows for the kinds the seed lacks).
- `unit_reveals_circle_and_explored_persists_after_it_leaves`.
- `allies_share_visibility_enemies_do_not`.
- `incremental_visibility_equals_full_recompute` (random moves, 500 ticks).
- `hidden_enemy_is_not_acquired_and_cannot_be_ordered_attacked`.
- `remembered_building_persists_in_fog_until_seen_again`.
- `space_is_unseen_and_unreachable_before_the_space_age`, `…_reachable_after`
  (grant the age through a tech effect).
- Existing tests keep passing; scenarios that relied on unrestricted placement use the setup method.

## Rules that matter most

- Integers only; no `HashMap`/`HashSet`; no clocks, threads or `rand`; deterministic iteration; everything new in the hash.
- No branching on definition keys or names in non-test code (rule kinds, stat names, layer and scale keys are vocabulary).
- Change only `rom-sim` and `rom-cli`. Do not change the schema, the seed, `rom-rules`, `rom-mapgen`, `rom-render`, `rom-app`.
  If `rom-app` stops compiling because of an API change you made, make the
  smallest compatible fix there and list it.
- Performance: report `rom run --seed 7 --ticks 6000 --units 10000` before and after; visibility must not more than double it.
- Do not run the GUI; no window tools; no `sudo`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- territory --db target/rules.db --seed 1 --players 4 --size 256x256 --ppm target/territory.ppm
cargo run --release -p rom-cli --locked -- battle --db target/rules.db --seed 3 --ticks 2400 --per-side 200
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000
```

## Out of scope

Attrition, city capture and assimilation, victory conditions, diplomacy,
line-of-sight occlusion by terrain, rendering of fog or borders, crossing space.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Opus <noreply@anthropic.com>`; stage files by name;
do not push or merge. Final message: commands with real output and timings,
files changed, any change outside `rom-sim`/`rom-cli`, deviations and why,
anything you could not verify.
