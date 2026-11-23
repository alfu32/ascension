# Task 143 — Formations for troops and ships: square, compact, circle, line, double line

Owner (chat, 2026-11-23), verbatim: "i have an issue with big ship formations, they are disposed in a circle which sometimes
is not convenient, lets have more ships and troops formations: square, compact, circle, line,
double line etc."

Crates: `rom-sim` (formation slots for ground units and for ships in system space and orbit),
`rom-app` (the formation buttons, hotkeys, the selection's current formation), seed (the
`formation` rows; schema only if a shape is missing from its CHECK: authorised here).
Implementer: Claude (Opus).

## What exists

`formation` rows (`form.line`, `form.column`, `form.wedge`, `form.square`, …) and
`entity_type.formation_spacing`; find how ground groups and space fleets are laid out today
(the fleet "circle" the owner sees) and where a formation is chosen.

## What changes

- The player picks a formation for a selection from the command card (and hotkeys): **square**
  (as square as the group allows), **compact** (tight block, minimum spacing), **circle**
  (ring, the current fleet layout), **line** (one rank), **double line** (two ranks), plus the
  existing ones that already work. Ground units and ships alike; ships use the same shapes in
  their plane.
- The default for a big fleet is no longer the circle: use square (say why if you choose
  otherwise).
- A moving group keeps its formation and faces the direction of travel; ranged and support
  units behind melee where `ranged_behind` says so; the slowest sets the pace as now.
- The formation chosen is part of the order (deterministic, saved and hashed).

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests; every simulation change adds a scenario run twice with equal
  hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new
  playthrough and ascension hashes and rules fingerprint reported with why they moved (or that
  they did not).
- Screenshots you have Read, saved as `app/docs/img/143-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/crates/rom-sim/src/movement.rs`, `app/crates/rom-sim/src/space.rs`, `app/crates/rom-sim/src/order.rs`, `app/docs/hud.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t143/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/143-formations` on branch `task/143-formations`; commit there
(stage by name); do not merge, tag or push.
