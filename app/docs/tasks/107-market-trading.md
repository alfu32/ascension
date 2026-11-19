# Task 107 — Market trading: buy and sell resources at the market

Crates: `rom-sim` (orders, prices), `rom-rules` (loading), schema/data (`generate.py`), `rom-app`
and `rom-render` (the market panel), `rom-ai` (computer players use it). Implementer: Claude
(Opus). Owner 2026-11-19: "ok, lets do that".

## Why

The market exists as a building (`bld.market`, ancient age) and the data already has
`resource.tradeable` and `resource.market_base_price` and an `enable_rule` key `market_trading`,
but no buying or selling is implemented. The reference game (Reference A) lets a player with a
market trade resources for wealth at prices that rise when bought and fall when sold, recovering
over time; we take that idea, in our own numbers.

## Build

1. **Orders.** `buy` and `sell` (a resource, an amount in lots of 100), submitted by a player who
   owns a completed market and has the rule enabled; paid in wealth (`res.wealth` is the currency;
   wealth itself is not traded). One way in: these are orders, validated like the rest.
2. **Prices.** Per player or shared per match (decide, and say why: shared prices make one
   player's selling cheapen everyone's; per player is simpler and fairer in single player).
   Integer prices from `market_base_price`; each lot bought raises the price by a data-driven step,
   each lot sold lowers it; prices drift back toward base over time; a spread (selling pays less
   than buying costs) so buying and selling back loses wealth. All constants in data (rule
   constants), technologies may improve them through `modifier` rows (e.g. a later-age tech that
   narrows the spread), deterministic, hashed, saved.
3. **Which resources.** Those with `tradeable = 1` in the data; check the seed's values make sense
   (food, timber, metal, oil, knowledge?; not wealth) and set them in `generate.py`.
4. **Panel.** Selecting a market shows buy and sell buttons per resource with the current price;
   a click submits the order; prices update live; the help line explains the spread.
5. **Computer players** sell surplus and buy shortfalls through the same orders (a simple rule:
   keep a reserve, trade above a threshold), data-driven weights in `ai_weight_key`, never cheating.
6. **Save**: prices and their drift state are saved (check whether the save layout changes; if so,
   bump the save format and say so; older saves are refused as usual).

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` (resources, enable_rule, orders,
modifiers), `research/01-reference-data-model.md` (what Reference A does with markets),
`research/05-computer-opponents.md`, `rom-sim` orders and economy code, `rom-ai` economy,
the HUD's building panel in `rom-render`/`rom-app`.

## Done when

- Gate green from `app/`; new hashes reported if the playthrough changes (it should not unless the
  playthrough player trades; say which).
- Fight matrix `unanswered=0`; both duel tables before and after (computer players now trade).
- Tests: buy and sell change stock and wealth by the quoted prices; prices step and drift back;
  the spread makes a round trip lose; refused without a market or without wealth; run twice, same
  hash; save and load keep prices.
- Screenshots you looked at (`app/docs/img/107-*.png`): the market panel with prices, before and
  after a few sales.

Memory rules: `CARGO_BUILD_JOBS=4`; duels `--jobs 2`; check `free -g`; run the app one instance
at a time under `systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 env -u
WAYLAND_DISPLAY …` in self-ending modes. Never `xdotool`, `wmctrl` or `sudo`; do not stop
processes you did not start. Wait loops must not use a `pgrep -f` pattern that matches the loop
itself. Logs only in your own scratchpad subfolder; never write `scratchpad/gate*.sh`.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the price rules chosen, balance before/after, deviations, anything not
verified.
