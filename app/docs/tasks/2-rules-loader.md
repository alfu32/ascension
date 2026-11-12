# Task 2 — Typed rules loader, validation and the modifier engine

Crate: `rom-rules` only (and `rom-cli` for one subcommand).

## Read first

- `AGENTS.md` — all of it (hard rules 1, 2, 4 and 5 matter most).
- `app/docs/database.md` — all of it, especially "Every bonus is a modifier".
- `app/db/schema.sql` — PART A (rules) in full. Ignore PART B (state).
- `app/db/seed.sql` — to see real rows.
- Existing code: `app/crates/rom-rules/src/lib.rs` (`build_rules_db`).

## Build

1. **Typed model.** A `Rules` struct that holds every PART A table as plain
   Rust data, loaded once from a `rusqlite::Connection`:
   `Rules::load(&Connection) -> Result<Rules, RulesError>`.
   - A `DefId(u32)` newtype. Definitions stored in a `Vec` indexed by id, with
     `rules.def(id)`, `rules.id_of("unit.citizen") -> Option<DefId>` and
     `rules.key(id) -> &str`.
   - One struct per subtype table (`Age`, `Faction`, `Resource`, `Terrain`,
     `EntityType`, `Tech`, `Ability`, `Formation`, `Stance`, `Material`,
     `Mesh`, `MapStyle`, `Victory`), reachable by `DefId`.
   - Components as `Option<…>` (or `Vec<…>` for weapons, harvest rows) on
     `EntityType`: movement, weapons (+ target layers), builder, harvester,
     gather site, node, healer, trader, container, producer, territory,
     energy, supply, placement rules, completion bonus.
   - Cross-cutting tables with lookups by the id they hang off: `cost`,
     `prerequisite`, `production` (by producer **and** by product),
     `availability`, tags (`has_tag(def, tag)`), `tech_effect` (by tech),
     `modifier` (by source), `faction_transition`, `resource_yield`,
     `damage_matrix` (`damage_pct(damage_class, armor_class)`, default 100),
     `movement_class_terrain`, `order_kind`, `game_constant`
     (`constant("sim.ticks_per_second")`), `stat`, map style features, match
     options, and the `ai_*` tables.
   - SQL `CHECK` enumerations become Rust enums (`EntityClass`, `TechKind`,
     `ModifierOp`, `AppliesTo`, `Condition`, `Layer`, …). An unknown text
     value is a load error naming the table, row and value.
   - Plain data only: no `rusqlite` types in the public structs, everything
     `Debug + Clone`. All containers iterate in id order (`Vec` or `BTreeMap`,
     never `HashMap` in anything that is iterated).
   - Split into modules (`model`, `load`, `validate`, `modifiers`); keep
     `build_rules_db` and `write_rules_db` working as they are.
2. **Validation** — `rules.validate() -> Vec<Problem>` (empty = valid), each
   problem naming the keys involved:
   - prerequisite cycles (A needs B needs A, any length);
   - `upgrades_from_id` chains that loop;
   - a `production` row whose `mode` does not fit the product
     (`research` ↔ tech; `train`/`construct`/`morph` ↔ entity type) or whose
     producer lacks the needed component (`et_producer` for train/research,
     `et_builder` for construct);
   - a `modifier` whose `stat_key` scope is `entity` but that names neither a
     tag nor a def **and** whose stat is not meaningful for all entities — keep
     this one simple: report only `player`-scope stats that carry a target tag
     or target def (a contradiction);
   - a `tech_effect` of kind `set_age` / `unlock` / `change_faction` /
     `upgrade_type` whose target is missing or of the wrong def kind;
   - an entity type with a weapon that has no `et_weapon_target` row;
   - a `faction_transition` whose `trigger_tech_id` is not a tech.
