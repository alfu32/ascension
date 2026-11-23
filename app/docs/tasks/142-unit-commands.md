# Task 142 — Commands: delete, attack ground, and the spaceport's rally point

Owner (chat, 2026-11-23), verbatim: "i want to be able to delete units and buildings. deleting an unconstructed building will
return the resources back proportionally with the construction progress." (The owner chose: the
**unspent part**, refund = cost × (1 − progress); finished buildings and units refund nothing.)
"i want the ranged units to have the command attack ground (all units that throw projectiles)."
"spaceport rally point should function just like the rally points of the other unit producing
buildings (ships that are exiting the production queue will rally at the rally point)."

Crates: `rom-sim` (orders), `rom-app` (buttons, hotkeys, the confirmation for delete, the
ground-target cursor), `rom-render` only for the ground-target marker. Implementer: Claude (Opus).

## What changes

1. **Delete** (order `delete`, the Delete key and a command-card button): own units and
   buildings, many at once; a confirmation only for a finished building (one click "Delete?
   yes/no" in the HUD, no OS dialog). A construction site refunds cost × (1 − progress), rounded
   down, per resource; anything else refunds nothing. A deleted building with units inside
   (garrison, transport, production queue) releases them and refunds its queue as cancelling
   does now. No death animation reward: the enemy gets no score or kill for it.
2. **Attack ground** (order `attack_ground`, a command-card button and hotkey, then a click on
   the ground; Shift queues): every unit whose weapon throws a projectile (data: the weapon's
   projectile kind, not melee and not instant beams if those cannot hit ground — say what you
   chose) fires at the point within its range, moving into range first, until stopped or given
   another order; splash damage applies to whatever is there, own units included if splash
   already hurts them. Ships and aircraft with such weapons too, if their weapons can hit the
   ground layer.
3. **Spaceport rally point**: ships leaving a spaceport's queue move to its rally point as units
   leaving a barracks do (orbit-bound ships stay in orbit above it; say what space ships do with
   a rally point on the ground). Find why it does not work today and fix it.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests; every simulation change adds a scenario run twice with equal
  hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new
  playthrough and ascension hashes and rules fingerprint reported with why they moved (or that
  they did not).
- Screenshots you have Read, saved as `app/docs/img/142-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/crates/rom-sim/src/order.rs`, `app/crates/rom-sim/src/rally.rs`, `app/crates/rom-sim/src/construction.rs`, `app/crates/rom-sim/src/combat.rs`, `app/docs/hud.md` (command card).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t142/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/142-unit-commands` on branch `task/142-unit-commands`; commit there
(stage by name); do not merge, tag or push.
