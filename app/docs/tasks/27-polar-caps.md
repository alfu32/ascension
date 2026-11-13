# Task 27 — Polar ice caps and a start latitude band

Crates: `rom-mapgen` (sphere generation), `rom-rules` (two map-style columns), `rom-geo` (latitude helper). Data: schema columns, seed rows.

## Owner's request

"All planets should have poles; let's say they take 20 % of each hemisphere.
Placing users' start points should be within ±60° of the equator."

Interpretation (state it in `app/docs/world-model.md`, section "Planet surface"):
the planet's axis is the `rom-geo` sphere y axis (north = +y). Each polar cap
covers the poleward 20 % of latitude, i.e. latitude ≥ 72° (with an irregular
edge), on land **and** sea. Player starts lie within ±60° latitude.

## Read first

- `AGENTS.md` (hard rules 1, 2, 5, 9); `app/docs/world-model.md`.
- `app/crates/rom-geo/src/lib.rs` (`sphere_point`, scale 2^20), `app/crates/rom-mapgen/src/sphere.rs`
  (biomes, starts, fairness), `tests/sphere.rs`, `tests/universe.rs`.
- `app/db/schema.sql` (`map_style`, `terrain`, `movement_class_terrain`, `material`), `app/db/seed.sql`.
- `app/crates/rom-rules/src/` (loader, validation for map style knobs).

## Build

1. **`rom-geo`:** `latitude_mdeg(point: [i32; 3]) -> i32` — latitude in
   milli-degrees (−90,000..=90,000) from a sphere point, integer only (an
   `asin` approximation on the y component; document the error, target
   < 0.1°), and `longitude_mdeg(point) -> i32` (−180,000..180,000, prime
   meridian through face 0's centre — document which). Tests for both.
2. **Data.**
   - `map_style` gains `polar_cap_pct INTEGER NOT NULL DEFAULT 20` (share of
     latitude, 0–50) and `start_latitude_max_mdeg INTEGER NOT NULL DEFAULT 60000`
     (0–90,000). Load and validate them in `rom-rules`.
   - Seed: a new terrain `ter.ice` "Ice" (layer ground, **not buildable**,
     does not block sight), with a material `mat.ice` base colour `#e8eef2`,
     and `movement_class_terrain` rows so foot and wheeled units cross it at
     50 % speed (other ground classes as they cross plain, at 50 %); ships do
     not cross it. Both lint views stay empty.
3. **Generation (sphere only; flat maps unchanged).**
   - After height and sea level and **before** biomes and starts: every tile
     whose latitude is at or beyond the cap boundary becomes ice, where the
     boundary is `90° × (100 − polar_cap_pct)/100` (72° by default) perturbed
     by low-frequency 3D integer noise of ±3° so the edge is irregular. Ice
     replaces both land and sea there.
   - Biome coverage targets count only non-ice land; biomes are never placed on ice.
   - Starts are only chosen among tiles with |latitude| ≤ `start_latitude_max_mdeg`;
     the rest of the start rules and fairness stay as they are.
   - Resource nodes are not placed on ice.
4. **CLI images:** ice is drawn with its material colour (the image export
   already uses terrain colours by property — check it picks the material
   colour, or extend it the same way the app does).
5. Update `app/docs/database.md` (the two columns, ice) and
   `app/docs/world-model.md` (axis, caps, start band).

## Tests to add

- `rom-geo`: `latitude_of_axis_points_and_equator`, `latitude_error_is_within_bound`,
  `longitude_round_trip_and_prime_meridian`.
- `rom-rules`: `polar_and_start_latitude_knobs_load_and_validate`, `bad_polar_cap_pct_is_reported`.
- `rom-mapgen`: `caps_cover_the_poles_and_nothing_near_the_equator` (every tile above
  75° is ice, none below 69°), `cap_share_is_close_to_target` (ice tiles ≈ the area
  poleward of 72°, ±15 %), `starts_lie_within_the_latitude_band` (30 seeds, 2/4/8 players),
  `no_biome_or_node_on_ice`, `zero_cap_pct_makes_no_ice`, determinism, and
  `flat_generation_is_unchanged` (same hash as main).

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run -p rom-cli --locked -- check-rules --db target/rules.db
cargo run --release -p rom-cli --locked -- mapgen --db target/rules.db --style map.one_world --seed 1 --factions nation.romans,nation.egyptians,nation.greeks,nation.persians --sphere 192 --ppm target/poles.ppm --net target/poles-net.ppm
cargo run --release -p rom-cli --locked -- mapgen --db target/rules.db --style map.one_world --seed 1 --players 4 --size 256x256
```

Sphere line twice identical; flat line identical to main. Describe the
equirectangular image (caps at top and bottom, starts within the band).

## Rules that matter most

- Integers only in `rom-geo` and `rom-mapgen`; deterministic; no hash maps in iteration.
- No branching on definition keys or names in non-test code (ice is found by
  being the terrain the style's caps use — add a `map_style.polar_terrain_id`
  column referencing `terrain` instead of looking for a key; default the seed
  rows to `ter.ice`).
- Do not touch `rom-sim`, `rom-render`, `rom-app`. Parallel tasks are editing
  `rom-sim`, `rom-app` and `rom-render`.
- Do not run the GUI.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands with real output and timings, the latitude error bound,
ice share, files changed, deviations and why, anything you could not verify.
