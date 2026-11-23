# Task 138 — The home star system can be seen from the Fusion age

Owner (chat, 2026-11-23): "the visibility over the solar system should be one or two ages
sooner". Asked, the owner chose **Fusion (age 9)**, and **own system only**: zoom out to the
star system (its planets, moons and resource bodies, seen), no travel there yet; the star map of
other systems still opens as now. This changes a decision recorded in `CLAUDE.md` (updated by
the orchestrator). Crates: app/db seed (and `generate.py` if the age rows come from it); `rom-sim`,
`rom-app`, `rom-render` or `rom-ai` only if the change shows a side effect that must be fixed
(stop and report before a larger change). Implementer: Claude agent (Sonnet). Runs beside task
137 (which changes `rom-render/src/vision.rs` and the terrain drawing): do not touch those.

## What exists

`age.map_scale` gives each age its tier (`app/docs/world-model.md`, "Camera and visibility by
age"): rank 0 planetary (camera on the home planet, `view_range`), rank 1 interplanetary (home
system, ships may move and orbit in it), rank 2 interstellar (star map, routes). Today Fusion is
planetary with `view_range` 0 (the whole globe), Starfaring the first interplanetary age, and
Expansion the first interstellar one. The code reads ranks, never age names.

## What changes

- `age.fusion` gets the interplanetary tier, so a Fusion player can zoom out to the home star
  system.
- **View only.** Before Starfaring there are no ships that could travel (the Spaceport offers
  only satellites and probes, which stay in orbit: task 130). Check every other effect of rank 1
  across the code (`map_scale` / tier rank readers in `rom-sim`, `rom-ai`, `rom-app`,
  `rom-render`, `rom-match`): system visibility and exploration, fleets and orbits, resource
  bodies, claiming by presence, the computer players' space behaviour, the "reached space"
  events and the HUD's space panels, alerts, the launcher's descriptions. Anything that would
  give a Fusion player more than the view (moving or claiming in the system, an AI expecting
  ships) must not happen: say what you found and how it stays view-only (prefer data; if code
  must check something other than the rank, keep it data-driven, no age names).
- What the player sees of the system: the planets, moons and resource bodies of the home system
  as the system view draws them now; other players' units in the system only where the existing
  visibility rules show them.
- Docs: `app/docs/world-model.md` (the table: rank 1 first seeded at Fusion, view only until
  Starfaring brings ships), `app/docs/content.md` if it lists the ages' reach.

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main);
  fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new playthrough,
  ascension hashes and rules fingerprint reported with why they moved (or that they did not).
- Screenshots you have Read (`app/docs/img/138-*.png`): a Fusion player zoomed out to the home
  system (e.g. `--view system,home` after `grant-age age.fusion` in a script, as task 131's
  scripts did), and the same player unable to reach the star map.
- Report: files changed, every rank-1 effect you found and what it does at Fusion now, what you
  could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md`, `app/docs/database.md` (ages,
map scale), `app/docs/tasks/130-capital-reveal-ladder.md`, `app/docs/tasks/131-capital-markers.md`
(its scripts show how to grant an age and take a system-view screenshot).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t138/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/138-system-view` on branch `task/138-system-view`; commit
there (stage by name); do not merge, tag or push.
