# Task 26 — Attrition, city capture, elimination and victory

Crates: `rom-sim`, `rom-cli`. Data: seed constants and victory rows only.

## Read first

- `AGENTS.md` (hard rules 1–5).
- `research/01-reference-data-model.md` section 2 (territory, cities: captured
  not destroyed, assimilation timer; attrition; supply) and section 5;
  `research/04-hud-and-screens.md` section 2 (victory and elimination options).
- `app/db/schema.sql` — `victory`, `match_option`, `et_territory.capturable`,
  `et_supply`, `stat` (`attrition_taken`, `attrition_dealt`), `modifier`
  (conditions `in_enemy_territory`, `near_supply`), PART B `victory_timer`, `player.status`.
- `app/db/seed.sql` — victories, match options, constants (`city.assimilate_ticks`).
- `app/crates/rom-sim/src/` — all of it (territory, vision, combat, world, universe, space).

## Build

1. **Attrition.** Every tick (staggered: each entity every 20 ticks, applying
   20 ticks' worth), a **unit** standing on a tile owned by a player its owner
   is at war with (different team) loses hit points:
   `base = constant combat.attrition_per_period` (new; per income period)
   modified by the owner's stat `attrition_taken` and the territory owner's
   stat `attrition_dealt` (both via the modifier engine; `pct` ops). Units
   within the radius of an own supply entity (`et_supply`) take none. City
   centres and buildings take none. Attrition never kills below 1 hit point
   when constant `combat.attrition_floor_hp` is 1 (new; set it to 1).
2. **City capture.** A building whose type has `et_territory.capturable = 1`
   does not die at 0 hit points: it becomes **neutralised** (stays at 0 HP,
   stops emitting territory, stops producing and attacking, its garrison if
   any is ejected — there is no garrison yet, ignore). While neutralised, if
   within its `economic_radius` the strongest military presence (sum of
   current HP of armed units, own and allied together) belongs to a team
   other than the owner's and **no enemy of that team is in the radius**, the
   city is captured by the strongest player of that team: owner changes, hit
   points restored to 25 %, and an **assimilation** countdown
   (`city.assimilate_ticks`) starts; during assimilation it emits territory
   at half radius and cannot produce. If the former owner's units retake the
   radius while neutralised, it is restored to 25 % for them instead. Buildings
   of that city (`city_id`) change owner with it. Repairing a neutralised city
   by its owner's builders restores it. Plunder of a city on capture: the
   capturer receives `plunder_pct` of its cost.
3. **Capitals and elimination.** Each player's first city is its capital
   (record it; `World::capital(player)`). Elimination rule from the match
   setting (`match_option` `elimination`, read through a new `World` option
   field set at construction, default `capital`):
   `conquest` — a player with no city and no unit is eliminated;
   `capital` — a player whose capital is held by another team continuously for
   constant `victory.capital_hold_ticks` (new) is eliminated (their remaining
   entities pass to the capturer);
   `sudden_death` — losing the capital eliminates at once.
   Eliminated players' status becomes `defeated`; their orders stop.
4. **Victory.** Read the `victory` rows (`rule`, `threshold`, `timer_ticks`)
   and the match's enabled victories (a `World` option, default: conquest +
   territory). `conquest`: the last team with an undefeated player wins.
   `territory`: a team owning at least `threshold` % of all owned-able land
   tiles of all surfaces starts a timer of `timer_ticks`; it stops if the
   share drops; when it runs out the team wins. Expose `world.outcome()`
   (`Playing` or `Won { team, rule, tick }`) and `world.victory_timers()`.
   After a win the simulation keeps ticking but accepts no orders.
5. **Hash** covers attrition remainders, neutralised and assimilation states,
   capitals, hold timers, timers, statuses and the outcome.
6. **CLI:** `rom siege --db <rules.db> --seed <n> [--sphere <n>]`: two players
   on a flat 96×96 map (or a sphere), each with a city; player 1 gets 40
   spearmen near player 2's city, ordered to attack it; run until an outcome
   or 12,000 ticks; print
   `siege tick=<t> outcome=<won team rule|playing> captured=<n> eliminated=<players> hash=<16 hex>`.
   With the default elimination rule the attacker should capture the city and,
   after the hold time, eliminate player 2 and win by conquest.

## Tests to add

- `attrition_hurts_units_in_enemy_territory_only`, `supply_prevents_attrition`,
  `attrition_never_kills_with_floor`, `attrition_modifiers_apply`.
- `capturable_city_is_neutralised_not_destroyed`, `city_is_captured_by_dominant_enemy_presence`,
  `contested_city_is_not_captured`, `former_owner_can_retake_a_neutralised_city`,
  `captured_city_assimilates_then_emits_full_border`, `city_buildings_change_owner_with_it`,
  `capture_pays_plunder`.
- `capital_hold_eliminates_after_timer`, `sudden_death_eliminates_at_once`,
  `conquest_eliminates_players_with_nothing_left`, `eliminated_entities_pass_to_the_capturer`.
- `last_team_standing_wins_by_conquest`, `territory_share_starts_and_stops_the_timer`,
  `territory_victory_after_timer`, `no_orders_after_victory`.
- `siege_scenario_same_seed_same_hash` (twice).
- Existing tests pass; existing battle and run outcomes unchanged where no
  territory or capturable city is involved.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run -p rom-cli --locked -- check-rules --db target/rules.db
cargo run --release -p rom-cli --locked -- siege --db target/rules.db --seed 1
cargo run --release -p rom-cli --locked -- siege --db target/rules.db --seed 1 --sphere 128
cargo run --release -p rom-cli --locked -- battle --db target/rules.db --seed 3 --ticks 2400 --per-side 200 --sphere 192
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000 --sphere 192
```

Siege lines twice identical; report the timing of the last command (not more
than 10 % slower than main).

## Rules that matter most

- Integers only; no floats, no `HashMap`/`HashSet`, no clocks, threads or `rand`; everything new in the hash.
- No branching on definition keys or names in non-test code.
- Change only `rom-sim`, `rom-cli`, and seed constants/victory rows. Do not
  change the schema, `rom-rules`, `rom-mapgen`, `rom-render`, `rom-app` (if
  `rom-app` stops compiling, make the smallest compatible fix and list it).
  A parallel task is editing `rom-app` and `rom-render`.
- Do not run the GUI.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands with real output and timings, files changed, deviations and
why, anything you could not verify.
