# Task 137 — Terrain is always known: the fog hides what moves and what is built, not geography

Owner (chat, 2026-11-23): "theres no point in hiding mountains in the first ages". Asked what
should change, the owner chose: **show all terrain** (land, sea, mountains, forests, ice drawn
normally everywhere from the start; the fog only hides units, buildings and resource deposits
not yet seen, and still dims what is out of sight now), on **every planet** (the home planet
from the start and, after Starfaring, every planet). Crates: `rom-render` (the vision shading,
features, the minimap), `rom-app` only where it hands the vision map or the minimap over.
Implementer: Claude agent (Sonnet).

## What exists

`app/crates/rom-render/src/vision.rs`: per tile `UNEXPLORED` (drawn at 32 % brightness),
`EXPLORED` (72 %, seen before, not in view now), `VISIBLE` (100 %); the terrain meshes and the
minimap are darkened by it (`tile_shade`, `worker.rs` hands the map to the render thread).
Check how features (forests, mountains, ice, rocks) and the ground's relief are drawn on
unexplored tiles (`features.rs`, `terrain.rs`, `globe.rs`, `hud_minimap.rs`) and whether
anything is left out there rather than only darkened.

## What changes

- An unexplored tile's **terrain** is drawn like an explored one: every terrain kind, feature
  and relief shows, at the out-of-sight shade (72 %) — the same as ground seen before and not in
  view now; tiles in view stay at full light. The minimap does the same.
- **Unchanged:** units and buildings follow the fog as now (visible ones drawn, remembered
  buildings at their last known place, nothing on tiles never seen); resource deposits and
  gather sites never seen stay hidden (check whether they are entities or terrain features;
  if they are terrain features, keep them hidden on unexplored tiles); territory borders and
  tints stay only on explored tiles (as now: `owner` is "only on explored tiles").
- Space: a planet seen from the system view or from orbit shows its full terrain too
  (every planet, the owner's choice); unexplored **systems** on the star map keep their
  "(unexplored)" marker and what it hides (this is about ground, not about systems).
- Presentation only: no change to the simulation's vision, orders or saved state. If a rule in
  `rom-sim` refuses something on unexplored ground (e.g. building placement) that now looks
  inviting, do not change it; list it in the report.

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main), with
  a test that an unexplored tile's terrain shade equals the out-of-sight shade and that the
  vision map still hides units and buildings there. Playthrough and ascension hashes unchanged
  (556f67e158b4959a / 5655fb44760aa038).
- Screenshots you have Read (`app/docs/img/137-*.png`, the game's own screenshot paths): the
  start of a match on the home planet showing the mountains and coasts of ground never seen,
  the whole-planet view, the minimap, and another planet seen from orbit after Starfaring.
- Report: files changed, what was hidden before besides the darkening, what you could not
  verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md`, `app/docs/hud.md` (minimap),
`app/crates/rom-render/src/vision.rs` and its callers.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t137/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/137-terrain-known` on branch `task/137-terrain-known`;
commit there (stage by name); do not merge, tag or push.
