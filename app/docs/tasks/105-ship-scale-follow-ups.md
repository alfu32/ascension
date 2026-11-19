# Task 105 — Ship scale follow-ups: counts at the floor, pick boxes at reduced render scale

Crates: `rom-render` (space scene, picking), `rom-app` only where the render scale reaches picking.
Implementer: Claude (Sonnet). From task 96's open items; owner 2026-11-19: "we need a visual check
and fix" (counts) and agreed the render-scale pick box is a fault.

## Build

1. **Counts at the floor (visual check, then fix).** Task 96 shows "×N" only on marker groups; a
   fleet still drawn at its floor size (models overlapping on screen) shows no count. Take
   screenshots of a 6-ship and a 20-ship fleet at the zooms where they sit at the floor
   (`app/docs/tasks/96-*-script.txt` set up fleets), look at them, and decide with the pictures:
   overlapping floor-size ships of one owner get one count label like the markers, placed so it
   does not cover the ships; the label never flickers as ships move in and out of overlap (use
   hysteresis or a stable grouping by screen cell). Show before/after.
2. **Pick box at render scale below 100 %.** The floor is computed in render-target pixels, the
   pick box in canvas pixels; at a render scale under 100 % a ship is drawn larger on screen than
   its pick box. Compute both in the same space (canvas pixels, converting with the render scale),
   so the pick box is at least the drawn hull at every render scale. Test: at 50 %, 75 % and
   100 %, the pick box covers the drawn hull and is never under the minimum.

Read first: `AGENTS.md`, `CLAUDE.md`, task 96's card and `world-model.md` section "Ships in space
that scale with the zoom (task 96)", `rom-render/src/layers.rs`, `scene/space.rs`, `space.rs`,
`pick.rs`, `worker.rs`; the render-scale setting (`publish_render_scale`, `rom-app`).

## Done when

- Gate green from `app/`; playthrough hashes unchanged.
- Tests for both points; screenshots you looked at (`app/docs/img/105-*.png`, scripts
  `app/docs/tasks/105-*-script.txt`): a fleet at the floor with its count, before and after; a
  scripted click on a ship at render scale 50 % that selects it.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; run the app one instance at a time under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 env -u WAYLAND_DISPLAY …` in
self-ending modes. Never `xdotool`, `wmctrl` or `sudo`; do not stop processes you did not start.
Wait loops must not use a `pgrep -f` pattern that matches the loop itself. Logs only in your own
scratchpad subfolder; never write `scratchpad/gate*.sh`.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, deviations, anything not verified.
