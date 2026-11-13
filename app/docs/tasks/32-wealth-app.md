# Task 32 — Wealth in the app: carry and trade orders, relics and routes on screen

Crates: `rom-app`, `rom-render`. No simulation, rules, schema or seed changes.

## Context

Task 31 added wealth to the simulation (`app/docs/tasks/31-wealth.md`):
priests carry relic nodes (`et_node.claim_mode = carry`, `claim_tag_id`) to
keepers (`et_keeper`, the temple), which pay wealth every income period; units
with `et_trader.runs_routes` run `trade` orders between the nearest own city
and an own or allied city, paid on each arrival. The player cannot use any of
it yet: there is no way to issue `carry` or `trade` from the app and nothing
is drawn.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/hud.md`; `app/docs/content.md` §3 (wealth); `app/docs/tasks/31-wealth.md`.
- `app/crates/rom-sim/src/wealth.rs` (`World::kept_nodes`, `World::trade_routes`,
  `TradeRoute`), `lib.rs` (`SimError`: `InvalidCarryTarget`,
  `CannotCarry`, `CarrierFull`, `NoHomeCity`, `RouteTooShort`,
  `NotATradePartner`, `CaravanLimit`), `order.rs` (`OrderStatus::Blocked("no keeper")`,
  `Failed("city_lost")`), `income.rs`.
- `app/crates/rom-rules/src/model.rs` (`Node`, `ClaimMode`, `Keeper`, `Trader`), `lib.rs` (`has_tag`).
- `app/crates/rom-app/src/selection.rs` (`right_click`, `rejection_message`,
  `palette`, hover help), `sim_thread.rs` (entity and HUD snapshots), `script.rs`, `hud.rs`.
- `app/crates/rom-render/src/entities.rs` (`EntityView`, node drawing),
  `hud_model.rs`, `hud.rs` (selection panel, data view), `overlay.rs`, `worker.rs`.

## Build

1. **Right-click orders** (data-driven, like `harvest`): with own units selected,
   right-click on
   - a `carry` node not inside anything → each selected unit whose type has the
     node's `claim_tag_id` gets `carry` (others move as today);
   - an own or allied completed city → each selected unit with
     `trader.runs_routes` gets `trade` with that city as target (others move).
   Shift queues, as for other orders. The order marker shows as for move.
2. **Hover help** (the help line): over a carry node with a carrier selected —
   "Carry to a temple" built from the keeper type's display name (first type
   with `et_keeper` in id order; no key literals); over a city with a trader
   selected — "Trade route" plus, once a route exists, its per-trip income.
3. **Rejections** in the event line, in the player's words: `CannotCarry` "This
   unit cannot carry that", `CarrierFull` "Already carrying", `NoHomeCity` "No
   home city on this world", `RouteTooShort` "Too close for a trade route (N tiles)"
   (N from `trade.min_route`), `NotATradePartner` "Not a trade partner",
   `CaravanLimit` "Trade route limit reached (N)" (N = the player's `caravan_limit`).
   An order that turns `Blocked("no keeper")` → event "No temple with room for
   the relic" once; `Failed("city_lost")` → "Trade route ended: city lost".
4. **On screen.**
   - Carrying units: a small gold marker above the unit (`EntityView` gains
     `carrying: bool`).
   - Keepers: kept count as small gold pips on the building (`EntityView`
     gains `kept: u8`, `capacity: u8`).
   - Carry nodes on the ground: drawn distinctly (gold-capped plinth) so the
     player can find them; they show on the minimap as gold dots once explored.
   - A selected trader: its route as a dashed gold line on the ground from home
     to target (flat and sphere, conforming like the city circles), with a
     marker at the end it is heading to.
5. **Selection panel.** A keeper shows "Relics k / capacity" and "+W wealth per
   30 s" (its share of income). A trader shows "Route: <home> → <target>",
   "+N per trip", or "No route" when idle.
6. **ECONOMY data tab** gains a "Wealth" block: relics kept (sum over own keepers),
   trade routes running / limit, per-trip income summed over routes.
7. **Script verbs** for screenshots: entity names `relicN` (carry nodes in id
   order), `templeN`, `marketN`, `unit:<n>` (n-th own unit of the last trained
   type) — or anything equivalent, documented in `script.rs`.

## Tests to add (no window)

- `rom-app`: `right_click_relic_with_priest_issues_carry`,
  `right_click_city_with_trader_issues_trade`,
  `right_click_relic_with_soldier_moves`, `wealth_rejections_name_the_rule`,
  `keeper_panel_counts_kept_relics`, `economy_tab_sums_routes`.
- `rom-render`: `route_line_conforms_to_the_globe`, `carried_marker_sits_above_the_unit`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Write `docs/tasks/32-wealth-script.txt`: seed 1 flat; grant resources; build a
temple and a market near the city and a second city ≥ 25 tiles away (use the
data from the city-limits script for a valid spot); train a priest and a
merchant; right-click a relic with the priest and the second city with the
merchant; at the end select the merchant. The orchestrator runs:

```
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 1 --size 1672x941 --script docs/tasks/32-wealth-script.txt --screenshot target/wealth.png --after 40
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

If you cannot open a window, say so; do not skip writing the script.

## Rules that matter most

- No branching on definition keys or names: carriers from `claim_tag_id`,
  keepers from `et_keeper`, traders from `et_trader.runs_routes`.
- `rom-app` does not call raylib; `rom-render` does not depend on `rom-sim` or
  `rom-rules` — pass plain data in the snapshots.
- Do not modify `rom-sim`, `rom-rules`, `rom-cli`, the schema or the seed.
  Missing simulation API → work around it and list it.
- Never use `xdotool`, `wmctrl` or similar. `env -u WAYLAND_DISPLAY`. No `sudo`.

## Finish

Do not commit (the orchestrator commits after review). Final message: commands
with real output, files changed, missing simulation API, deviations and why,
anything you could not verify.
