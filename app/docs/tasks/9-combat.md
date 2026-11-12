# Task 9 — Combat, stances and healing

Crate: `rom-sim` only.

## Read first

- `AGENTS.md` — all of it. Hard rules 1–5.
- `research/01-reference-data-model.md` section 2 (unit record, stances) and `app/docs/database.md` (tags, damage matrix, modifiers).
- `app/db/schema.sql` — `et_weapon`, `et_weapon_target`, `damage_class`,
  `armor_class`, `damage_matrix`, `et_healer`, `et_energy`, `stance`,
  `entity_type` (`hp`, `armor`, `armor_class_id`, `los`, `radius`, `plunder_pct`),
  `game_constant`, `order_kind`.
- `app/crates/rom-sim/src/` — all of it. `app/crates/rom-rules/src/modifiers.rs` (`entity_stat`).

## Build

1. **Hostility.** Two players are enemies when their `team` values differ.
   Player 0 / team 0 owners with controller-less entities (nature) are never
   attacked automatically. (Diplomacy comes later.)
2. **Effective stats.** Use `rules.entity_stat(stat, type, base, sources, context)`
   for `hp` (maximum), `armor`, `attack` (applied to each weapon's `damage`),
   `range` (added to `range_max`), `speed`, `los`, `heal_rate`. Maximum hit
   points may change when a tech is researched: keep the current/maximum ratio.
3. **`attack` order** (executor has a weapon; target entity must be an enemy):
   - Choose the first weapon (by `slot`) that can hit the target's layer
     (`et_weapon_target`; a target's layer comes from the terrain layer of the
     tile it stands on, or `air` for a flying movement class).
   - If the target is farther than the effective maximum range (measured to
     the edge of the target's footprint or radius), walk toward it with the
     existing path following until in range; closer than `range_min` → the
     weapon cannot fire (try the next weapon; if none, step away is **not** required — just hold).
   - Units with `et_movement.packs = 1` are out of scope: treat as never packed.
   - Fire every `cooldown_ticks`. Each firing rolls `accuracy_pct −
     accuracy_falloff × distance_in_tiles` (minimum 5 %) with the simulation
     random generator; a miss deals nothing.
   - Damage on a hit = `max(combat.min_damage, damage × matrix_pct / 100 − target armor)`
     with `matrix_pct = rules.damage_pct(weapon class, target armor class)`,
     then × `shots`.
   - `projectile_speed = 0` → applied at once. Otherwise the hit is scheduled
     to land after `distance / projectile_speed` seconds (in ticks, at least
     1); it still lands if the attacker dies, and is lost if the target is
     gone (no homing onto a new target).
   - Splash: every enemy entity within `splash_radius` of the impact point
     other than the main target takes `splash_pct` percent of the computed
     damage (their own armour and matrix value applied).
   - `consumed = 1` → the attacker is despawned when it fires.
   - Target dead or gone → the order is `Done`. Buildings without movement
     attack only targets inside their range.
4. **Death.** Hit points ≤ 0 → despawn at the end of the tick (so
   simultaneous kills are possible). Workers in its slots are freed, orders
   targeting it end as they already do. If `plunder_pct > 0`, the killer's
   owner receives that percentage of the victim type's base cost. Count kills
   and losses per player in `world.stats()`.
5. **`attack_move`** (point target): walk to the point; whenever an enemy is
   within acquisition range, fight it (as an `attack` order would), then continue.
6. **Stances and automatic acquisition.** Each entity has a combat stance
   (default: the stance definition whose `behavior` is `aggressive`; look it up
   by the `behavior` text, which is vocabulary). A `set_stance` call on the
   world changes it (not an order kind). Idle entities with a weapon:
   - `aggressive`: acquire the nearest enemy within constant
     `unit.respond_range` (ties: lower id) and attack; chase without limit.
   - `defensive`: acquire within the weapon range + 3 tiles; give up and
     return to the original position if the target moves more than 6 tiles from it.
   - `stand_ground`: attack only what is already in range; never move.
   - `hold_fire`: never attack automatically.
   - `raid` and `raze`: as `aggressive`, but prefer civilians (tag `civilian`)
     / buildings respectively when several targets are available.
   Buildings with weapons behave as `stand_ground`. Acquisition runs for an
   entity at most once every 10 ticks (stagger by entity id) to bound cost.
7. **Healing** (`et_healer`): an idle healer, or one given the `heal` order on
   a target, restores `hp_per_second / ticks_per_second` hit points per tick
   (carry the remainder) to one friendly, damaged entity that has the healer's
   `target_tag_id` and is within `range`; with `auto = 1` it picks the most
   damaged such entity in range by itself (ties: lower id) but does not walk
   to it. Never heals itself. If `energy_per_hp > 0` it spends energy per hit
   point healed and stops at zero.
8. **Energy** (`et_energy`): starts at `max`, regenerates `regen_per_second`
   (carry the remainder) up to `max`.
9. **Under attack.** When a worker (has a harvester row) with no weapon able
   to answer is damaged, nothing happens in this task (fleeing comes later).
   When an entity in `aggressive`/`defensive` stance is damaged by an enemy it
   has not targeted and it is idle, it attacks that enemy.
10. **Hash** covers every new field (stances, cooldowns, pending hits, energy, remainders, stats).
11. **CLI:** `rom battle --db <rules.db> --seed <n> --ticks <t> --per-side <n>`:
    two players on different teams, each with `n` units of the first entity
    type (id order) that has both a weapon and a movement component, lined up
    30 tiles apart on a 96×96 map of crossable terrain, each side given
    `attack_move` to the other's line. Prints
    `tick=<t> alive=<a>,<b> kills=<a>,<b> hash=<16 hex>`.

## Tests to add

- `melee_unit_walks_into_range_then_hits_on_cooldown` (exact damage: spearman 9 melee vs citizen flesh armour 0).
- `damage_matrix_and_armor_apply` (seed: melee vs structure is 50 %; city armour 3) and `minimum_damage_is_one`.
- `ranged_building_attacks_enemy_in_range_only` (the city's 10-tile weapon).
- `accuracy_roll_is_deterministic_and_can_miss` (extra rows: 50 % accuracy; same seed same hits).
- `projectile_lands_after_travel_time` and `projectile_is_lost_when_target_gone` (extra rows).
- `splash_hits_neighbours_but_not_friends` (extra rows).
- `consumed_weapon_destroys_attacker` (extra rows).
- `weapon_cannot_hit_layer_without_target_row` (extra rows: a flying target).
- `death_despawns_at_end_of_tick_and_mutual_kills_happen`.
- `kill_frees_gather_slot_and_ends_orders_targeting_it`.
- `plunder_is_paid_to_the_killer` (extra rows).
- `attack_rejected_on_friend_or_same_team`.
- `aggressive_acquires_and_chases`, `defensive_returns_home`, `stand_ground_does_not_move`, `hold_fire_never_attacks`, `raid_prefers_civilians`.
- `attack_move_fights_then_continues`.
- `idle_unit_retaliates_when_hit`.
- `priest_heals_most_damaged_biological_in_range_only` and `repair_drone_heals_mechanical_not_biological`.
- `medic_spends_energy_and_stops_at_zero`; `energy_regenerates_to_max`.
- `race_modifier_changes_max_hp_keeping_ratio` (a Reptilian player's biological unit has +15 % hit points).
- `battle_is_deterministic`: the CLI scenario as a library call, 200 per side, 2,400 ticks, twice → same hash; one side wins or both are reduced.

## Rules that matter most

- Integers only; no `HashMap`/`HashSet`; no clocks, threads or `rand`; iteration in id order.
- No branching on definition keys or names in non-test code (stat names,
  constants, order-kind keys, tag keys `civilian`, stance `behavior` values and layer keys are vocabulary).
- Do not change the schema, `rom-rules`, `rom-render`, `rom-app`, `rom-mapgen`.
  You may add `game_constant` rows to `app/db/seed.sql` if a number you need
  is missing (list them). In `rom-cli` add the subcommand in a new file `src/battle_cmd.rs`.
- New modules: `combat`, `stance`, `healing`. Keep `world.rs` under ~500 lines.
- Do not run the GUI application; no window tools.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- battle --db target/rules.db --seed 3 --ticks 2400 --per-side 200
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000
```

The battle line is identical on a second run. Report the wall-clock time of the last two.

## Out of scope

Garrison and transports, capture of cities, territory and attrition, fog of
war (everything is visible to targeting for now), flanking and high ground,
abilities and conversion, pack/deploy, fleeing workers, diplomacy.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands with real output and timings, files changed, constants
added, deviations and why, anything you could not verify.
