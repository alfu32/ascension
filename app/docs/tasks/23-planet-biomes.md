# Task 23 — Forests, mountains and sites at planet scale

Crate: `rom-mapgen` (sphere generation only), plus `rom-cli` images if useful.

## Problem

`map_style_feature` rows were tuned for a 256 × 256 flat map: a forest clump
of 5–9 tiles, a mountain of 2–3. On a sphere planet of 221,184 tiles
(n = 192) they are specks — see `app/docs/img/` sphere images or run
`rom mapgen … --sphere 192 --ppm`. Planets should have forests and mountain
ranges that read as regions, like the reference game's maps scaled up.

## Read first

- `AGENTS.md` (hard rules 1, 2, 9).
- `app/docs/world-model.md` — scale table.
- `app/crates/rom-mapgen/src/sphere.rs`, `lib.rs`, `tests/sphere.rs`,
  `tests/universe.rs`; `app/crates/rom-geo/src/lib.rs`.
- `app/db/seed.sql` — the map styles and their `map_style_feature` rows.

## Build (sphere generation only — the flat generator and its outputs must not change)

1. **Coverage from the style rows.** For each `region_kind = 'planet'` row
   whose feature is a **terrain**, its expected tile count on a flat 256 × 256
   map is `avg(clumps) × avg(size) × (players if per_player else 1)`.
   On a sphere, scale it by `tiles / 65,536` to get a **target coverage** for
   that terrain. (Document this rule in a comment and in
   `app/docs/database.md` section 9, one paragraph.)
2. **Regions, not specks.** Place terrain features as coherent regions:
   - Terrains that block line of sight and sit on high ground (decide by the
     terrain's properties and the style row order, not by key: e.g. the row
     whose terrain is unbuildable and blocks sight → follows elevation) are
     placed on the highest land tiles first, forming ranges along height ridges.
   - Other land terrains (forests) follow a second 3D integer noise field
     ("moisture") on mid-elevation land: the highest-moisture land tiles up to
     the target coverage, so they form large woods with ragged edges.
   - Both stay out of every start's 3-tile radius and out of the sea.
3. **Near every start** (`per_player` rows): after the global placement, each
   start must still have at least one region of that terrain within
   `start_min_dist..start_max_dist` tiles (scaled by `sqrt(tiles / 65,536)`,
   capped at 40 tiles); if not, grow a clump there as today. Fairness checks
   this per player.
4. **Nodes** (`scattered` entity features such as relic sites): count scaled
   by `tiles / 65,536` (at least the flat count), placed on valid land tiles,
   spread out (minimum spacing 12 tiles).
5. **Performance:** n = 192 still under 1.5 s release.

## Tests to add

- `terrain_coverage_matches_scaled_target` (each terrain feature within ±15 % of its target, n = 128 and 192).
- `mountains_sit_on_high_ground` (mean height of mountain tiles above the 80th land percentile).
- `forests_form_regions` (the largest forest region holds at least 15 % of all forest tiles; fewer than 5 % of forest tiles are isolated singles).
- `every_start_has_each_per_player_terrain_in_range` (30 seeds).
- `node_count_scales_with_planet_area` and `nodes_are_spaced`.
- `flat_generation_is_unchanged` (hash of the flat 256 × 256 one-world map for seed 1 equals the value on main before your change — record it first).
- `sphere_generation_is_deterministic` still holds.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- mapgen --db target/rules.db --style map.one_world --seed 1 --factions nation.romans,nation.egyptians,nation.greeks,nation.persians --sphere 192 --ppm target/biomes.ppm --net target/biomes-net.ppm
cargo run --release -p rom-cli --locked -- mapgen --db target/rules.db --style map.one_world --seed 1 --players 4 --size 256x256
```

Report the sphere line (twice, identical) with its time, and the flat line
(identical to main's). Describe the equirectangular image (forest and mountain coverage).

## Rules that matter most

- Integers only; no `rand`; no hash maps in anything iterated; deterministic.
- No branching on definition keys or names in non-test code.
- Do not touch `rom-sim`, `rom-render`, `rom-app`, the schema or the seed.
  Parallel tasks are editing `rom-sim`, `rom-render` and `rom-app`.
- Do not run the GUI.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands with real output and timings, coverage numbers per terrain,
files changed, deviations and why, anything you could not verify.
