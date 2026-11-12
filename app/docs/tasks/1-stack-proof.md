# Task 1 — Stack proof: raylib world view inside a GPUI window

Crates: `rom-render`, `rom-app`, plus workspace manifests and `app/vendor/`.

This is the highest technical risk in the project. The goal is evidence, not
polish: a GPUI window that shows a continuously rendered raylib scene, driven
by mouse and keyboard, with measured performance.

## Read first

- `AGENTS.md` — all of it (hard rules 7 and 8 matter most).
- `research/02-rendering-ui-stack.md` — sections 1, 2 and 7.
- The working example to port from, in the sibling project
  `/home/alfu64/Development/meshcraft` (read-only; do not modify it):
  - `apps/desktop/src/gpui_app.rs` — `ViewportBridge`, `ShutdownSignal`,
    `RenderThreadGuard`, `RaylibWorker`, `render_loop`,
    `copy_target_to_gpui_image`, `RaylibCanvas`, `retire_viewport_image`,
    and the mouse/scroll/key handlers that feed the bridge.
  - `apps/desktop/Cargo.toml` and the root `Cargo.toml` (`[patch.crates-io]`).
  - `vendor/gpui-component/` (with `MESHCRAFT-PATCHES.md`) and
    `experiments/stack-gpui-raylib/vendor/xattr/`.
  - `apps/desktop/src/linux_graphics.rs` only if start-up fails on this display.

## Build

1. **Dependencies.** Add to the workspace, exact versions:
   `gpui =0.2.2` (`default-features = false`, features `font-kit`, `x11`),
   `gpui-component =0.5.1` (`default-features = false`), `raylib =6.0.0`,
   `image =0.25.10` (`default-features = false`, feature `png`).
   Copy the two patched crates into `app/vendor/gpui-component` and
   `app/vendor/xattr` (keep their licence files; rename the patch notes file
   to `PATCHES.md` and keep its content) and wire them with
   `[patch.crates-io]` in `app/Cargo.toml`. `rom-render` depends on `raylib`
   and `image` only — **not** on `gpui`. `rom-app` depends on `gpui`,
   `gpui-component` and `rom-render`.
2. **`rom-render`: offscreen renderer on its own thread.**
   - A `RenderBridge` (cloneable handle) with: `publish_layout(width, height)`,
     `publish_camera(Camera)`, `push_input(InputEvent)`, `latest_frame() ->
     Option<Frame>`, `stats() -> RenderStats`.
   - `Frame { pixels: Arc<…> BGRA premultiplied, top-down; width; height; sequence: u64 }`
     using a type from the `image` crate or plain bytes — nothing from GPUI.
   - `RenderWorker::start() -> (RenderWorker, RenderBridge)` spawns the thread
     that owns a **hidden** raylib window and every GPU object; `Drop` stops
     and joins it cleanly.
   - The loop renders **continuously** at a 60 Hz target (unlike the editor
     example, which redraws only on change): draw scene to a render texture,
     read it back, convert RGBA→BGRA and flip rows, store in a **latest-frame
     mailbox** (one slot, overwritten). Resize the target when the layout changes.
   - **Internal render scale**: `publish_render_scale(percent)` (50–100). The
     scene is rendered at `scale × layout` and stretched to the layout size
     before read-back — or read back small and let the UI stretch it; pick the
     cheaper one and say which.
   - `RenderStats`: frames rendered, average and worst frame time, average and
     worst read-back + conversion time, over the last second.
3. **Scene for the proof.** A flat grid ground of 256×256 tiles and **5,000
   boxes** drawn with instancing, each moving along its own small circle
   (positions computed on the render thread from elapsed time — this is a
   display test, not simulation code). One directional light with simple
   diffuse shading is enough; no shadows in this task.
4. **RTS camera** in `rom-render`: target point on the ground, yaw, pitch,
   distance. Pan with WASD/arrow keys and with the mouse at the window edge,
   zoom with the wheel (clamped), rotate with middle-button drag, `R` resets.
5. **`rom-app`: GPUI shell.**
   - One window titled "Rise of Man" containing a full-size canvas element that
     shows `latest_frame()` as an image, polling the mailbox every 16 ms and
     notifying only when the sequence number changed.
   - Evict the previous GPU atlas image **two frames after** it is replaced
     (`on_next_frame` twice, then `window.drop_image`), as in the example.
   - Report the laid-out canvas size (in physical pixels, honouring the scale
     factor) to `publish_layout`. Forward mouse move, buttons, wheel and keys
     to `push_input`.
   - A small GPUI overlay in a corner (above the image) showing the stats and
     the current render scale — this proves GPUI elements stack over the frame.
   - Closing the window shuts the render thread down without errors.
6. **Benchmark mode.** `rise-of-man --bench <seconds> [--size WxH] [--scale <percent>]`
   opens the window at that canvas size, runs for the given time, prints one
   line and exits 0:
   `bench size=<W>x<H> scale=<p> frames=<n> fps=<f> frame_ms_avg=<x> frame_ms_max=<x> readback_ms_avg=<x> readback_ms_max=<x>`
7. **Measure and record.** Run the benchmark for 10 seconds at 1280×800,
   1920×1080 and 3840×2160, each at scale 100 and 50, in a **release** build
   (`cargo run --release -p rom-app -- --bench 10 …`), with
   `env -u WAYLAND_DISPLAY` so the X11 path is used. Write the table, the
   machine's CPU/GPU (from `lscpu` and `glxinfo -B` or equivalent), the display
   backend, and a one-paragraph conclusion to `app/docs/performance.md`.
   If a size cannot be opened on this display, record that instead of guessing.

## Rules that matter most

- All raylib calls and GPU objects stay on the render thread.
- The mailbox holds one frame. No queues, no back-pressure onto the render thread.
- `rom-render` must not depend on `gpui`; `rom-app` must not call raylib.
- `unsafe` only where the example needs it (pixel buffer view), each with a
  one-line reason.
- Do not touch `rom-rules`, `rom-sim`, `rom-cli`, the schema, the seed or `research/`.
- Naming rules in `AGENTS.md` apply. The vendored notes file mentions the
  sibling project's name — that is fine; it is the owner's project.

## Tests to add

- `rom-render`: `rgba_to_bgra_flips_rows_and_swaps_channels` — unit test of the
  pixel conversion on a 2×2 image.
- `rom-render`: `camera_zoom_is_clamped`, `camera_pan_moves_target_on_ground_plane`.
- `rom-render`: `mailbox_keeps_only_latest_frame` — publish three frames,
  read once, get sequence 3.
- Tests must not open a window (CI has no display). Keep window code out of
  the tested functions.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --bench 10 --size 1920x1080 --scale 100
```

The last command prints the `bench …` line and exits 0, and
`app/docs/performance.md` contains the six measurements. The target is
**fps ≥ 55 at 1920×1080, scale 100**. If it is not met, do not hide it:
report the numbers and what dominates (render or read-back).

CI installs no graphics libraries. If the workspace no longer builds on a
plain `ubuntu-latest` runner, add the needed `apt-get install` step to
`.github/workflows/ci.yml` (the sibling project's
`packaging/linux/Dockerfile` lists the packages).

## Out of scope

Simulation data, terrain, shadows, materials, HUD, menus, asynchronous
read-back (only if the target is missed — and then as a follow-up task).

## Finish

Commit on the current branch with a descriptive message; do not push. Final
message: commands run and their output, the six benchmark lines, files
created, and anything you could not verify.
