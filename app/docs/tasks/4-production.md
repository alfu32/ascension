# Task 4 — Production, construction and research orders

Crate: `rom-sim` only.

## Read first

- `AGENTS.md` — all of it. Hard rules 1–5.
- `app/docs/database.md` — "The economy is an order book", "Every bonus is a modifier".
- `app/db/schema.sql` — PART A tables `production`, `cost`, `prerequisite`,
  `tech`, `tech_effect`, `entity_type` (`ramp_mode`, `ramp_group`,
  `limit_scope`, `limit_count`, `pop_cost`, `build_ticks`, footprint),
  `et_producer`, `et_builder`, `completion_bonus`, `availability`.
- `app/crates/rom-rules/src/modifiers.rs` and the unlock/availability queries
  in `rom-rules` (`cost_of`, `build_ticks`, `player_stat`, `is_unlocked`,
  `is_available`, `ActiveSources`).
- `app/crates/rom-sim/src/` — all of it; you extend this code.

## Build

1. **Player economy state.** `Player` gains `resources: BTreeMap<DefId, i64>`,
   `techs: BTreeMap<DefId, u32>` (times researched) and helpers:
   - `active_sources()` → the player's researched techs, current age and
     faction as a `rom_rules::ActiveSources`.
   - `owned_types()` → the set of entity types the player has at least one
     **completed** entity of.
   - `world.grant(player, resource, amount)` for test and scenario setup.
   - Population used = sum of `pop_cost` of the player's entities of class
     `unit`. Population cap = `rules.player_stat("pop_cap", base, …)` with
     base = constant `player.pop_cap.base`.
2. **Entities** gain `build_progress: Option<u32>` (`Some(ticks_done)` while
   under construction, `None` when complete). An incomplete building cannot
   produce and does not count in `owned_types()`.
3. **Costs.** `world.price(player, def) -> Vec<(DefId, i64)>`:
   `rules.cost_of(def, sources)` computed on **base + ramp × n**, where `n` is
   the number of the player's entities (complete or not) plus queued train /
   construct orders — of the same type when `ramp_mode = per_type`, or of any
   type sharing the `ramp_group` when `per_group`; `none` → no ramp. (Apply
   the ramp to the base amount first, then the cost modifiers.) Techs have no ramp.
4. **Common checks at `submit`** for `train`, `research`, `construct`, in this
   order, each with its own `SimError` variant:
   1. a `production` row links the executor's type to the subject with the matching mode;
   2. the executor is complete;
   3. `rules.is_available(subject, player.faction)`;
   4. `rules.is_unlocked(subject, sources, owned_types)`;
   5. limits: `limit_scope = per_player` → player's count (existing + queued)
      below `limit_count`; `per_match` → the same across all players;
      `per_city` and `none` → no check in this task;
   6. the producer's queue has room (`et_producer.queue_limit`) — train/research only;
   7. the player can pay `price`; if so the amount is **deducted at submit**
      and stored on the order as its payment.
5. **`train`** (executor has `et_producer`; subject is an entity type):
   - Runs in the producer's order queue; `et_producer.parallel` orders at the
     front progress at once.
   - Duration = `rules.build_ticks(subject, sources)` evaluated when the order
     becomes active.
   - On completion: if population used + `pop_cost` exceeds the cap, the order
     stays `Blocked("pop_cap")` and is retried every tick; otherwise spawn the
     unit for the same owner just outside the producer's footprint (choose a
     deterministic side; clamp inside the map) and mark the order `Done`.
6. **`research`** (subject is a tech):
   - Rejected at submit if the tech is already researched and not
     `repeatable`, or already queued anywhere by this player, or if another
     tech of the same non-empty `exclusive_group` is researched or queued.
   - Duration = `tech.research_ticks` adjusted by player stat `research_time`
     (`rules.player_stat("research_time", research_ticks, sources)`).
   - On completion add the tech to the player and apply its `tech_effect` rows:
     - `set_age` → the player's age becomes the target;
     - `change_faction` → the player's faction becomes the target;
     - `upgrade_type` → every entity of the player whose type is `from_def_id`
       becomes `target_def_id`, keeping its hit-point percentage;
     - every other effect kind: no action in this task (they are derived from
       the tech being present, or belong to later tasks).
