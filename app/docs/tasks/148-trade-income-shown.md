# Task 148 — Trade income counts in the resource counters

Owner (chat, 2026-11-23), verbatim: "wealth revenue from the number of caravans and merchants is
not accounted for in the resource counters, it should."

Crates: `rom-sim` (the income rates: `src/income.rs`, and the trade payments of caravans,
merchants and merchant fleets), `rom-app` (the top bar's `+N` per resource and the wealth card in
`src/hud.rs`). Implementer: Claude agent (Sonnet).

## What exists

`World::income(player, resource)` (`rom-sim/src/income.rs`) is the rate the top bar shows: worker
output, complete sites' unattended output and kept-node income, per income period, after
modifiers and the commerce cap. Payments from trade (caravans and merchants on routes between
cities, merchant fleets, interstellar trade) arrive as lump sums when a trip ends and are not in
that rate, so the wealth counter shows "+0" or too little while traders earn.

## What changes

- The rate includes trade: for each own trader on a route, its expected payment per income
  period (payment per round trip ÷ round-trip time, from the same numbers the trade code pays
  with: distance, speed, the route's cities, modifiers), added to the resource it pays in.
  Deterministic, integer, derived from current state (no new saved state; if that is not
  possible, say why and propose).
- The top bar's `+N` and the wealth card show it; the tooltip of the counter may split "from
  gathering" and "from trade" if the HUD already has such a tooltip.
- Payments themselves do not change (the playthrough hash must stay the same unless the rate
  feeds a decision: if the computer players read `income`, say what changes for them).

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main), with a
  test: a player with N caravans on a route shows a wealth rate that matches the wealth actually
  received over a long run within a small margin. Fight matrix `unanswered=0`; duel 20–0;
  hashes reported (unchanged expected, or why not).
- A screenshot you have Read (`app/docs/img/148-*.png`) of the top bar with trade income shown.
- Report: files changed, the formula, the test numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` (trade, income), `app/crates/rom-sim/src/income.rs`,
`app/crates/rom-sim/src/economy.rs`, `app/crates/rom-sim/src/wealth.rs`, `app/crates/rom-app/src/hud.rs`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t148/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/148-trade-income` on branch `task/148-trade-income`; commit
there (stage by name); do not merge, tag or push.
