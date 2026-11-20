# Task 108 — Frame-rate re-check across ages and map sizes, and fixes

Crates: measurement first (no code); fixes in `rom-render` and `rom-app` only, as the
measurements show. Implementer: Claude (Opus). Owner 2026-11-19: "do it".

## Why

Since the last frame-rate pass the game gained the Galaxy style (task 99, 101), visible
projectiles (95, 102), higher-detail ships (93), zoom-scaled ships (96, 105), chat (94), market trading (107),
the Sector preset (109) and jumps from anywhere in a system (111). Nobody has
measured the whole range since.

## Owner report and first measurements (2026-11-20)

The owner, playing the Sandbox preset (universe, `map.many_worlds`, age 13, rich-start script,
`--speed 500`, window about 1920×1080) on the integrated Intel HD 530: "choppy picture" and "clicks
react late". Owner: leave the discrete GPU aside; do the best on the integrated GPU.

Orchestrator measurements (`--bench`, 0.0.108; 0.0.103 built alongside for comparison):
- No regression from the network work: 0.0.103 and 0.0.108 within noise everywhere (reference
  `--single-planet --sphere 192 --demo`: 53–57 fps shown; universe age 13 with a camera script
  that turns, pans and zooms out to orbit and the system: 55–56 fps at 1280×800, 43–45 fps at
  1882×1043).
- **Game speed costs frames.** Universe age 13 at 1882×1043, no camera script: speed 100 →
  42 fps shown, render `frame_ms_avg` 8.8; speed 500 → 34.6 fps, `frame_ms_avg` 17.3. `--sun
  fixed` does not change it; `--scale 75` does not raise the shown fps at speed 500 (34.8). So the
  cost follows the number of new simulation snapshots, not pixels: the simulation thread calls
  `publish` after every tick (`sim_thread.rs`, inside `for _ in 0..ticks`), up to 100 snapshots a
  second at 500 %, and the render thread appears to rebuild per new snapshot.
- At 1882×1043 the window shows ~43 of 60 rendered frames even at speed 100 (presentation:
  read-back, upload, GPUI).

Added to the build (do these first):
- **One snapshot per loop turn.** Publish after the last tick of a turn, not after each tick
  (keep every per-tick duty such as notices and autosave per tick). Measure the simulation
  thread's CPU and the render frame time at speeds 100, 300 and 500.
- **Cheaper per-snapshot work on the render thread.** Profile what a new snapshot costs
  (instance lists, fog or vision textures, labels, HUD) and cut it; target: speed 500 costs no
  more than 2 ms of render frame time over speed 100.
- **Click-to-response time.** Measure from a pointer event to the first shown frame that reflects
  it (selection ring or order marker), at speed 100 and 500, before and after; report it.
- **Presentation at 1920×1080.** Find what limits shown frames to ~43 of 60 and fix what is
  clearly fixable; target ≥ 55 fps shown at 1882×1043, speed 100, on the HD 530.
- The camera script used: rotate (plain drag), pan (Shift drag), wheel out to orbit and to the
  system and back, repeated (keep one in `app/docs/tasks/108-camera-bench-script.txt`).
- The worktree holds uncommitted work from the stopped earlier attempt (`bench.rs`,
  `readback.rs`, `scene/shadow.rs`, `worker.rs`): review it, keep what measures better, drop the
  rest, and merge `main` first (the branch is older than 0.0.104).

## Build

1. **Measure** with `--bench` (1280×800 and 1920×1080, 20 s, after a warm-up), one run at a time,
   twice each, on a quiet machine (check `uptime` load and say it):
   - the home planet at ages 0, 4, 7, 10, 13 (`--age`), default zoom and zoomed out;
   - a large battle (task 95's 200-versus-200 script) with projectiles on;
   - the star-system view and the star map for every preset since 0.0.102: Planet, Solar, Cluster,
     Sector, Quadrant and Galaxy (largest seed of each from `docs/world-model.md`, "Map styles";
     Galaxy alone under the memory cap);
   - orbit with 20 ships (`72-bench-script.txt`).
   Record render-thread fps, presented fps, frame ms avg/max, sim ticks per second, memory (RSS).
2. **Find** the worst cases (anything under 45 fps presented, or frame max over 100 ms, or sim
   below 20 ticks/s) and profile them as task 101 did (temporary timing marks, removed after).
3. **Fix** what is clearly fixable in the render and app threads without changing the look or the
   simulation; for anything bigger, write it up as a follow-up with numbers.
4. Record the table in `app/docs/performance.md` (before and after).

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/performance.md`, task 101's card and STATUS row,
`research/02-rendering-ui-stack.md`, the bench scripts under `app/docs/tasks/*bench*`.

## Done when

- Gate green from `app/`; playthrough hashes unchanged.
- The measurement table before and after in `performance.md`; each fix with its numbers.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g` before every run; run the app one instance at
a time under `systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 env -u
WAYLAND_DISPLAY …` (Galaxy may need more: up to `MemoryMax=8G`, alone, only with 12 GB free).
Never `xdotool`, `wmctrl` or `sudo`; do not stop processes you did not start (the owner may be
playing: if another `rise-of-man` process is running, say so, since it skews the numbers). Wait
loops must not use a `pgrep -f` pattern that matches the loop itself. Logs only in your own
scratchpad subfolder; never write `scratchpad/gate*.sh`.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, the table, files changed, deviations, anything not verified.
