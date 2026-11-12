# Task 5 — Gathering, income, commerce cap and idle workers

Crate: `rom-sim` only.

## Read first

- `AGENTS.md` — all of it. Hard rules 1–5.
- `app/docs/database.md` and `research/01-reference-data-model.md` section 2
  ("Resource economy": workers fill slots at gather buildings; income is a
  rate limited by a commerce cap).
- `app/db/schema.sql` — PART A tables `et_harvester`, `et_gather_site`,
  `resource` (`rate_capped`), `stat`, `game_constant`.
- `app/crates/rom-sim/src/` — all of it, especially `economy.rs`,
  `construction.rs`, `order.rs`, `world.rs`. You extend this code.

## Build

1. **Slots.** When a building with `et_gather_site` becomes complete, compute
   and store its slot count:
   `min(slots_max, slots_base + slots_per_node × node_tiles)`, where
   `node_tiles` is the number of map tiles whose terrain is
   `needs_node_terrain_id` within `node_radius` (milli-tiles, measured from the
   site centre to the tile centre). With no `needs_node_terrain_id`,
   `node_tiles = 0`.
2. **`harvest` order** (executor has an `et_harvester` row; target entity is a gather site):
   - Rejected at submit unless the site belongs to the same player, is
     complete, and the executor's type has an `et_harvester` row for the
     site's resource.
   - The worker walks to the site (existing movement) until within constant
     `build.reach` of the footprint, then takes a slot: the entity records
     `work_site = Some(site)`. If no slot is free on arrival the order ends
     `Failed("no_slot")`.
   - While working the order stays `Active`. The worker leaves the slot when
     it receives any other order, is despawned, or the site is despawned
     (the order then ends `Failed("target_gone")`).
3. **Income.** Rates are "per income period" (constant
   `economy.income_period_ticks`). For each player and each base resource:
   - raw rate = Σ over workers occupying a slot at a site of that resource of
     their `et_harvester.amount` + Σ `et_gather_site.self_income` of the
     player's complete sites of that resource;
   - modified = `rules.player_stat("income.<suffix>", raw, sources)` where
     `<suffix>` is the resource key after `res.` (skip if the stat does not exist);
   - if the resource is `rate_capped`, the effective rate is
     `min(modified, cap)` with cap = `rules.player_stat("commerce_cap", base, sources)`,
     base = constant `player.commerce_cap.base`;
   - every tick add `effective / period` to the player's stock, carrying the
     remainder per player and resource so that over one full period exactly
     `effective` is added (when the rate is constant).
   - `world.income(player, resource) -> Income { raw, effective, capped: bool }`
     for display.
4. **Idle workers find work.** An entity that has at least one `et_harvester`
   row, no orders and no work site counts idle ticks. When the count reaches
   constant `citizen.auto_work_delay_ticks`, it looks for the nearest (squared
   distance, ties by lower entity id) complete gather site of its owner that
   has a free slot **not already claimed by a worker walking to it** and whose
   resource it can harvest; if found, it receives a `harvest` order exactly as
   if its owner had submitted it. If none is found the counter restarts.
   Any player order resets the counter.
5. **Hash.** Include slot counts, work sites, idle counters and income remainders.
6. Keep `rom run` and its output format working.

## Tests to add (`rom-sim`, real schema + seed; hand-written extra rows where the seed lacks a case)

- `farm_has_one_slot_and_second_worker_fails_no_slot`.
- `slots_scale_with_nearby_node_terrain_and_respect_max` (extra rows: a site
  with `slots_per_node` and a needed terrain; place it near forest tiles).
- `harvest_rejected_for_foreign_incomplete_or_wrong_resource_site`.
- `worker_adds_exact_income_over_one_period`: one citizen on a farm adds
  exactly 10 food over `economy.income_period_ticks` ticks after taking the slot.
- `income_is_spread_evenly_not_lumped`: after half a period the stock rose by 5.
- `leaving_the_slot_stops_income` (give the worker a move order).
- `commerce_cap_limits_capped_resources_only` (extra rows: lower the cap
  constant; a non-capped resource is unaffected).
- `income_modifier_applies` (extra rows: a tech with `income.food` pct +50).
- `self_income_site_pays_without_workers` (extra rows).
- `idle_worker_takes_nearest_free_site_after_delay`.
- `idle_workers_do_not_all_pick_the_same_slot`: two farms, two idle citizens → one each.
- `player_order_resets_idle_counter`.
- `site_destroyed_frees_worker`.
- `bootstrap_to_classical_age_using_only_orders`: one city, five citizens, 200
  food and 200 timber; orders only (construct farms and a library, research
  the classical age) — the player reaches `age.classical`, and running the
  scenario twice gives the same hash.

## Rules that matter most

- Integers only; no `HashMap`/`HashSet`; iteration in id order; no clocks, threads or `rand`.
- No branching on definition keys or names in non-test code (stat names,
  constant names, the `res.` suffix rule and order-kind keys are vocabulary).
- Do not change the schema, the seed, `rom-rules`, `rom-cli`, `rom-render`,
  `rom-app`, `rom-mapgen`. Missing accessor → list it and work around it;
  structural schema gap → stop and report.
- New code goes in a new module `gathering` (and `income` if you prefer); keep `world.rs` under ~500 lines.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 1200 --units 1000
```

## Out of scope

Depleting node entities, rare resources and merchants, trade routes, market,
enhancer buildings, per-city limits, territory, combat, path search.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands run with their real output, files changed, accessors you
missed, deviations and why, anything you could not verify.
