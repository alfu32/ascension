# Task 29 — Full content: technology tree, buildings, units (generated)

Crates: `rom-rules`, `rom-sim`, `rom-cli`. Data: `app/db/seed.sql` via a new
generator `app/db/content/generate.py`. CI: one step added.

## Why

The seed holds placeholder content (13 units and buildings, two age techs).
A player cannot get past the Classical age and has no military buildings.
`app/docs/content.md` is the design for the full content. **Implement it.**

## Read first

- `AGENTS.md` (hard rules 1, 2, 5); `app/docs/content.md` — **the specification**;
  `app/docs/database.md`; `app/db/schema.sql`; `app/db/seed.sql` (all of it).
- `app/crates/rom-rules/src/` (model, load, validate, `is_unlocked`, `cost_of_ramped`),
  `app/crates/rom-sim/src/production.rs`, `research.rs`, `construction.rs`,
  `economy.rs`, `gathering.rs`, `income.rs`, `world.rs`, `scenario.rs`.
- `app/crates/rom-cli/src/` (existing commands; `run`, `siege`, `universe` for patterns).
- `.github/workflows/ci.yml`.

## Build

1. **Generator** `app/db/content/generate.py` (Python 3, standard library
   only, deterministic output): holds the tables of `content.md` sections 2–6
   and the formulas of section 8, and rewrites the block between
   `-- BEGIN GENERATED CONTENT` and `-- END GENERATED CONTENT` in
   `app/db/seed.sql` (insert the markers before the final `COMMIT;`). It emits
   `def`, `entity_type` and every component row, `def_tag` (body and role tags
   — `biological`, `mechanical`, `infantry`, `mounted`, `vehicle`, `siege`,
   `ship`, `aircraft`, `starship`, `civilian`, `military`, `healer` …),
   `cost`, `prerequisite` (with OR-groups for age advances), `production`
   (with `ui_page`, `ui_col`, `ui_row` so palettes are tidy), `tech`,
   `tech_effect` (`set_age`, `upgrade_type`), `modifier` rows for track and
   building techs, weapons with `et_weapon_target` layers (ground units hit
   ground; anti-air and fighters hit air; ships hit fluid and ground near
   shore; starships hit space), gather sites, placement rules, completion
   bonuses, `entity_visual` is not needed. Add `python3 app/db/content/generate.py --check`
   (exit 1 if the seed differs from what it would write) and run it in CI.
2. **Remove the placeholder rows** the generated content replaces (section 1
   of `content.md`): keep their keys for the same things; remove anything else
   placeholder-only. Keep races, factions, nations, blocs, terrains, map
   styles, constants, AI profiles, orders, stances, formations, victories,
   materials, resource-body nodes. Both lint views stay empty; the seed loads.
   Add the terrain `ter.oil` and its `map_style_feature` rows (`coverage_pct` 1 on planets; clumps on flat maps).
3. **Obsolete steps** (`content.md` §7.1): `Rules::is_obsolete(def, active, owned)`
   in `rom-rules`; `rom-sim` rejects training an obsolete type with
   `SimError::Obsolete` (check before the price).
4. **Reachability** (`content.md` §7.3): `Rules::validate()` reports every
   unit, building and tech that cannot be reached from the start state
   (Ancient age; owning a city type and a builder type, chosen by component).
   The seed must validate with zero problems.
5. **Playthrough** (`content.md` §7.4): `rom playthrough --db <rules.db> --seed <n>`.
   A deterministic bot on a flat 128 × 128 map (generated with the
   one-world style; one player, plus the neutral player) with starting
   resources of 1,000,000 each and the commerce cap irrelevant: each tick it
   assigns idle builders to construct one of every building type that is
   available and not yet owned (placing by `check_placement` spiralling out
   from the city; per-city limits respected; cities when the limit allows), then
   researches at the Library the lowest unresearched level of each track and
   the next age as soon as each is unlocked, researches every other available
   tech once at its building, and trains one unit of each newly unlocked type.
   Orders only (hard rule 3). It prints one line per age reached
   (`age <key> tick=<t>`), then
   `playthrough final_age=<key> buildings=<n> units=<n> techs=<n> hash=<16 hex>`.
   It must reach `age.galactic` within 400,000 ticks. Add the scenario as a
   library function in `rom-sim` (`scenario.rs`) with a test that reaches at
   least `age.industrial` quickly (debug-build friendly) and the full run as
   an `#[ignore]` test.
6. **Docs:** `app/docs/database.md` — a short section on the generator.

## Tests to add

- `rom-rules`: `generated_seed_validates_with_no_problems`, `every_age_has_an_advance_tech`,
  `age_advance_needs_two_tracks`, `obsolete_when_successor_unlocked`,
  `unreachable_item_is_reported` (hand-written extra rows),
  `every_line_has_one_first_step_and_continuous_ages`.
- `rom-sim`: `training_an_obsolete_type_is_rejected`,
  `upgrade_tech_converts_existing_units_and_obsoletes_the_old_step`,
  `barracks_needs_military_1`, `classical_needs_two_level_one_tracks`,
  `playthrough_reaches_industrial` (fast), `playthrough_reaches_galactic` (`#[ignore]`),
  `playthrough_is_deterministic`.
- Existing tests keep passing; update only tests that relied on placeholder
  numbers or keys that changed, and list each.

## Done when (from `app/`)

```
python3 db/content/generate.py --check
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run -p rom-cli --locked -- check-rules --db target/rules.db
cargo run --release -p rom-cli --locked -- playthrough --db target/rules.db --seed 1
cargo run --release -p rom-cli --locked -- siege --db target/rules.db --seed 1
cargo run --release -p rom-cli --locked -- universe --db target/rules.db --style map.many_worlds --seed 1 --factions nation.romans,nation.egyptians,nation.ashscale,nation.highwind --simulate 3000 --space
```

The playthrough reaches `age.galactic`; report every age line and the timing.
Report counts: definitions per kind, units, buildings, techs.

## Rules that matter most

- Integers only in `rom-sim`; deterministic; no hash maps in iteration;
  everything new in the hash.
- No branching on definition keys or names in non-test code (the bot chooses
  by components, production rows, tech kinds and prerequisites).
- Change only `rom-rules`, `rom-sim`, `rom-cli`, `app/db/`, the CI file and
  `app/docs/database.md`. Do not touch `rom-render`, `rom-app`, `rom-mapgen`
  (a parallel task is editing `rom-render` and `rom-app`; if `rom-app` stops
  compiling, make the smallest compatible fix and list it).
- Names: historical peoples, ordinary vocabulary or invented; nothing from
  other games or fiction.
- Do not run the GUI.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands with real output and timings, the playthrough age lines,
definition counts, tests you had to update and why, files changed, deviations
from `content.md` and why, anything you could not verify.
