# Task 106 — Shots to and from orbit travel the orbit's height

Crates: `rom-sim` (combat distance for flight time), schema/data for one rule constant,
`rom-app` (`travel_ticks` in `sim_thread.rs`, which mirrors the simulation). Implementer: Claude
(Sonnet). From task 102's open item; owner agreed 2026-11-19.

## Why

Task 102 gave projectiles a flight time: `travel = distance × tick_rate / speed`. For a shot
between the surface (or air) and orbit, `distance` is measured on the ground to the target's
subpoint, so a shot straight up or straight down takes the one-tick minimum, while the drawing
shows it crossing the orbit's height (8 tiles up in the renderer).

## Build

1. A rule constant for the orbit's height in milli-tiles (e.g. `space.orbit_height`, in the
   `rule_constant`/constant table the simulation already reads; match the renderer's drawn height
   or say why not). This is a data change: add the row in `generate.py` (or the seed's hand part,
   wherever constants live), note it in `database.md`.
2. For flight time only (not for weapon reach, which stays measured on the ground as task 58
   defined it), a shot between layers where one end is in orbit uses
   `sqrt(ground² + height²)` in integer arithmetic (an integer square root; no floats in
   `rom-sim`). Orbit ↔ orbit and space ↔ space are unchanged.
3. Mirror it in `rom-app`'s `travel_ticks` (or, better, have the view take the land tick from
   `World::hits_in_flight` only and drop the mirror where possible).
4. The renderer draws to the same height: if `rom-render` uses its own 8-tile constant, make it
   read the same value from the snapshot or say why they may differ.

Read first: `AGENTS.md`, `CLAUDE.md`, task 102's card and its `database.md` note,
`rom-sim/src/combat.rs` (firing, `travel`), `rom-app/src/sim_thread.rs` (`travel_ticks`,
`recent_shot`), `rom-render/src/projectiles.rs`.

## Done when

- Gate green from `app/`; the playthrough fires no shot, so its hashes should not change (say why
  if they do); fight matrix `unanswered=0`; the two duel tables reported (planet duels; they should
  not move).
- Tests: a ground battery's missile at a ship straight overhead lands after `height / speed`, not
  one tick; a shot at a ground point far away takes the hypotenuse; reach is unchanged; run twice,
  same hash.

Memory rules: `CARGO_BUILD_JOBS=4`; duels `--jobs 2`; check `free -g`; run the app one instance
at a time under `systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 env -u
WAYLAND_DISPLAY …` in self-ending modes. Never `xdotool`, `wmctrl` or `sudo`; do not stop
processes you did not start. Wait loops must not use a `pgrep -f` pattern that matches the loop
itself. Logs only in your own scratchpad subfolder; never write `scratchpad/gate*.sh`.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, deviations, anything not verified.
