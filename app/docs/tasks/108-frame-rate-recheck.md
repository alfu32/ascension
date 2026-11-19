# Task 108 — Frame-rate re-check across ages and map sizes, and fixes

Crates: measurement first (no code); fixes in `rom-render` and `rom-app` only, as the
measurements show. Implementer: Claude (Opus). Owner 2026-11-19: "do it".

## Why

Since the last frame-rate pass the game gained the Galaxy style (task 99, 101), visible
projectiles (95, 102), higher-detail ships (93), zoom-scaled ships (96, 105), chat (94), market trading (107),
the Sector preset (109) and jumps from anywhere in a system (111). Nobody has
measured the whole range since.

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
