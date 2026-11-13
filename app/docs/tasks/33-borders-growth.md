# Task 33 — Borders that grow: Civic research multiplies them, buildings widen cities

Crates: `rom-rules` (only if a new stat or constant must be validated), `rom-sim`,
`rom-cli`. Data: `app/db/seed.sql` (hand-written part) and
`app/db/content/generate.py` (then regenerate). Docs: `app/docs/database.md`,
`app/docs/content.md` §2 and §4.

## Owner's report (2026-11-13)

> City and national borders expand, multiplied by a factor of 1.xx with each
> Civic research; city borders also expand a little with each building added.
> Military buildings can be built outside cities, and so can spaceports.

Today: each Civic level adds a `pct` +5 modifier to `border_radius` on the city
tag (percentages add up, they do not compound); the city's **economic radius**
(`et_territory.economic_radius`, 16000) never changes — the simulation reads
it straight from the component. Military buildings and the spaceport already
use `in_own_territory` (national border), not `in_city_radius`; the owner must
be able to rely on that.

## Read first

- `AGENTS.md` — all of it.
- `app/docs/database.md` (stats, modifiers, `stack_group`), `app/db/schema.sql`
  (`stat`, `modifier`, `et_territory`, `placement_rule`, `game_constant`).
- `app/crates/rom-rules/src/modifiers.rs` (how `add`, `pct`, `mul` combine:
  `mul` values are per cent and applied one after another, so they compound).
- `app/crates/rom-sim/src/territory.rs` and `territory/` (border radius,
  economic radius, `in_city_radius`, refresh on research and on building
  completion), `placement.rs`, `world.rs` (`check_placement`), `income.rs`
  (the economic radius limits gather sites).
- `app/db/content/generate.py` (`TRACKS`, civic modifiers), the tests that pin
  the original city geometry (`PINNED_GEOMETRY` in `territory/tests.rs`,
  `tests/pathing.rs`, `victory_tests.rs`, `income.rs`).

## Build

1. **Stat.** New entity stat `economic_radius` (seed `stat` row). The
   simulation reads a city's economic radius through the stat engine
   (`entity_stat("economic_radius", base, …)`) wherever it reads
   `et_territory.economic_radius` today, exactly as it does for `border_radius`.
2. **Civic compounds.** In the generator, each Civic level gives the `city`
   tag `border_radius` `mul` 106 and `economic_radius` `mul` 106 (replacing the
   `pct` +5). Thirteen levels → ×2.13. Leave `city_limit` +1.
3. **Buildings widen a city.** Constant `city.radius_per_building` = 400
   milli-tiles and `city.radius_from_buildings_max` = 8000. A city's economic
   radius = stat value + min(max, per_building × completed own buildings
   (not cities, not sites) whose centre lies inside the city's economic radius
   **before** this bonus — no feedback loop). Recomputed when a building
   completes, is destroyed or captured, and when a modifier changes.
   The national border is not changed by buildings.
4. Every place that depends on the economic radius (placement, gather-site
   income, the app-facing query the HUD uses for the radius circle) uses the
   new value; keep the incremental structures equal to a full recount.
5. **Placement test** proving a barracks and a spaceport can be placed inside
   the national border but outside every city's economic radius, and a granary
   cannot.
6. `content.md` §2 (Civic row) and §4: describe both rules.

## Tests to add

- `rom-sim`: `civic_levels_compound_border_and_city_radius`,
  `buildings_widen_the_city_radius_up_to_the_cap`,
  `destroyed_building_shrinks_the_city_radius`,
  `military_and_spaceport_placeable_outside_city_radius`,
  plus the determinism test (scenario twice → same hash) for the territory scenario.
- The `PINNED_GEOMETRY` tests must keep passing: pin the new constants to 0 /
  the old modifiers there if needed, as was done for the radius change.

## Done when (from `app/`)

```
python3 db/content/generate.py --check
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run --release -p rom-cli -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli -- check-rules --db target/rules.db
cargo run --release -p rom-cli -- playthrough --db target/rules.db --seed 1
cargo run --release -p rom-cli -- territory --db target/rules.db --seed 1
```

and the database lint (`AGENTS.md` → Commands) prints nothing.

## Rules that matter most

- Data, not names; determinism; one way in.
- Do not modify `rom-render`, `rom-app`, `rom-ai`, `rom-mapgen`, `rom-save`.

## Finish

Commit on the current branch (stage files by name) with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge. Final message: commands with real output, files changed, schema
and seed changes, deviations and why, anything you could not verify.