3. **Modifier engine.**
   - `ActiveSources`: an ordered set of `DefId` a player currently "has"
     (researched techs, current age, faction, claimed rare resources, completed
     unique buildings).
   - `rules.entity_stat(stat_key, entity_type, base_value, &ActiveSources, &Context) -> i64`
     and `rules.player_stat(stat_key, base_value, &ActiveSources) -> i64`.
   - `Context` carries the facts conditions need: `in_own_territory`,
     `in_enemy_territory`, `garrisoned`, `near_supply`, `entrenched`,
     `on_high_ground` (all `bool`). A modifier with a condition applies only
     when its flag is true.
   - Selection: a modifier applies to an entity type when its `target_def_id`
     equals the type, or the type has `target_tag_id`, or both are NULL.
   - **Order of evaluation**, integer arithmetic only:
     1. if any `set` applies, the value becomes the largest `set` value;
     2. add every `add`;
     3. multiply by `(100 + Σ pct) / 100`;
     4. multiply by each `mul / 100` in turn.
     Round toward zero after step 3 and after each step-4 multiplication.
   - `stack_group`: among applying modifiers that share a non-empty stack
     group **and** the same stat and op, only the one with the largest
     absolute value counts.
   - Ignore modifiers with `aura_radius` or `duration_ticks` set, and those
     whose `applies_to` is not `owner` — they need simulation state and are
     out of scope; expose them through `rules.modifiers_from(source)` untouched.
   - **Costs:** `rules.cost_of(def, &ActiveSources) -> Vec<(DefId /*resource*/, i64)>`
     applies stat `cost.all` and then `cost.<resource suffix>` (the part of the
     resource key after `res.`) to each base amount, never below zero.
     `rules.build_ticks(entity_type, &ActiveSources)` applies stat `build_time`.
4. **Availability and unlocks.**
   - `rules.is_available(def, faction)`: no `availability` rows → true;
     otherwise true if the faction or any ancestor (`parent_id` chain) is listed.
   - `rules.is_unlocked(def, &ActiveSources, &Owned)`: every `alt_group` of
     its prerequisites has at least one requirement satisfied — a tech or age
     in `ActiveSources`, or an entity type present in `Owned` (a set of entity
     types the player has at least one of). For an **age** requirement, any
     age with an equal or higher `ordinal` in `ActiveSources` satisfies it.
5. **CLI.** `rom check-rules --db <path>` loads a built `rules.db`, runs
   `validate`, prints each problem on its own line and exits 1 if there are
   any; on success prints `rules ok: <N> definitions, <M> modifiers`.

## Tests to add (all in `rom-rules`, using the real schema and seed via `build_rules_db`)

- `loads_seed`: `Rules::load` succeeds; `id_of("unit.citizen")` is `Some`;
  the citizen has movement, builder and a harvest row; the temple is a producer.
- `seed_validates`: `validate()` is empty for the seed.
- `abundance_makes_everything_free`: with `tech.age.abundance` active,
  `cost_of("unit.spearman")` is all zeros and `build_ticks` is unchanged.
- `religion_extends_temple_border`: with `tech.religion` active,
  `entity_stat("border_radius", temple, 2000, …)` is 4000; for the barracks it is unchanged.
- `religion_pct_on_city_hp`: with `tech.religion`, `entity_stat("hp", city, 1200, …)` is 1500.
- `relics_speed_research`: with `rare.relics`, `player_stat("research_time", 1000, …)` is 670.
- `op_order_set_add_pct_mul`: built from an in-memory database made of the
  schema plus a tiny hand-written seed in the test, with one modifier of each
  op on one stat — assert the exact integer result.
- `stack_group_keeps_strongest`: two `pct` modifiers in one stack group → only the larger applies.
- `condition_requires_context_flag`.
- `priest_needs_religion`: `is_unlocked(priest)` false without `tech.religion`, true with it.
- `age_requirement_met_by_later_age`: the temple (needs `age.classical`) is unlocked when only `age.medieval` is active.
- `availability_inherits_from_parent_faction` (hand-written seed).
- `prerequisite_cycle_is_reported`, `production_mode_mismatch_is_reported`,
  `weapon_without_target_is_reported` (each: seed + a few bad rows → the matching problem, naming a key).
- `unknown_enum_value_is_a_load_error`.

## Rules that matter most

- Integers only. No floats anywhere in this crate.
- No branching on a definition's key or name in non-test code. (The cost
  stat name derived from a resource key suffix is the one sanctioned use of key text.)
- Do not change `app/db/schema.sql` or `app/db/seed.sql`. If loading shows
  the schema is wrong or ambiguous, **stop and report it** in your final message.
- Do not touch `rom-render`, `rom-app`, the root `Cargo.toml` patches, or `research/`.
- New dependencies only if truly needed, exact reasoning in the final message.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run -p rom-cli --locked -- check-rules --db target/rules.db
```

The last command prints a line starting with `rules ok: 100 definitions`.

## Out of scope

Simulation state, auras, timed effects, ramping cost counts (needs unit
counts from the simulation), anything in PART B of the schema.

## Finish

Do **not** run `git commit` (the sandbox cannot write the repository
metadata); leave the changes in the working tree. Final message: commands run
and their outcome, files changed, any schema ambiguity you found, and
anything you could not verify.
