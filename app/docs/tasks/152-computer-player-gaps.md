# Task 152 — Computer player: rebuild what it lost, build energy, call up militia

Gaps found while merging tasks 140, 141 and 146 (orchestrator, 2026-11-24):

- Task 140: an enemy bombardment destroyed the computer player's spaceport while its first
  cruiser was in training, and **it never rebuilt the spaceport** (the test
  `the_warship_class_follows_the_enemy_ships_seen` in `rom-ai/tests/space.rs` was loosened to
  count warships in training).
- Task 146: **it never builds energy sites** (Nuclear Reactor from Automation, Fusion Plant from
  Fusion: fixed income, no workers; `res.energy`), so its energy comes only from its start
  stock, and everything that costs energy (starships: 100 energy each) stalls.
- Task 141: **it never calls up militia**; `app/docs/computer-player.md`, section "Militia
  (task 141)", already describes what calling up properly needs.

Crate: `rom-ai` only, plus its tests and `app/docs/computer-player.md`. No change to rom-sim,
rom-app, the seed or the schema: if one is needed, stop and report it. Implementer: Claude (Opus).
It runs beside task 151 (rom-render, rom-app), so do not touch those crates.

## What changes

1. **Rebuild what was lost.** A building the player's plan needs that it had and lost (any
   producer, the spaceport, a library, a resource building, a wonder-like unique) is put back
   in the build plan, at the place it stood if free, else the nearest legal place; it competes
   with the other wishes by the same scores, but a producer whose products are wanted right now
   (the spaceport when ships are wished) ranks as if it were the wish itself. Revert the
   loosening in `the_warship_class_follows_the_enemy_ships_seen` if the original assertion
   (finished warships) holds again; say which.
2. **Energy.** From the age that unlocks the first energy building (by data: a building with a
   fixed `res.energy` income, not by key), the economy wants energy income proportional to what
   its plans will spend on energy (starships, energy-costing research and units), at least one
   such building per city once unlocked; the reactor's upgrade to the fusion plant needs nothing
   (the sim does it).
3. **Militia.** When an own city is under attack (the alert the sim raises, or enemy units
   inside its border radius that outnumber the defenders nearby), call up the workers within the
   city's radius in proportion to the threat (not all: keep the economy alive; say the rule),
   keep them at the city (the army manager must not take them away), and stand them down when
   no enemy has been in the border for a while (say the ticks). Do not train replacement workers
   while militia are out.

All decisions deterministic (integers, id order), through orders only.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests: a lost spaceport is rebuilt, energy buildings appear in an
  Automation-age start, militia called up and stood down in a raid scenario; each scenario run
  twice with equal hashes); fight matrix `unanswered=0`; **duel 20–0** (`balanced:moderate` beats
  `balanced:easiest` on seeds 1..20) and report mean_tick; database loads with lint views empty;
  playthrough and ascension hashes reported with why they moved.
- Also run `rom duel --profiles balanced:moderate,balanced:moderate --seeds 1..20 --jobs 2` and
  report it (no stalls, matches end).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/computer-player.md` (all, especially the
Militia section), `app/crates/rom-ai/src/` (the economy, build plan, fleet and army managers),
`app/crates/rom-ai/tests/space.rs`, `app/docs/content.md` (energy rows), `research/05-computer-opponents.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Another agent builds at the
same time: keep `--jobs 2` for duels. Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes
you did not start (the owner may be playing). Logs only in the scratchpad's `t152/`; never write
`scratchpad/gate*.sh`; no cargo target dir in the scratchpad or /tmp. Work in
`../rise-of-man-worktrees/152-computer-player-gaps` on branch `task/152-computer-player-gaps`;
commit there (stage by name); do not merge, tag or push.
