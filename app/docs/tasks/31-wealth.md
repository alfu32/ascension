# Task 31 — Wealth: relics kept in temples, trade routes between cities

Crates: `rom-rules`, `rom-sim`, `rom-cli`. Data: `app/db/schema.sql`,
`app/db/seed.sql` (hand-written part), `app/db/content/generate.py` (then
regenerate the seed's generated block). Docs: `app/docs/database.md`,
`app/docs/content.md` §3.

## Owner's report (2026-11-13, after playing)

> The player cannot collect wealth. Wealth should be collected either by
> priests taking possession of relics and bringing them to the temple (the
> relics generate a steady amount of wealth), or by merchants or caravans that
> travel between cities, bringing a certain amount of wealth on every trip.

Today nothing produces wealth: `et_trader` rows exist (`unit.merchant`,
`claims_nodes = 1`) but no simulation code reads them; `node.relics` has
`claim_mode = 'trader'` and income 0; there is no caravan unit. Most building
upgrades cost wealth, so they cannot be researched.

## Read first

- `AGENTS.md` — all of it (determinism, data-not-names, one way in, units,
  schema rules, scope).
- `app/docs/database.md`; `app/db/schema.sql` (`et_node`, `et_trader`,
  `et_container`, `entity.container_id`, `trade_route`, `order_kind`, `stat`,
  `game_constant`); `app/db/seed.sql` (constants, order kinds, tags, the
  hand-written `node.relics` rows, the relic `map_style_feature` row).
- `app/docs/content.md` §1, §3, §5, §7; `app/db/content/generate.py` (`LINES`,
  per-line stats, `et_trader` row, `BUILDINGS`).
- `app/crates/rom-rules/src/` — `model.rs` (`Node`, `ClaimMode`, `Trader`),
  `load.rs`, validation.
- `app/crates/rom-sim/src/` — `world.rs` (order submission, `has_component`,
  `SimError`), `order.rs`, `gathering.rs` (the closest existing pattern: a
  unit walking between a site and a drop-off, re-ordering itself), `income.rs`
  (income period, incremental economy terms), `movement.rs`, `pathing.rs`,
  `geometry.rs` (distances on flat and sphere), `capture.rs`, `hash.rs`,
  `scenario.rs`, `space.rs` (presence claims — another claim mode, for contrast).
- `app/crates/rom-cli/src/` — `main.rs`, `siege_cmd.rs` or `territory_cmd.rs`
  as the pattern for a scenario command.

## Build

### A. Relics (carry mode)

1. **Schema.** `et_node.claim_mode` gains `'carry'`: a unit whose type carries
   `claim_tag_id` may pick the node up and bring it to a building that keeps
   it. New table:

   ```sql
   CREATE TABLE et_keeper (               -- keeps carried nodes (temple ← relics)
     entity_type_id INTEGER PRIMARY KEY REFERENCES entity_type(id),
     capacity       INTEGER NOT NULL      -- nodes kept at once
   ) STRICT;
   ```

   New order kind `('carry','movement','entity','target = carry node; brought to the nearest own keeper')`.
   New tag `clergy` (role). Note every schema change in `database.md`.
2. **Data.** `node.relics`: `claim_mode 'carry'`, `claim_tag_id` = `clergy`,
   `resource_id` = `res.wealth`, `income` 20 (per income period, per relic kept).
   The priest line gets the `clergy` tag (generator). `bld.temple` and its
   upgraded variants get `et_keeper` capacity 5 (generator). The unobtainable
   rare resource `rare.relics` and its rows (resource, yield, research-time
   modifier) are removed; keep both lint views empty.
3. **Simulation.**
   - `carry` order: the unit walks to the node (path search as for any
     move), picks it up on arrival (the node leaves the map: it is now inside
     the unit, `container_id`), then walks to the nearest own completed keeper
     with free capacity (by path or straight distance; ties by entity id) and
     deposits it (`container_id` = keeper). If there is none, the unit waits
     holding the relic (order `Blocked("no keeper")`) and retries each income
     period. One relic per unit.
   - A keeper pays its owner `income` of the node's `resource_id` per income
     period for every node it keeps, through the existing economy terms
     (so `income.wealth` modifiers and the commerce cap apply as for any income,
     and the incremental economy stays equal to a full recount).
   - A carrier that dies drops the relic on the nearest free tile where it
     stood. A keeper that is destroyed drops its relics around it; a keeper
     that is captured keeps them for the new owner.
   - Only units whose type has the node's `claim_tag_id` may carry; others are
     rejected (`SimError` variant, typed). Enemy-kept relics can be taken only
     by capturing or destroying the keeper.
   - Hash covers carried and kept nodes.

### B. Trade routes

1. **Schema/data.** Constant `player.caravan_limit.base` = 2 (stat
   `caravan_limit` already exists; Commerce levels add +1 each). Constant
   `trade.route_unit` = 10000 milli-tiles. Constant `trade.min_route` =
   the value of `city.spacing` (20000). `et_trader.route_income` means:
   wealth paid on each arrival per `trade.route_unit` of route length.
   - `unit.merchant`: `runs_routes 1`, `route_resource_id res.wealth`,
     `route_income 3`; keep `claims_nodes` 0 (no trader-claim nodes remain).
   - New line **caravan** at the Market (generator `LINES`):
     `caravan:Caravans:1|freight_wagon:Freight Wagons:4|lorry:Lorries:6|cargo_drone:Cargo Drones:8`,
     civilian, unarmed, `runs_routes 1`, `route_income` 6 / 9 / 12 / 15,
     hp and speed rising with the step, cost in food and timber (later metal/oil),
     `pop_cost` 1. Upgrade techs as for every other line (at the Market).
     Requirement: `tech.commerce.2` (Coinage) and its age.
2. **Simulation.** `trade` order (`order_kind` already has it; target = a city):
   - The home city is the own completed city nearest to the trader when the
     order is accepted (ties by id); the target is an own or allied completed
     city on the same body other than home, at least `trade.min_route` away.
     Reject otherwise with typed errors (`NoHomeCity`, `RouteTooShort`,
     `NotATradePartner`).
   - At most `caravan_limit` traders of a player run routes at once; the
     next one is rejected (`CaravanLimit`).
   - The trader walks home → target → home … On each arrival (within
     `build.reach` of the city footprint) the owner receives
     `route_income × length / trade.route_unit` of `route_resource_id`
     (integer division, length = the geometry module's distance between the
     two city centres, fixed at acceptance, flat and sphere), modified by the
     `income.wealth` stat; **not** capped by the commerce cap (it is a delivery,
     not a rate). Allied target: the partner receives the same amount.
   - The route ends (unit idles, `Failed` with a reason) when either city is
     destroyed, captured by a non-ally, or a new order replaces it.
   - Routes are part of the state hash and visible to the app through a
     read-only query (`World::trade_routes(player)` → trader, home, target,
     length, per-trip income) and the kept relics (`World::kept_nodes(entity)`).
3. Nothing generated during a match; iteration in id order; integers only.

### C. Content and checks

- `content.md` §3: Wealth row → "Temple (relics carried by priests), Market
  (merchants and caravans on trade routes)"; describe both in a short paragraph.
- `rom playthrough` still reaches `age.galactic` (it may use the new orders
  to earn wealth; report the tick).
- New CLI scenario `rom wealth --db <rules.db> --seed <n> [--sphere]`: one
  player with two cities 30 tiles apart, a temple, a market, two priests near
  two relics, one merchant and one caravan; runs 6,000 ticks; prints
  `wealth relics_kept=… routes=… trips=… wealth_gained=… hash=…`. Identical on
  repeat; works on the sphere.

## Tests to add

- `rom-rules`: loads `et_keeper` and the `carry` claim mode; validation rejects a
  `carry` node without `claim_tag_id` and a keeper with capacity < 1.
- `rom-sim` (beside the code): `priest_carries_relic_to_temple`,
  `kept_relic_pays_every_period`, `non_clergy_cannot_carry`,
  `dead_carrier_drops_relic`, `destroyed_temple_drops_relics`,
  `captured_temple_keeps_relics_for_captor`, `keeper_capacity_is_respected`,
  `trade_pays_on_each_arrival_scaled_by_length`, `route_too_short_rejected`,
  `caravan_limit_rejected`, `route_ends_when_city_lost`,
  `incremental_economy_matches_full_recount` still passes with kept relics.
- Scenario test under `tests/`: the wealth scenario twice → same hash, on flat
  and sphere.

## Done when (from `app/`)

```
python3 db/content/generate.py --check
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run --release -p rom-cli -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli -- check-rules --db target/rules.db
cargo run --release -p rom-cli -- playthrough --db target/rules.db --seed 1
cargo run --release -p rom-cli -- wealth --db target/rules.db --seed 1
cargo run --release -p rom-cli -- wealth --db target/rules.db --seed 1 --sphere
```

and the database lint (`AGENTS.md` → Commands) prints nothing.

## Rules that matter most

- Data, not names: no branching on `node.relics`, `bld.temple`, `unit.caravan`;
  behaviour comes from `et_node.claim_mode`, `claim_tag_id`, `et_keeper`,
  `et_trader`.
- Determinism (integers, id order, no clocks); one way in (orders only).
- Do not modify `rom-render`, `rom-app`, `rom-ai`, `rom-mapgen`, `rom-save`
  (the app hook-up is the next task). If `rom-mapgen` must change to place
  relics as `carry` nodes, stop and report instead.
- Keep the nine tests that pin the original city geometry (`PINNED_GEOMETRY`)
  as they are.

## Finish

Do not commit (the orchestrator commits after review). Final message: commands
with real output, files changed, schema changes, deviations and why, anything
you could not verify.