7. **`construct`** (executor has `et_builder`; subject is a building type;
   target is a point = centre of the footprint, snapped to the tile grid):
   - Placement check at submit: the whole footprint is inside the map; every
     tile's terrain is `buildable`; it overlaps no other building footprint
     (complete or not). Territory and other `placement_rule` rows are **out of
     scope** — leave a single clearly named hook function that currently returns OK.
   - When the command is applied the **site** is created at once: a building
     entity with `build_progress = Some(0)` and hit points at constant
     `build.site_start_hp_pct` percent of its maximum.
   - The builder walks toward the site (existing straight-line movement) and
     stops when within constant `build.reach` of the footprint; each tick in
     reach it adds `et_builder.build_pct / 100` ticks of progress (carry the
     remainder). Hit points rise in proportion to progress.
   - When progress reaches `rules.build_ticks(type, sources)`: the building is
     complete, hit points are full, the owner receives every
     `completion_bonus` row, and the order is `Done`.
   - **Helping:** the order kind `repair` with an entity target that is an
     unfinished site of the same owner makes another builder walk there and
     add progress the same way. (`repair` on a finished building: `Failed("unsupported")` for now.)
   - If the site is destroyed or completed by someone else, the order ends (`Failed("target_gone")` / `Done`).
8. **Cancel and refund.** `stop` on an executor cancels its queue as before
   and now **refunds** each cancelled train/research order's stored payment
   multiplied by constant `economy.cancel_refund_pct` / 100. A cancelled
   `construct` leaves the site in place (no refund; a later task adds razing).
9. **Hash.** Include every new piece of state (resources, techs, age,
   faction, build progress, payments, remainders).
10. Keep the existing `rom run` command and its output format working.

## Tests to add (`rom-sim`, real schema + seed via `rom-rules`)

- `train_deducts_cost_and_spawns_after_build_ticks` (city trains a citizen:
  food −20 at submit; unit exists exactly `build_ticks` ticks after the order became active).
- `train_cost_ramps_per_type` (second citizen costs 21 with one alive, per the seed's ramp of 1).
- `ramp_group_is_shared` (spearman price rises when a medic exists — both in group `barracks`).
- `insufficient_resources_rejected_and_nothing_deducted`.
- `queue_runs_in_order_and_respects_queue_limit`.
- `pop_cap_blocks_until_room`: fill the cap, the next unit stays `Blocked("pop_cap")`,
  despawn one unit, it appears on a later tick.
- `stop_refunds_queued_training`.
- `construct_creates_site_then_completes`: a citizen builds a farm; site exists
  the tick after submit with reduced hit points; completes after walking +
  `build_ticks`; the completion bonus (20 food) is granted once.
- `two_builders_finish_sooner_than_one`.
- `construct_rejects_overlap`, `…_unbuildable_terrain`, `…_outside_map`.
- `incomplete_building_cannot_produce`.
- `locked_item_rejected`: training a priest before `tech.religion` fails with the unlock error.
- `research_sets_age`: build a library, research `tech.age.classical` → the
  player's age is `age.classical`; a temple (needs that age) can then be constructed.
- `research_twice_rejected`; `exclusive_group_rejected` (hand-written seed rows).
- `upgrade_type_converts_existing_units_keeping_hp_pct` (hand-written seed rows).
- `abundance_age_makes_training_free_but_not_instant`: with
  `tech.age.abundance` granted, `price` is all zeros and the unit still takes `build_ticks`.
- `per_player_limit_enforced` (hand-written seed rows).
- `economy_scenario_is_deterministic`: one city, five citizens, starting
  resources; build farms, a library and a barracks, research the classical
  age, train spearmen — run twice, identical hash, and the player ends in the classical age.

Use `rules.id_of("…")` in **tests only** to find definitions.

## Rules that matter most

- Integers only; no `HashMap`/`HashSet`; iteration in id order; no clocks, threads or `rand`.
- **No branching on definition keys or names** in non-test code. Stat names
  (`pop_cap`, `research_time`), constant names and order-kind keys are vocabulary and are fine.
- Do not change the schema, the seed, `rom-rules`, `rom-render`, `rom-app`, `rom-cli`
  (beyond keeping it compiling). If `rom-rules` lacks an accessor, **list it
  and work around it**; if the schema lacks something structural, stop and report.
- Split the code: new modules `economy`, `production`, `construction`, `research`. Do not grow `world.rs` past ~500 lines.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 1200 --units 1000
```

## Out of scope

Gathering and income, commerce cap, idle workers, rally points, territory
and placement rules beyond terrain and overlap, repair of finished buildings,
razing, combat, path search.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands run with their real output, files changed, accessors you
missed in `rom-rules`, deviations from this card and why, and anything you could not verify.
