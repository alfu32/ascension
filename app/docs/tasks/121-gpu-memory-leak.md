# Task 121 — GPU memory grows without bound until the card is full (issue #4)

Crates: `rom-render` (most likely), `rom-app` only where it owns GPU objects (the raylib-in-GPUI
bridge, the frame hand-over), docs (`app/docs/performance.md`). Implementer: Claude (Opus).
Priority: a bug that makes long matches unplayable; it goes before cards 118–120.

## Why — the owner, verbatim (issue #4, "Big bug on win11")

"I tested it on Windows. It uses the NVIDIA GPU, which is great. So first few minutes runs okay.
But it's hogging GPU memory. And I saw it with my own eyes. It rises up, you can see in the
picture. It rises up to the limit of 4 gigabytes. Never gives anything back. And at which point
when it reaches the upper limit of the NVIDIA graphics, it starts being sluggish. And even on low
detail. So that's not a problem of detail. On high detail it on ultra detail it works just fine
until the 4 gigabytes of the NVIDIA GPU fills up. So I don't know what it is. But I think, as I
said, I think it is hogging memory and never releases it back. That's it
Goes without saying that we absolutely need a fix for this."

The Task Manager photo in the issue: NVIDIA RTX A2000 Laptop GPU, dedicated GPU memory 3.2 / 4.0 GB and
climbing, 3D load only 15 %. Release 0.0.113 (MSI/MSIX, Windows 11).

## Reproduce here

This machine has an NVIDIA GTX 960M (2 GB) beside the Intel HD 530. Run the game on it with
PRIME offload (`__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia`, under
`env -u WAYLAND_DISPLAY`) and sample the process's GPU memory once a second with
`nvidia-smi --query-compute-apps=pid,used_memory --format=csv,noheader` (for graphics processes
use `nvidia-smi --query-accounted-apps` or `nvidia-smi pmon -s m`, whichever reports the
process; check which works first). A self-ending `--bench N` (or a script) of several minutes
that does what a player does: a universe match with computer players at speed 200, camera
panning, rotating, zooming out to the star map and back, switching planets, opening the menus.
Record the memory curve (CSV in the scratchpad `t121/`). If memory grows only under some
actions, bisect by action.

## Where to look (all GPU objects must be created, used and destroyed on the render thread)

- Anything created per frame or per change without a matching `Unload*`: render textures,
  textures, images uploaded as textures (HUD vector rasterisation, text, icons, minimap, star
  map labels), meshes and models (surface chunks re-baked after painting, grass, shadow casters,
  projectiles, selection rings), shaders, framebuffers.
- Objects replaced on resize, on a detail change, on a planet switch, on zoom tier changes, on
  camera moves (LOD rebuilds, grass and far-terrain caches): the old one must be unloaded.
- Caches keyed by something that keeps changing (tick, float distance, camera position) so that
  they never hit and only grow; caches with no size limit.
- The frame readback and worker path from task 108 (PBOs, the latest-frame mailbox): buffers
  re-created instead of reused.
- Drop implementations: raylib-rs wrappers free on drop only if they are owned values;
  `unsafe` raw handles (`ffi::LoadTexture…`) never free themselves.

## Build

1. Measure (as above) on 0.0.113 / main; find the growing objects (counting live GPU objects
   by kind in a debug counter is a good tool: add `rom-render` counters of live textures,
   render textures, meshes, shaders, shown with `--stats` or printed by the bench).
2. Fix every leak: unload what is replaced, bound every cache (LRU by count or bytes), reuse
   buffers.
3. Add the live-object counters to the `--bench` summary line, and a test (headless where
   possible, or a bench check) that a long run's counters stay flat after warm-up.
4. `performance.md`: a "GPU memory" section with the before/after curves (numbers), what
   leaked, and how to measure.

## Done when

- Gate green from `app/` (hashes unchanged: presentation only).
- On the 960M: GPU memory of the game process flat (within a few MB, after warm-up) over at
  least 10 minutes of the scripted session above, before and after numbers in the report; the
  same on the Intel GPU if a counter exists there (the live-object counters at least).
- Report: what leaked (file:line), the fix, the curves, what could not be verified (Windows).

Read first: `AGENTS.md`, `CLAUDE.md`, `research/02-rendering-ui-stack.md`, `app/docs/performance.md`,
card 108 and `108-camera-bench-script.txt`, `rom-render/src/` (all GPU object creation:
grep `load_texture`, `load_render_texture`, `load_model`, `upload`, `gen_mesh`, `ffi::Load`),
`rom-app/src/` render thread and the bridge.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; app runs self-ending under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 env -u WAYLAND_DISPLAY …`.
Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start (the owner may be
playing). Logs only in the scratchpad's `t121/`; never write `scratchpad/gate*.sh`; no cargo
target dir in the scratchpad or /tmp. Work in `../rise-of-man-worktrees/121-gpu-memory` on branch
`task/121-gpu-memory`; commit there; do not merge, tag or push.
