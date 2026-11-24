# Task 153 — Computer player: armies and fleets in formation, and a firmer win

Follows task 143 (formations for players) and task 152 (orchestrator, 2026-11-24). The
computer player still sends its armies as a crowd: task 143 gave player orders a formation
(`subject` of `move`/`attack_move`/`patrol`/`orbit`/`travel` names a `formation` row; ranks
face the goal; `ranged_behind` puts ranged and support units behind; the group arrives
together), but `rom-ai` only uses it where it already sent a line. Task 152 left duel seed 14
won at tick 103,472 of 108,000: too close to the limit.

Crate: `rom-ai` only, plus its tests and `app/docs/computer-player.md`. No change to rom-sim,
rom-app, rom-render, the seed or the schema: if one is needed, stop and report it.
Implementer: Claude (Opus). It runs beside tasks that work in rom-app, rom-render and rom-sim.

## What changes

1. **Armies march and attack in formation.** The army manager picks a formation per order by
   data (formation rows and their `ranged_behind`, `rank_ratio_pct`, `spacing_pct`): a wide line
   or double line facing the enemy for the advance and the assault on a city, square for a
   group holding a place or escorting, column through narrow ground if the path search says the
   way is narrow (say how you measure it, or leave it out). Mixed armies keep ranged and support
   units behind (that is `ranged_behind`). Never pick a formation by its key: choose by shape
   and numbers in the rows.
2. **Fleets in formation.** Space fleets on `orbit`, `travel` and `move` in system space use the
   same choice (line abreast for an attack, square for holding a body).
3. **A firmer win.** Find why the moderate player needs so long on duel seed 14 (trace it; say
   what you found) and fix the cause in rom-ai if it is a computer-player weakness (an army
   that waits too long, a siege that does not commit, workers kept idle): the target is every
   seed of `balanced:moderate` vs `balanced:easiest` won well inside the limit (report the
   slowest tick), without making the easiest level stronger.

All decisions deterministic (integers, id order), through orders only.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests: an army's advance carries a formation with ranged behind, a
  fleet's travel carries one; each scenario run twice with equal hashes); fight matrix
  `unanswered=0`; **duel 20–0** (`balanced:moderate` beats `balanced:easiest` on seeds 1..20),
  report mean_tick and the slowest tick; also `balanced:moderate,balanced:moderate` seeds 1..20
  (no stalls); database loads with lint views empty; playthrough and ascension hashes reported
  (the playthrough is rom-sim's own bot: they should not move; say so).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/computer-player.md` (all, including the task 152
section), `app/docs/database.md` ("Formations a player picks (task 143)"),
`app/crates/rom-sim/src/formation.rs` (read only), `app/crates/rom-ai/src/` (army, fleet, navy,
militia), `research/05-computer-opponents.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Other agents build at the
same time: keep `--jobs 2` for duels. Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes
you did not start (the owner may be playing). Logs only in the scratchpad's `t153/`; never write
`scratchpad/gate*.sh`; no cargo target dir in the scratchpad or /tmp. Work in
`../rise-of-man-worktrees/153-computer-player-formations` on branch
`task/153-computer-player-formations`; commit there (stage by name); do not merge, tag or push.
