# Task 36 — Asynchronous frame read-back

Crate: `rom-render` only (plus its tests). No data or schema change.

## Why

Every frame the render thread draws the world and HUD into a render texture,
then `read_back` (`worker.rs`) calls `LoadImageFromTexture`, which blocks until
the GPU has finished the frame, and converts the bottom-up RGBA rows into
top-down BGRA (`pixels.rs`) on the CPU. On the globe at 1280×800 this read-back
is about 15.5 ms of an 18 ms frame (bench 0.0.38, quiet machine: sphere demo
49.9–52.2 fps, renderer 52–54 fps; flat demo 58.1 fps). The target in
`docs/performance.md` is ≥ 55 fps on the sphere demo; the frame budget is the
wait for the GPU, not the drawing commands.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task"
  and rule 8 (graphics threading: everything GL on the render thread; frames
  reach GPUI through a latest-frame mailbox, never a queue).
- `research/02-rendering-ui-stack.md` (the bridge, read-back notes).
- `app/crates/rom-render/src/worker.rs` (the frame loop, `read_back`,
  `snapshot_wanted`, stats), `pixels.rs`, `mailbox.rs`, `stats.rs`.
- `app/crates/rom-app/src/screenshot.rs` and `bench.rs` (how frames are
  captured and counted).
- `app/docs/performance.md`.

## Build

1. **Pixel-buffer read-back.** Keep two (or three) pixel pack buffers per canvas
   size. After drawing frame N, start an asynchronous read of the render
   texture into buffer N mod k (`glReadPixels` with a bound
   `GL_PIXEL_PACK_BUFFER`, ideally in `GL_BGRA` so no channel swap is needed)
   and place a fence. Then map the buffer of an earlier frame whose fence has
   signalled, copy it out flipped (row copies, no per-pixel work when the
   format is BGRA), unmap, and publish it. One or two frames of latency are
   acceptable; never block on a fence that has not signalled unless no frame
   has been published for 100 ms.
2. **GL entry points.** raylib's `rlgl` does not expose pack buffers or fences.
   Load the few functions needed (`glGenBuffers`, `glBindBuffer`,
   `glBufferData`, `glReadPixels`, `glMapBufferRange`, `glUnmapBuffer`,
   `glFenceSync`, `glClientWaitSync`, `glDeleteSync`, `glDeleteBuffers`,
   framebuffer binding) through the loader raylib already uses (for example
   `glfwGetProcAddress`, which the raylib library exports), once, on the render
   thread. Each `unsafe` block carries a one-line reason. If any entry point is
   missing, fall back to today's synchronous `read_back` and say so on stderr
   once.
3. **Resize.** When the canvas size changes, drop the in-flight reads and
   recreate the buffers; never publish a frame of the old size as the new one.
4. **Screenshots.** `snapshot_wanted` must capture a frame drawn after the
   request (the screenshot path waits `--after` seconds; one frame of latency
   is fine). Screenshot output must be identical in content to today's for the
   same scene (pixel layout: top-down BGRA, opaque alpha).
5. **Stats.** `readback_ms` measures the time the render thread spends on
   read-back work (issue + map + copy), so the bench shows the gain; note the
   change in `docs/performance.md` with before/after numbers.
6. Release all GL objects on the render thread when the worker stops.

## Tests to add (no window)

- `pixels.rs`: the flipped BGRA copy from a bottom-up BGRA buffer
  (`bgra_rows_flipped`), and stride handling if rows are padded.
- The ring logic (which buffer to issue, which to map, resize dropping
  in-flight frames, the 100 ms fallback) as a pure state machine with a test:
  `ring_maps_oldest_ready_frame`, `resize_drops_in_flight_frames`,
  `stalled_fence_forces_a_wait_after_100_ms`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --screenshot target/readback-home.png --after 3
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 1 --size 1672x941 --script docs/tasks/35-library-script.txt --screenshot target/readback-library.png --after 12
```

Targets: sphere demo ≥ 55 fps at 1280×800 (report the machine load with
`uptime` next to every bench line; run each bench twice); flat demo not slower;
both screenshots correct (look at them).

## Rules that matter most

- `rom-render` only; no change to `rom-app`'s frame consumption contract
  (`Frame`, mailbox).
- All GL on the render thread; `unsafe` only in `rom-render`, each with a reason.
- Never use `xdotool`, `wmctrl` or similar. `env -u WAYLAND_DISPLAY`. No `sudo`.

## Finish

Commit on the current branch (stage files by name) with a descriptive message
ending with `Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push
or merge. Final message: commands with real output (bench lines with `uptime`),
files changed, deviations and why, anything you could not verify.
