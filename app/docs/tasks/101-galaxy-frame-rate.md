# Task 101 — Galaxy at a playable frame rate

Crates: `rom-app` (space view, snapshots), `rom-render` if the drawing itself is the cost.
Implementer: Claude (Sonnet). After task 99 merges.

## Why

Task 99 made the Galaxy style (745 systems) fit in memory (784 MB) and offered it again, but the
app's star map runs at 12.9 fps with Galaxy (12.6 in a system view) against 46.5 with Quadrant
and 56 with Cluster, at 1280 × 800 (`--bench`). The cost grows with the universe on every frame,
even in a system view. Task 99's suspect, unverified: `rom-app/src/space_view.rs`
`known_systems` (about lines 441–462) scans every body for every system each frame (about
746 × 4,476), and `World::space_of` is a linear search.

## Build

1. **Measure first**: time the per-frame work on the app side (snapshot building, space view,
   star map scene) with a Galaxy match, using `--bench` and timing spans or counters printed at
   the end (no profiler is available). Report where the frame goes.
2. Fix what dominates: index bodies by system once per match (they never move between systems),
   cache what only changes on events (explored systems, claims), and avoid per-frame work over
   all 745 systems where only the visible ones matter. If `rom-sim` needs an index (`space_of`),
   stop and report it: that crate is not in this card.
3. Target: Galaxy star map and system view at 45 fps or more at 1280 × 800; Quadrant and
   Cluster not slower than today.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md` ("Scale"), task 99's notes in
`STATUS.md`, `rom-app/src/space_view.rs`, `sim_thread.rs` (snapshots), `rom-render/src/scene/universe.rs`.

## Done when

- Gate green from `app/`; playthrough hashes unchanged.
- `--bench` numbers before and after for Cluster, Quadrant and Galaxy, star map and system view.
- A test that the index gives the same systems' bodies as the scan.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes, one instance at a time,
Galaxy runs under `systemd-run --user --scope -p MemoryMax=6G -p MemorySwapMax=0`; cargo with
`CARGO_BUILD_JOBS=4`; check `free -g` first (6 GB free or wait). Never use `xdotool`, `wmctrl`
or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name, work in progress committed as soon as it compiles); do not
push or merge. Final message: commands with real output, files changed, numbers, deviations,
anything not verified.
