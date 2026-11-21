# World view performance

Measurements of the stack proof (task 1): raylib renders the world offscreen on
its own thread, the frame is read back to the CPU, converted to BGRA and shown
by GPUI as an image. Scene: a 256×256-tile grid ground (one quad, grid lines
in the fragment shader) and 5,000 boxes drawn with one instanced draw call,
each moving on its own circle, one directional light, no shadows.

## How to reproduce

```sh
cd app
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --bench 10 --size 1920x1080 --scale 100
```

The window opens without a title bar at the requested size, waits for the
first frame at that size, warms up for 1 s, then measures for the given time
and prints one line on stdout (plus a `bench-detail` line on stderr).

| Field | Meaning |
| --- | --- |
| `size` | canvas size the window system actually gave, in physical pixels |
| `frames`, `fps` | frames **shown** by GPUI during the measurement |
| `frame_ms_avg` / `_max` | render thread, frame start → end of its read-back work: draw calls and read-back (since task 36 this no longer includes waiting for the GPU, see below) |
| `readback_ms_avg` / `_max` | read-back work on the render thread. Since task 36: starting the asynchronous read, polling the fence, mapping an earlier frame's buffer and copying its rows. Before: `LoadImageFromTexture` (which first waited for the GPU to finish the frame) plus RGBA→BGRA and row flip |
| `bench-detail rendered_fps` | frames the render thread produced (paced to 60 Hz) |

Averages are over the whole measurement; maxima are the worst frame seen.
With render scale below 100 the frame is rendered **and read back** at the
reduced size and GPUI stretches the image to the canvas; that is cheaper than
stretching on the GPU before read-back, because read-back, conversion and the
GPUI texture upload all shrink with the pixel count.

## Machine

| | |
| --- | --- |
| CPU | Intel Core i7-6700HQ, 4 cores / 8 threads, 2.6–3.5 GHz |
| GPU used | Intel HD Graphics 530 (SKL GT2), integrated, Mesa 25.2.8; raylib on OpenGL 4.6 core (per `glxinfo -B`); GPUI on Vulkan, where `vulkaninfo` lists this device first and also a GeForce GTX 960M (NVK) — which one GPUI picked was not checked |
| Memory | 32 GB |
| OS | Ubuntu 25.10, Linux 6.17, GNOME Shell 49 (Wayland session) |
| Display path | X11 via XWayland (`env -u WAYLAND_DISPLAY`); two 1920×1080 monitors at 60 Hz; the work area is 1882 px wide because of the dock |
| Build | Rust 1.99.1, `--release` |

Runs were taken on a normal desktop session (editor, browser and the shell
compositor running), not an idle machine. Builds of a parallel task may have
run at any time; one was observed only during the repeats at the end.

## Results

Release build, 10 s each, mailbox polled every 8 ms (see below):

| Requested | Scale | Canvas measured | Render target | Shown fps | Rendered fps | Frame avg / max (ms) | Read-back avg / max (ms) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1280×800 | 100 | 1280×800 | 1280×800 | 56.5 | 59.9 | 8.55 / 15.49 | 8.29 / 14.74 |
| 1280×800 | 50 | 1280×800 | 640×400 | 59.6 | 59.9 | 4.14 / 20.83 | 3.90 / 20.63 |
| 1920×1080 | 100 | **1882×1048** | 1882×1048 | 59.8 | 60.0 | 11.26 / 21.10 | 11.01 / 20.75 |
| 1920×1080 | 50 | **1882×1048** | 941×524 | 56.9 | 60.1 | 6.07 / 12.53 | 5.77 / 12.24 |
| 3840×2160 | 100 | **cannot be opened** — got 1882×1048 | 1882×1048 | 60.0 | 60.0 | 10.86 / 16.64 | 10.59 / 16.30 |
| 3840×2160 | 50 | **cannot be opened** — got 1882×1048 | 941×524 | 59.9 | 60.0 | 4.82 / 8.20 | 4.55 / 7.89 |

The benchmark lines, verbatim:

```
bench size=1280x800 scale=100 frames=565 fps=56.5 frame_ms_avg=8.55 frame_ms_max=15.49 readback_ms_avg=8.29 readback_ms_max=14.74
bench size=1280x800 scale=50 frames=596 fps=59.6 frame_ms_avg=4.14 frame_ms_max=20.83 readback_ms_avg=3.90 readback_ms_max=20.63
bench size=1882x1048 scale=100 frames=598 fps=59.8 frame_ms_avg=11.26 frame_ms_max=21.10 readback_ms_avg=11.01 readback_ms_max=20.75
bench size=1882x1048 scale=50 frames=569 fps=56.9 frame_ms_avg=6.07 frame_ms_max=12.53 readback_ms_avg=5.77 readback_ms_max=12.24
bench size=1882x1048 scale=100 frames=600 fps=60.0 frame_ms_avg=10.86 frame_ms_max=16.64 readback_ms_avg=10.59 readback_ms_max=16.30
bench size=1882x1048 scale=50 frames=599 fps=59.9 frame_ms_avg=4.82 frame_ms_max=8.20 readback_ms_avg=4.55 readback_ms_max=7.89
```

**Sizes this display cannot open.** Both monitors are 1920×1080 and the
window manager keeps windows inside the work area, so a 1920×1080 request
became a 1882×1048 canvas (95 % of the pixels; one run got 1882×1080) and a
3840×2160 request was clamped to the same 1882×1048. The last two rows are
therefore repeats of the 1882×1048 case, not 4K measurements. 4K has **not**
been measured; it needs a 4K monitor. A fullscreen request did not change the
size on this desktop.

## Polling interval

The task card asked for a 16 ms mailbox poll. At 1920×1080 that lost up to a
fifth of the frames although the render thread kept 60 frames/s: the poll and
the render clock run at almost the same rate, so their phase drifts, and while
the UI thread is busy with a draw two frames can land in one poll interval and
the older one is overwritten. Temporary counters confirmed that every lost
frame was overwritten in the mailbox, none in the UI. Polling every 8 ms costs
one uncontended mutex check per poll and still notifies GPUI only when a new
frame arrived.

| Poll | Shown fps, 1920×1080 request, scale 100, three 10 s runs |
| --- | --- |
| 16 ms | 49.5, 47.8, 60.0 |
| 8 ms | 57.7, 60.0, 59.1 |

Even at 8 ms, runs of the same configuration vary: 1280×800 at scale 100
showed between 52.1 and 59.9 frames/s over six runs (some of them with
simulated input or screen captures running at the same time). The rendered
rate stayed at 60 in every run, so the variation is on the presenting side
(UI thread stalls on a shared desktop and GPU).

## Repeats of the definition-of-done command

`--bench 10 --size 1920x1080 --scale 100`, run last, after the table above.
During these runs another project build (several `rustc` processes, about
4 cores busy) was running on the same machine:

```
bench size=1882x1080 scale=100 frames=508 fps=50.8 frame_ms_avg=13.49 frame_ms_max=38.90 readback_ms_avg=13.12 readback_ms_max=36.80
bench size=1882x1080 scale=100 frames=550 fps=55.0 frame_ms_avg=12.14 frame_ms_max=33.21 readback_ms_avg=11.82 readback_ms_max=31.37
bench size=1882x1080 scale=100 frames=580 fps=58.0 frame_ms_avg=11.21 frame_ms_max=20.69 readback_ms_avg=10.93 readback_ms_max=20.46
```

Under that load the render thread itself fell below 60 (56.8 and 58.6
rendered frames/s in the first two runs) and the worst frame reached 39 ms,
so the target is met on a quiet machine but not with margin on a busy one.

## Where the time goes

Temporary timers around the two halves of the read-back, 300-frame averages:

| Render target | `LoadImageFromTexture` (GPU wait + copy) | RGBA→BGRA + flip (CPU) |
| --- | --- | --- |
| 1882×1048 | 7.7–7.9 ms | 3.3–3.4 ms |
| 1280×800 | 6.4 ms | 1.6–1.8 ms |
| 941×524 | 4.1–4.5 ms | 0.9 ms |

Issuing the draw calls takes about 0.3 ms. The read-back call blocks until the
GPU has finished drawing, so its time is mostly GPU work (scene plus
transfer) on an integrated GPU that the compositor and GPUI share.

## Conclusion

The image hand-off reaches the target on this machine when it is otherwise
quiet: the render thread sustains 60 frames/s at every size that could be
opened, and GPUI shows 57–60 of them at about 1920×1080 with scale 100 (59.8
in the recorded run, 57.7–60.0 over quiet repeats), meeting the ≥ 55 fps
target for the windowed 1882×1048 canvas, the largest this display allows.
With a parallel build running it dropped to 50.8–58.0. Exact 1920×1080 and
3840×2160 could not be opened here. The render thread's own cost is
dominated by the synchronous read-back (about 11 ms of a 16.7 ms budget at
1882×1048: 7.7 ms waiting for the GPU and copying, 3.3 ms converting), so it
grows with pixel count and leaves little headroom. 4K was not measured;
scaling the measured parts linearly (4.2× the pixels of 1882×1048) suggests it
would not fit in a 60 Hz frame with synchronous read-back at scale 100. Halving the render scale cuts
the render-thread time roughly in half and is the first mitigation; the
second, asynchronous read-back through pixel-buffer objects (and doing the
channel swap on the GPU), should be a follow-up task before shadows and a
real HUD are added. Shown-frame losses that remain come from the UI side, not
the renderer.

## Terrain scene (task 8)

The scene is now a generated map (default style, seed 1, 4 players, 256×256
tiles): chunked terrain meshes with vertex colours and sun lighting, a
translucent sea, 3,000 stars and start/node markers. The 5,000 moving boxes are
off by default (`--boxes <n>`). Mesh build for the 256×256 map: about 9 ms for
the vertex data plus about 8 ms for the upload (30 meshes); for a 384×384
`map.many_worlds` map 16 ms plus 13 ms (54 meshes). Printed once as
`terrain-mesh` on stderr when the terrain is published.

```
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --bench 10 --size 1280x800 --scale 100
bench size=1280x800 scale=100 frames=599 fps=59.9 frame_ms_avg=8.59 frame_ms_max=13.84 readback_ms_avg=8.43 readback_ms_max=13.62
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --bench 10 --size 1920x1080 --scale 100
bench size=1882x1080 scale=100 frames=562 fps=56.2 frame_ms_avg=12.26 frame_ms_max=24.95 readback_ms_avg=12.11 readback_ms_max=24.74
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --bench 10 --size 1280x800 --scale 100 --boxes 5000
bench size=1280x800 scale=100 frames=560 fps=56.0 frame_ms_avg=9.84 frame_ms_max=16.15 readback_ms_avg=9.43 readback_ms_max=15.70
```

The largest canvas this display allows is 1882×1080 (a 1920×1080 request is
clamped by the window manager). The terrain adds almost nothing to the frame
time at 1280×800; read-back still dominates. Screenshots are kept in
`docs/img/terrain-home.png` and `docs/img/terrain-worlds.png`.

## Live entities (task 10)

The simulation runs on its own thread (20 ticks per second at `--speed 100`) and publishes a
plain snapshot after every tick; the render thread keeps the last two, interpolates positions
and facing by wall-clock time, and draws units, buildings, nodes and health bars with one
instanced draw call (unit cubes whose colour rides in the unused matrix cells). The default
demo has 4 players with a city and 5 builders each plus the map's nodes (37 entities). The
stress case adds 5,000 wandering units on the same 256×256 map (`--boxes` stays 0). Both runs:
1280×800, scale 100, seed 1, 10 s, on the machine described above (quiet desktop).

```
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
bench-detail sim_tick=226 sim_ticks_per_second=20.0 entities=37
bench size=1280x800 scale=100 frames=599 fps=59.9 frame_ms_avg=8.13 frame_ms_max=13.18 readback_ms_avg=7.97 readback_ms_max=12.98
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --stress-units 5000 --seed 1 --size 1280x800 --bench 10
bench-detail sim_tick=226 sim_ticks_per_second=19.9 entities=5037
bench size=1280x800 scale=100 frames=584 fps=58.4 frame_ms_avg=11.12 frame_ms_max=17.50 readback_ms_avg=9.63 readback_ms_max=15.81
```

Two earlier runs of the same commands gave 57.0 fps (default demo) and 59.9 fps (stress), so
the shown rate varies by a couple of frames per second between runs as in the earlier tables.
With 5,000 wandering units the simulation still kept 19.9 of 20 ticks per second, and the
render thread stayed at about 60 frames per second; the extra cost is about 3 ms per frame
(frame time 8.1 ms to 11.1 ms), mostly building the instance list on the CPU. A screenshot of
the default demo after 20 s is in `docs/img/demo.png`.

## HUD chrome (task 11)

HUD-on GPU measurements are pending the orchestrator's display-enabled run.
This implementer was explicitly restricted to CPU previews and compilation;
no application window was opened, no GPU screenshot was produced, and no FPS
number is claimed. Run the following to obtain the required bench line:

```sh
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

The HUD uses one artwork atlas texture, one glyph atlas texture, and a terrain
minimap texture uploaded once per terrain. Each frame adds textured quads,
entity dots and the camera footprint. Atlas/glyph baking happens at startup
and after scale changes beyond 10%, outside steady-state frame costs.

## Sphere planet (task 18)

The commands below use task 41's `--single-planet` flag to preserve the isolated
sphere. When running a commit from before task 41, omit that flag: `--sphere`
already selected an isolated planet there. Measurements retain the versions
described in each section.

`--single-planet --sphere 192` draws a generated planet of 6 × 192² = 221,184 tiles on a globe of radius
122 tiles: 216 land chunks and 156 sea chunks (at most 32 × 32 tiles each, 372 meshes in
all), about 440,000 land triangles. Chunks wholly beyond the horizon or outside the view
pyramid are not drawn (`ChunkBounds::may_be_seen`). The simulation is not started in sphere
mode, so the numbers are rendering only (HUD on, 1280×800, scale 100, 10 s, seed 1).

```sh
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --single-planet --sphere 192 --seed 1 --size 1280x800 --bench 10 --orbit 0,10,12,30   # street
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --single-planet --sphere 192 --seed 1 --size 1280x800 --bench 10 --orbit 0,10,90,0    # mid
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --single-planet --sphere 192 --seed 1 --size 1280x800 --bench 10 --orbit 0,10,330,0   # globe
```

| View | Bench line |
| --- | --- |
| street (distance 12) | `bench size=1280x800 scale=100 frames=585 fps=58.5 frame_ms_avg=8.82 frame_ms_max=18.62 readback_ms_avg=8.47 readback_ms_max=18.25` |
| mid (distance 90) | `bench size=1280x800 scale=100 frames=572 fps=57.2 frame_ms_avg=10.90 frame_ms_max=15.34 readback_ms_avg=10.43 readback_ms_max=14.99` |
| globe (distance 330) | `bench size=1280x800 scale=100 frames=557 fps=55.7 frame_ms_avg=9.92 frame_ms_max=15.97 readback_ms_avg=9.30 readback_ms_max=15.44` |

Mesh build and upload, printed once at start-up as `planet-mesh n=192 meshes=372
build_ms=… upload_ms=…`: about 110–125 ms to build the globe geometry (vertex directions,
heights, colours and normals for 223,000 corners) and the chunk meshes, and 30–45 ms to
upload them (five runs: build 109–126 ms, upload 29–43 ms). Nothing is built after
start-up.

The render thread keeps 60 frames per second in every view; the "shown" rate is what GPUI
presents and varies by a few frames per second between runs, more while other builds run on
the machine (this task's runs shared the machine with another task's compiler). Before
chunk culling the same scene showed 49–59 fps, the extra GPU time going to the far side of
the planet, which the culling now skips. The whole frame is dominated by waiting for the GPU
in the read-back, as in the flat scene. Screenshots: `docs/img/globe-street.png`,
`globe-mid.png`, `globe-full.png`.

## Continuous zoom (task 22)

`--universe --seed 1 --zoom-all`, 1280x800, 10 s, with the `--view` named below. The
machine was busy with other tasks' builds and runs; the unchanged `--single-planet --sphere 192 --demo`
bench measured in the same session is listed for comparison. The frame time is dominated
by the read-back in every row (11-15 ms), not by drawing.

| view | bench line |
| --- | --- |
| globe | `bench size=1280x800 scale=100 frames=521 fps=52.1 frame_ms_avg=13.31 frame_ms_max=42.65 readback_ms_avg=11.38 readback_ms_max=31.99` |
| system | `bench size=1280x800 scale=100 frames=466 fps=46.6 frame_ms_avg=17.32 frame_ms_max=70.81 readback_ms_avg=14.52 readback_ms_max=42.25` |
| star map | `bench size=1280x800 scale=100 frames=453 fps=45.3 frame_ms_avg=18.39 frame_ms_max=60.01 readback_ms_avg=15.54 readback_ms_max=37.50` |
| `--single-planet --sphere 192 --demo` (reference) | `bench size=1280x800 scale=100 frames=532 fps=53.2 frame_ms_avg=12.18 frame_ms_max=45.41 readback_ms_avg=10.64 readback_ms_max=30.76` |

Screenshots: `docs/img/zoom-globe.png`, `zoom-system.png`, `zoom-starmap.png`, `zoom-capped.png`.

## Day/night lighting, shadows, sky and atmosphere (task 28)

The directional light that represents the sun orbits the planet orb once per
6,000 simulation ticks by default. The snapshot tick drives terrain lighting,
the local sky, star fade and the atmosphere halo; pausing freezes all of them.
`--sun fixed` keeps the light over the initial view; `--sun night` is a fixed
midnight preview. `G` toggles the graticule; Shift+G works when the command
palette occupies G. `--graticule off` starts with the lines hidden.

Shadows use a 2048² RGB-packed render target, 3×3 PCF, slope-scaled depth bias
and a normal offset. The fitted camera encloses the visible surface, including
raised terrain beyond the street-level sea horizon, rather than the entire
planet. Its depth range includes terrain chunks and entity bounds overlapping
the receiver footprint, so tall and offscreen casters are not clipped. The map is reused
until the light turns by 0.15°, the fitted view changes or caster geometry
changes. Above 0.8 planet radii in orbit distance the pass is skipped and the
soft terminator supplies globe lighting. Entity colours retain neutral fill
through the day and night; daylight shadows remain visible on them.
Camera-facing health bars do not cast shadows or invalidate the cached map.

GPU measurements for the completed implementation are **pending the
orchestrator's display-enabled run**. The continuation sandbox cannot open a
window. No new FPS numbers or screenshots were produced here, and the 55 fps
target has not been verified. Capture the actual bench lines with these
commands from `app/` (fixed daylight ensures the street and mid shadow passes
are active even when the chosen camera is far from the home start):

```sh
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --single-planet --sphere 192 --demo --seed 1 --sun fixed --size 1280x800 --orbit 0,10,12,30 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --single-planet --sphere 192 --demo --seed 1 --sun fixed --size 1280x800 --orbit 0,10,90,0 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --single-planet --sphere 192 --demo --seed 1 --sun fixed --size 1280x800 --orbit 0,10,330,0 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

The WIP commit supplied `docs/img/sun-globe.png` and `sun-later.png` (two
terminator positions and graticule), `shadows-street.png` (street scene),
`sun-system.png` (star-facing planet lighting), and `sky-day.png` /
`sky-night.png` (blue sky versus stars over the same terrain). These are
inherited reference captures, not evidence for the final code: regenerate
the task-card screenshots to check the corrected entity fill, system-frame
orientation, shadow fit and sky gradient. Also inspect the outward-fading
halo around the globe and habitable system bodies, its brighter day side,
and the surface-to-space transition. The sky and halo colour/intensity,
star fade, lighting directions, graticule geometry and light-camera fit are
covered by headless unit tests. Regression tests also sample raised receivers
around the horizon, fit tall casters without widening the map, and verify that
damage and health-bar rotation leave caster geometry unchanged.

For matched surface-sky captures, use the same low camera in fixed daylight
and fixed midnight. The globe and system captures in the task card exercise
the atmosphere halo from outside:

```sh
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --single-planet --sphere 192 --seed 1 --sun fixed --size 1280x720 --orbit 0,10,8,0 --no-hud --after 1 --screenshot target/sky-day.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --single-planet --sphere 192 --seed 1 --sun night --size 1280x720 --orbit 0,10,8,0 --no-hud --after 1 --screenshot target/sky-night.png
```

## Terrain materials and baked feature objects (task 34)

Up close the land is shaded by the material library (`assets/shaders/`: grass,
sand, stone, snow, forest floor; animated water on the sea), blended to the map
colours between camera distances `MATERIAL_NEAR` = 64 and `MATERIAL_FAR` = 120
tiles (`rom-render/src/looks.rs`); each fragment also falls back to its map
colour between 70 and 150 tiles from the eye. Value noise is one fetch of a
256² noise texture (red = value, green = the next z slice), and octaves finer
than a pixel are skipped. Forests (canopy centre, edge trees with trunks),
mountain massifs and farm crop rows are baked once when the planet or map
reaches the render thread; the simulation starts only after that. Each feature
tile gets at most one instance in 16 × 16-tile chunks; chunks beyond 130 tiles
are skipped, instances beyond 36 tiles use coarse meshes, only chunks within
70 tiles cast shadows (coarse meshes), and the objects fade out across the
material band by a screen-door pattern.

Bake, printed once as `feature-bake` (sphere n = 192, seed 1): 19 variant
meshes, 2,960 vertices, 29,240 instances in 245 chunks, bake 39–200 ms
(41 ms on a quieter machine), upload 3–17 ms — well under the 3 s budget.

Benches (1280×800, scale 100, seed 1, 10 s). The machine was shared with other
tasks' test runs (load average 5–13 throughout); the parent commit's binary,
benched alternately in the same session, is given for comparison.

| Run | Bench line |
| --- | --- |
| `--single-planet --sphere 192 --demo` (home view) | `bench size=1280x800 scale=100 frames=501 fps=50.1 frame_ms_avg=19.40 frame_ms_max=45.98 readback_ms_avg=17.08 readback_ms_max=28.17` |
| `--single-planet --sphere 192 --demo --look-at ter.forest,8` (closest zoom over a forest edge) | `bench size=1280x800 scale=100 frames=501 fps=50.1 frame_ms_avg=19.34 frame_ms_max=44.80 readback_ms_avg=16.90 readback_ms_max=25.98` |
| `--demo` (flat) | `bench size=1280x800 scale=100 frames=566 fps=56.6 frame_ms_avg=13.82 frame_ms_max=26.81 readback_ms_avg=12.90 readback_ms_max=22.90` |
| parent commit, `--single-planet --sphere 192 --demo`, alternating runs | 53.0 / 52.0 fps, frame 16.7 / 15.8 ms (new build in between: 51.5 / 47.2 fps, 19.0 / 20.4 ms) |

The material look costs about 3 ms of render-thread frame time at the home
view (land shading about 2.5 ms, objects and their shadows about 1–1.5 ms, the
sea next to nothing). The 55 fps target was **not** reached on the sphere
during these runs; the parent commit did not reach it either under the same
load (49.5–53 fps). A quiet-machine measurement is still owed.

`--look-at <terrain>[+<terrain>][,<distance>]` places the camera over the
edge tile of a terrain stand nearest the home city, facing a second terrain
within 40 tiles. Screenshots: `docs/img/materials-street.png`,
`forest-mountain.png`, `forest-close.png`, `mountain-massif.png`,
`farm-rows.png` (farm built by `docs/tasks/34-farm-script.txt`),
`materials-blend.png`, `materials-globe.png`, `materials-globe-day.png`,
`materials-flat.png`.

## Asynchronous read-back (task 36)

The render thread no longer waits for the GPU. After drawing a frame it sets the
frame's alpha to opaque on the GPU (a colour-masked clear), starts
`glReadPixels` in `GL_BGRA` into one of three pixel pack buffers (two since task 108) and places a
fence (`rom-render/src/readback.rs`). It then polls the fence of the oldest read
in flight; once it has signalled, that buffer is mapped, its rows are copied
top-down into the frame (whole-row copies, no per-pixel work) and the frame is
published. Frames are published in order with one or two frames of latency. A
read whose fence has not signalled is waited for only when no frame has been
published for 100 ms. A canvas resize drops the reads in flight and resizes the
buffers, so a frame is always published at the size it was drawn. A snapshot
request is answered by the first frame drawn after it.

The GL entry points (buffers, fences, `glReadPixels`, framebuffer binding) are
loaded once on the render thread through raylib's own loader, `rlGetProcAddress`
(which calls `glfwGetProcAddress`; `nm` on the built `libraylib.a` shows both
exported, along with raylib's GLAD table). If the context is older than OpenGL
3.3 or an entry point is missing, the render thread says so once on stderr and
reads back synchronously as before. All buffers and fences are released on the
render thread before the GL context closes.

`readback_ms` now measures the read-back work on the render thread: issue, fence
poll, map and row copy. Benches (1280×800, scale 100, seed 1, 10 s), the parent
commit's binary and this one alternating in the same session, `uptime` after
each run:

| Run | Bench line | Rendered fps | Load (1 min) |
| --- | --- | --- | --- |
| sphere, before | `bench size=1280x800 scale=100 frames=539 fps=53.9 frame_ms_avg=17.23 frame_ms_max=48.55 readback_ms_avg=15.11 readback_ms_max=24.88` | 55.1 | 5.09 |
| sphere, after | `bench size=1280x800 scale=100 frames=553 fps=55.3 frame_ms_avg=3.69 frame_ms_max=30.68 readback_ms_avg=1.30 readback_ms_max=5.49` | 59.4 | 4.70 |
| sphere, before | `bench size=1280x800 scale=100 frames=510 fps=51.0 frame_ms_avg=17.76 frame_ms_max=48.79 readback_ms_avg=15.36 readback_ms_max=24.73` | 53.9 | 4.36 |
| sphere, after | `bench size=1280x800 scale=100 frames=544 fps=54.4 frame_ms_avg=4.14 frame_ms_max=26.46 readback_ms_avg=2.11 readback_ms_max=4.61` | 59.9 | 3.97 |
| flat, before | `bench size=1280x800 scale=100 frames=546 fps=54.6 frame_ms_avg=13.84 frame_ms_max=25.80 readback_ms_avg=13.01 readback_ms_max=22.79` | 59.0 | 3.99 |
| flat, after | `bench size=1280x800 scale=100 frames=540 fps=54.0 frame_ms_avg=3.12 frame_ms_max=10.88 readback_ms_avg=2.30 readback_ms_max=5.62` | 60.0 | 3.61 |
| flat, before | `bench size=1280x800 scale=100 frames=549 fps=54.9 frame_ms_avg=14.33 frame_ms_max=27.99 readback_ms_avg=13.43 readback_ms_max=23.64` | 58.4 | 3.44 |
| flat, after | `bench size=1280x800 scale=100 frames=589 fps=58.9 frame_ms_avg=2.97 frame_ms_max=10.79 readback_ms_avg=2.23 readback_ms_max=6.52` | 60.0 | 3.22 |

Two further sphere runs of this build: 55.2 and 55.5 fps shown, 59.6 and 59.5
rendered (load 3.1 and 3.0). Sphere is `--single-planet --sphere 192 --demo`, flat is `--demo`.

Read-back work fell from about 15 ms to about 2 ms on the sphere and from about
13 ms to about 2 ms on the flat map, and the render thread now keeps 60 frames
per second in both (it was 54–55 on the sphere). The rate GPUI *shows* rose
less: 54.4–55.5 on the sphere (target ≥ 55, met in three of four runs) against
51.0–53.9 before. The remaining loss is on the presenting side, as in the
earlier sections: the render thread publishes nearly every frame it draws (a
temporary counter during development: 663 published of 665 drawn), and the
GPU is now busy with the raylib frame while GPUI and the compositor draw, on an
integrated GPU all three share. The flat map was not slower (54.0 and 58.9 shown
against 54.6 and 54.9; 60.0 rendered against 58.4–59.0).

Screenshots are unchanged in content: the top HUD bar of `--single-planet --sphere 192 --seed 1`
and of the task-35 library script is pixel-identical to the parent commit's
capture, alpha is 255 everywhere, and the rest differs only by animated shading
(372 and 13 pixels differ by more than 3 %).

## Heavy bombardment (task 75)

```sh
env -u WAYLAND_DISPLAY target/release/rise-of-man --universe --players 2 --ai none --seed 1 \
  --sun fixed --size 1280x800 --script docs/tasks/75-bench-script.txt --bench 15
```

Eight warships (four battleships, four frigates) hold the orbit over a town of fifteen buildings
and twenty soldiers and fire on everything in reach from the first ticks; 0.0.75's binary runs the
same script with its own weaker bombardment (no blasts, no ruins). Two runs each, alternating, on
the i7-6700HQ:

| Build | Load (`uptime`, 1 min) | Shown fps | Rendered fps | Frame ms (avg / max) | Entities at the end |
| --- | --- | --- | --- | --- | --- |
| 0.0.75 | 0.42 | 56.4 | 59.9 | 4.11 / 23.6 | 56 |
| task 75 | 1.23 | 58.5 | 59.9 | 5.13 / 19.8 | 22 |
| 0.0.75 | 1.40 | 58.0 | 59.9 | 3.85 / 20.4 | 56 |
| task 75 | 1.41 | 57.4 | 59.9 | 5.40 / 20.3 | 22 |

The effects (flashes, rings, smoke and fire, the minute of scorched ground and the ruins) cost
about 1.3 ms of a frame here; the render thread keeps 60 frames and the simulation 20 ticks per
second. The town is levelled in the run (22 entities left of 56).

## Higher-definition starships (task 93)

```sh
env -u WAYLAND_DISPLAY target/release/rise-of-man --universe --players 2 --ai none --seed 1 \
  --sun fixed --size 1280x800 --script docs/tasks/93-bench-script.txt --view system --bench 12
env -u WAYLAND_DISPLAY target/release/rise-of-man --universe --players 2 --ai none --seed 1 \
  --sun fixed --size 1280x800 --script docs/tasks/72-bench-script.txt --bench 25
```

Sixty ships of every class on the plane of the home system, all in sight (system view; at this
distance they take their far versions, which kept their cost), and twenty starships in orbit over
the city (planet view, near versions, now about four times the triangles). 0.0.90's binary
against task 93's, alternating, on the i7-6700HQ beside another agent's build:

| View | Build | Load (1 min) | Shown fps | Frame ms (avg / max) |
| --- | --- | --- | --- | --- |
| system, 60 ships | 0.0.90 | 4.15 | 56.9 | 3.42 / 19.31 |
| system, 60 ships | task 93 | 3.75 | 59.6 | 3.18 / 15.45 |
| system, 60 ships | 0.0.90 | 3.50 | 56.2 | 3.20 / 14.79 |
| system, 60 ships | task 93 | 3.25 | 55.8 | 3.24 / 16.60 |
| orbit, 20 ships | 0.0.90 | 2.67 | 55.6 | 4.30 / 27.61 |
| orbit, 20 ships | task 93 | 2.46 | 54.8 | 4.24 / 31.33 |
| orbit, 20 ships | 0.0.90 | 2.20 | 56.0 | 4.25 / 21.63 |
| orbit, 20 ships | task 93 | 1.94 | 53.5 | 4.10 / 20.33 |

No difference beyond the noise: the frame time is not bound by the ships' triangles.

## Frame-rate re-check (task 108)

Owner report (2026-11-20), Sandbox preset (universe, `map.many_worlds`, age 13, rich-start script,
`--speed 500`, window about 1920×1080) on the integrated HD 530: "choppy picture" and "clicks react
late". Everything below is on the integrated GPU (no PRIME offload).

```sh
env -u WAYLAND_DISPLAY target/release/rise-of-man --universe --style map.many_worlds --players 1 \
  --ai none --age 13 --seed 1 --script <rich start + clicks> --speed 500 --size 1920x1080 --bench 20
```

The click script is `rich-start-13-galactic-script.txt` plus a left click every 40 ticks,
alternately on the city tower and on open ground (`cam click`, through the window's input path).
"Before" is 0.0.109 with only the new measurements added (the bench lines below); "after" is this
task's build. Runs alternate before/after, one at a time, under
`systemd-run --user --scope -p MemoryMax=6G -p MemorySwapMax=0`.

### New measurements in `--bench`

- `bench-detail sim_ticks_measured= sim_tps_measured= snapshots_per_second= peak_rss_mb=`:
  simulation ticks and entity snapshots during the measurement, peak resident memory.
- `bench-detail clicks= click_ms_avg= click_ms_max= click_stages_ms=a/b/c/d`: click-to-response
  time (`rom-render/src/response.rs`): from the button press reaching the bridge to the first frame
  the window shows that was drawn after the simulation answered the pick (selection or order).
  One press is followed at a time. Stages: press → pick sent by the render thread; → first frame
  started with the answer published; → that frame read back and published; → shown by the window.
  `click_ms_max` is the worst since start.
- `bench-detail cpu_pct process= <thread>=…`: CPU per thread name over the measurement
  (`/proc/self/task`; `rom-sim`, `rom-render`, the GPUI main thread named like the binary).

### What limited the picture

1. **Frames drawn that nobody saw.** The render thread draws offscreen with no swap, so nothing
   throttled it to the GPU: it queued frames at 60 Hz while the GPU finished about 45, and the
   three-buffer read-back skipped the frames it had no buffer for. At 1882×1043 the window showed
   21–23 of 58–59 rendered frames, and a frame reached the screen ~175 ms after it was started
   (stage c), most of the 212 ms click time. Fix (kept from the stopped first attempt, with
   numbers): before drawing, when every pack buffer holds a read in flight, map the oldest, waiting
   for its fence (`AsyncReadback::make_room`); and two pack buffers instead of three. The read-back
   work this does counts in `frame_ms`/`readback_ms`; the wait does not.
2. **GPU-bound at 1920×1080.** With the pacing the render rate is the GPU's. Temporary GL timestamp
   queries (removed after) split one 1882×1043 Sandbox frame, speed 100, before the fixes below:

   | Pass | GPU ms |
   | --- | --- |
   | terrain (materials ~4.2, shadow lookups ~2.0, rest ~1.8) | 8.0 |
   | feature objects (forests, rocks) | 4.7 |
   | grass | 2.6 |
   | read-back (`glReadPixels` 1.9–2.1, alpha clear 0.45) | 2.4 |
   | sky | 1.2 |
   | shadow map | 0.7 |
   | HUD | 0.7 |
   | everything else | 1.7 |
   | **total** | **22.0** |

   Fixes that leave the picture as it was:
   - **No world under the opaque HUD.** The frame artwork (top bar and bottom panels) covers about a
     third of the screen with fully opaque pixels. Their rectangles are found once per HUD bake from
     the artwork's alpha (runs of opaque pixels, 1.5 source pixels in from every edge), and before
     the sky and the globe are drawn the depth buffer is filled at the nearest depth there
     (`cover_depth`), so the depth test rejects those fragments before their programs run.
     Terrain 8.0 → 6.4 ms, grass 2.7 → 2.1, sky 1.2 → 1.0; screenshots before and after differ in
     170 of 1.96 M pixels (animation), none at the panel edges.
   - **Feature objects at full detail test depth first.** `object.fs` ends with the screen-door
     fade's `discard`, which keeps the driver from testing depth before the fragment program even
     when nothing fades. At full detail (the usual case; the hash is always below 1 there, so the
     fade never discards) a second program without that statement and with
     `layout(early_fragment_tests)` draws them: 5.0 → 3.8 ms.
   - Tried and dropped: drawing the objects before the ground (the ground under trees is then
     not shaded, but the trees behind hills are: 22.0 → 24.9 ms), and grass before the ground (no
     change).
3. **Game speed cost render time through the fog.** At speed 500 the simulation published 100
   snapshots a second and a visibility map every 5 ticks (20 a second); each map cost the render
   thread ~12 ms (minimap recolour 7.4, chunk scan 3.2, shades 1.3), 5 ms a frame on average,
   even though the map had not changed. Fixes: the render thread skips a map equal to the one its
   shades came from (until the ground is drawn anew); the simulation sends a map at most every
   200 ms of real time (still every 5 ticks at speed 100, where 5 ticks take 250 ms); and it
   publishes once per loop turn, after the turn's last tick, at most once per 16 ms (a click is
   answered at once) — per-tick duties (notices, autosave, script, journal, orders) stay per tick.
   Turns ran one tick each at 500 %, so the 16 ms bound is what halves the snapshots (100 → 53 a
   second). `Motions::prune` no longer relies on seeing a multiple of 200 ticks.
4. **Shadow map with many units (earlier attempt, kept with a change).** With 400 units fighting
   the shadow pass took 5.7 ms: the terrain and forests were drawn again whenever a unit moved.
   The casters that stay put get a map of their own, copied before the moving casters are drawn
   over it, but only once the light fit has held for three maps in a row (with the light turning
   at speed 500 the copy only added work: 2.1 → 4.7 ms without that rule). Battle (200 v 200,
   `--sun fixed`, 1600×900): shadow 5.7 → 4.6 ms, 46.8 → 48.5 fps.

### Grass reach and the rendering detail setting

The owner asked for grass visible from farther, then (same day) for the reach to sit under a
**rendering detail** setting with four levels, "the easy way to assess performance". The setting
(`--detail <min|medium|max|ultra>`, the launcher's Advanced form and its settings file
`launcher.json`, and a row of four buttons in the in-game menu that applies at once) sets the
grass's `Reach` (`rom-render/src/grass.rs`). Past the *coarse* distance the blades thin by the
square of the distance's ratio to it and are that ratio wider, and they draw from a budget of
their own, so the far ring never thins the near blades. `medium` is the reach before the task and
the default.

| Level | LOD | full height to | gone at | coarse past | budget near / far (candidates) |
| --- | --- | --- | --- | --- | --- |
| min | 12 | 20 | 28 | — | 15 000 / 0 |
| medium (default, as before) | 16 | 32 | 42 | — | 30 000 / 0 |
| max | 16 | 48 | 64 (1.52×) | 32 | 30 000 / 12 000 |
| ultra | 16 | 64 | 84 (2×) | 32 | 30 000 / 20 000 |

Only the grass follows the setting: it is the knob the measurements tied to distance. The feature
objects (3.8 ms) and the terrain material distance were not measured against their distances in
this task and keep theirs.

Cost of each level, Sandbox (age 13, rich start, speed 100), the home planet at the default zoom
(z0) and one wheel step out (z1), 15 s each, one run, after this task's other fixes. "Rendered" is
what the GPU sustains (the render thread waits for it), so 1000 / rendered is the GPU frame time;
"shown" is what the window presented:

| Size | Zoom | Level | Shown fps | Rendered fps | GPU ms (1000/rendered) | Frame ms avg / max (render thread) | Load |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1280x800 | z0 | min | 56.2 | 60.0 | 16.7 | 3.16 / 8.20 | 2.02 |
| 1280x800 | z0 | medium | 58.2 | 60.0 | 16.7 | 2.94 / 10.13 | 1.58 |
| 1280x800 | z0 | max | 56.9 | 60.0 | 16.7 | 3.07 / 8.73 | 1.59 |
| 1280x800 | z0 | ultra | 57.1 | 60.0 | 16.7 | 2.87 / 8.52 | 1.20 |
| 1280x800 | z1 | min | 56.7 | 60.0 | 16.7 | 2.73 / 8.34 | 0.86 |
| 1280x800 | z1 | medium | 57.2 | 60.0 | 16.7 | 2.75 / 9.93 | 0.74 |
| 1280x800 | z1 | max | 55.6 | 60.0 | 16.7 | 3.29 / 16.54 | 0.59 |
| 1280x800 | z1 | ultra | 57.7 | 60.0 | 16.7 | 3.06 / 9.54 | 0.46 |
| 1920x1080 | z0 | min | 53.2 | 59.6 | 16.8 | 4.56 / 19.06 | 0.60 |
| 1920x1080 | z0 | medium | 49.9 | 53.9 | 18.6 | 4.94 / 13.37 | 1.04 |
| 1920x1080 | z0 | max | 49.2 | 51.8 | 19.3 | 4.68 / 12.39 | 1.12 |
| 1920x1080 | z0 | ultra | 48.6 | 50.3 | 19.9 | 4.80 / 15.92 | 0.93 |
| 1920x1080 | z1 | min | 50.3 | 60.0 | 16.7 | 5.04 / 16.91 | 0.80 |
| 1920x1080 | z1 | medium | 53.0 | 59.9 | 16.7 | 4.29 / 13.01 | 0.63 |
| 1920x1080 | z1 | max | 53.1 | 59.9 | 16.7 | 4.53 / 11.42 | 0.73 |
| 1920x1080 | z1 | ultra | 50.7 | 58.9 | 17.0 | 4.76 / 19.66 | 0.79 |

Rendered frames are capped at 60. At 1280×800 every level keeps 60 rendered frames (the GPU has
room), so the level does not show there. At 1920×1080, default zoom, the GPU frame grows from
16.8 ms (min) to 18.6 (medium), 19.3 (max) and 19.9 (ultra); one step out, where medium draws few
blades, all levels but ultra keep 60. Shown fps varies by ±3 between single runs (presentation
side), more than the step between neighbouring levels. Grass candidates measured during
development (1882×1043, default zoom / one step out / two steps out): medium 29 700 (density
cut to 238 of 480 by the budget) / 15 300 / none; max 38 500 (near blades at the full 480) /
11 900 / 12 000. Screenshot pair (1280×800, default zoom, medium then max):
`docs/img/108-grass-before.png`, `docs/img/108-grass-after.png`: with max the blades reach past
the city, where medium shows the plain ground material.

### Owner scenario: before and after

Sandbox with clicks (above), 20 s each, alternating before/after, two rounds; no other `rise-of-man` process ran. Click times: presses followed / average / worst ms. Sim CPU is the `rom-sim` thread's share of a core.

| Size | Speed | Build | Load | Shown fps | Rendered fps | Frame ms avg / max | Sim ticks/s | Snapshots/s | Clicks n / avg / max ms | Sim CPU % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1280x800 | 100 | before | 0.91 | 59.5 | 59.9 | 3.66 / 21.79 | 20.0 | 20.0 | 10 / 101.0 / 108.3 | 5 |
| 1280x800 | 100 | after | 1.08 | 56.8 | 60.0 | 3.01 / 12.94 | 20.0 | 20.0 | 10 / 56.6 / 64.2 | 5 |
| 1280x800 | 100 | before | 1.24 | 55.8 | 59.9 | 4.25 / 21.16 | 20.0 | 20.0 | 10 / 93.4 / 98.9 | 5 |
| 1280x800 | 100 | after | 1.19 | 57.9 | 60.0 | 3.34 / 11.20 | 20.0 | 20.0 | 10 / 53.8 / 59.4 | 5 |
| 1280x800 | 500 | before | 0.85 | 45.9 | 59.6 | 7.20 / 23.77 | 100.0 | 100.0 | 50 / 94.1 / 120.5 | 21 |
| 1280x800 | 500 | after | 1.67 | 52.8 | 60.0 | 3.93 / 12.20 | 100.0 | 54.0 | 50 / 65.2 / 88.4 | 10 |
| 1280x800 | 500 | before | 1.08 | 45.1 | 59.6 | 7.26 / 29.12 | 100.0 | 100.0 | 50 / 91.9 / 118.5 | 21 |
| 1280x800 | 500 | after | 0.97 | 53.5 | 60.0 | 3.94 / 11.91 | 100.0 | 53.3 | 50 / 54.3 / 76.7 | 11 |
| 1882x1043 | 100 | before | 1.16 | 21.2 | 59.5 | 3.78 / 159.91 | 20.0 | 20.0 | 10 / 210.2 / 251.4 | 5 |
| 1882x1043 | 100 | after | 1.10 | 49.3 | 54.4 | 5.37 / 37.70 | 20.0 | 20.0 | 10 / 82.8 / 97.1 | 5 |
| 1882x1043 | 100 | before | 0.75 | 21.4 | 59.5 | 4.12 / 144.05 | 20.0 | 20.0 | 10 / 225.9 / 257.3 | 5 |
| 1882x1043 | 100 | after | 0.77 | 49.5 | 53.8 | 5.13 / 39.08 | 20.0 | 20.0 | 10 / 70.9 / 76.3 | 5 |
| 1882x1043 | 500 | before | 1.16 | 23.0 | 58.5 | 7.76 / 115.22 | 100.0 | 100.0 | 50 / 204.3 / 290.1 | 21 |
| 1882x1043 | 500 | after | 1.31 | 49.1 | 52.6 | 5.91 / 20.79 | 100.0 | 53.0 | 50 / 79.5 / 108.5 | 11 |
| 1882x1043 | 500 | before | 0.90 | 22.6 | 58.5 | 7.80 / 31.30 | 100.0 | 100.0 | 51 / 204.3 / 289.4 | 21 |
| 1882x1043 | 500 | after | 1.00 | 49.3 | 52.1 | 6.28 / 17.44 | 100.0 | 53.2 | 50 / 78.8 / 98.1 | 11 |

- **Shown frames at 1882×1043**: 21–23 → 49–50 at both speeds. The ≥ 55 target is not met: the
  GPU sustains 52–54 frames (18.5–19 ms) and the window shows 49–50 of them; the rest of the GPU
  work is fill-rate (terrain materials and shadow lookups, feature objects, grass) that the
  picture needs as it is; see "Follow-up" below.
- **Clicks**: 204–226 ms → 71–83 ms at 1882×1043, 92–101 → 54–65 ms at 1280×800. After, a click
  at 1882×1043 splits roughly 9 ms to the pick, 18 ms to the next frame started with the answer,
  37–45 ms drawing and reading back (two frames of the GPU), 11–14 ms until the window shows it.
- **Speed 500 against 100** (render frame time): before +3.5 to +4 ms; after +0.6 to +0.9 ms
  (target ≤ 2 ms). Snapshots 100 → 53 a second; the simulation thread's CPU at 500 % 21–23 % →
  10–11 % of a core. Speed 300 was not benched separately.
- `frame_ms` after is higher at 1882×1043 than before at speed 100 although the frames are the
  same: before, only the ~21 frames a second that were read back paid for the row copy; now every
  drawn frame is read back (about 1.5 ms of copying a frame at that size).

### Every view since 0.0.102: before and after

One 15 s run each (20 s for the camera script, battle and orbit; 12 s for the space views), alternating before/after, 1882×1043 unless noted, speed 100, seed 1; load 0.6–2.0, no other `rise-of-man` process. Ages: `--universe --style map.many_worlds --players 1 --ai none --age N`, default zoom and `--zoom-out`. Presets: `--universe --style map.<key> --players 2 --ai none --age 13 --view system|starmap` (seed 1 for every style, not the largest of each: deviation). Sim ticks/s were at the set rate in every run (the orbit script sets its own speed: 31 ticks/s).

| View | Build | Shown fps | Rendered fps | Frame ms avg / max | Peak RSS MB |
| --- | --- | --- | --- | --- | --- |
| camera script (108), 1280×800 | before | 55.5 | 60.0 | 3.82 / 18.82 | 409 |
| camera script (108), 1280×800 | after | 57.3 | 60.0 | 2.88 / 9.92 | 410 |
| camera script (108), 1882×1043 | before | 41.0 | 58.7 | 4.83 / 189.34 | 434 |
| camera script (108), 1882×1043 | after | 55.1 | 56.1 | 4.69 / 22.64 | 433 |
| age0 default | before | 24.1 | 59.9 | 3.58 / 22.72 | 428 |
| age0 default | after | 50.0 | 55.5 | 4.34 / 12.93 | 436 |
| age0 zoomed out | before | 25.6 | 59.9 | 3.64 / 22.54 | 425 |
| age0 zoomed out | after | 49.5 | 57.1 | 4.92 / 18.98 | 433 |
| age4 default | before | 22.1 | 59.9 | 3.74 / 21.45 | 422 |
| age4 default | after | 49.4 | 54.7 | 4.98 / 14.62 | 436 |
| age4 zoomed out | before | 40.1 | 59.9 | 4.49 / 23.28 | 434 |
| age4 zoomed out | after | 54.2 | 60.0 | 4.22 / 13.50 | 433 |
| age7 default | before | 20.3 | 60.0 | 3.34 / 18.51 | 424 |
| age7 default | after | 50.1 | 54.3 | 4.85 / 14.34 | 434 |
| age7 zoomed out | before | 54.5 | 60.0 | 3.89 / 19.42 | 430 |
| age7 zoomed out | after | 57.3 | 60.0 | 3.52 / 14.57 | 431 |
| age10 default | before | 20.9 | 59.9 | 3.87 / 22.40 | 433 |
| age10 default | after | 49.7 | 54.3 | 5.00 / 12.14 | 434 |
| age10 zoomed out | before | 54.7 | 60.0 | 4.86 / 20.40 | 433 |
| age10 zoomed out | after | 57.7 | 60.0 | 3.74 / 8.11 | 432 |
| age13 default | before | 21.0 | 59.9 | 3.81 / 25.97 | 425 |
| age13 default | after | 49.6 | 54.3 | 4.87 / 12.87 | 433 |
| age13 zoomed out | before | 54.9 | 60.0 | 3.51 / 15.69 | 433 |
| age13 zoomed out | after | 58.5 | 60.0 | 3.11 / 8.61 | 431 |
| battle 200 v 200, projectiles on, `--sun fixed` (95) | before | 10.7 | 48.0 | 10.45 / 106.99 | 433 |
| battle 200 v 200, projectiles on, `--sun fixed` (95) | after | 40.6 | 40.7 | 8.15 / 16.79 | 437 |
| orbit, 20 ships (72) | before | 55.4 | 59.9 | 4.71 / 23.07 | 442 |
| orbit, 20 ships (72) | after | 57.9 | 60.0 | 3.53 / 9.13 | 441 |
| Planet system | before | 54.9 | 60.0 | 5.82 / 17.24 | 423 |
| Planet system | after | 58.7 | 60.0 | 5.32 / 13.19 | 424 |
| Planet starmap | before | 59.7 | 60.0 | 3.52 / 13.68 | 422 |
| Planet starmap | after | 58.7 | 60.0 | 2.74 / 8.23 | 424 |
| Solar system | before | 55.1 | 60.0 | 5.76 / 19.10 | 426 |
| Solar system | after | 56.4 | 60.0 | 5.41 / 9.64 | 431 |
| Solar starmap | before | 59.7 | 60.0 | 3.71 / 14.07 | 431 |
| Solar starmap | after | 59.9 | 60.0 | 3.32 / 8.86 | 429 |
| Cluster system | before | 57.6 | 60.0 | 5.02 / 15.51 | 437 |
| Cluster system | after | 58.2 | 60.0 | 4.10 / 14.84 | 435 |
| Cluster starmap | before | 55.9 | 60.0 | 3.55 / 14.55 | 437 |
| Cluster starmap | after | 59.1 | 60.0 | 3.06 / 7.55 | 440 |
| Sector system | before | 59.3 | 60.0 | 7.70 / 18.62 | 530 |
| Sector system | after | 56.6 | 60.0 | 7.64 / 14.60 | 530 |
| Sector starmap | before | 53.9 | 60.1 | 4.04 / 14.85 | 531 |
| Sector starmap | after | 57.2 | 60.0 | 3.30 / 10.81 | 532 |
| Quadrant system | before | 52.4 | 59.1 | 12.87 / 27.63 | 591 |
| Quadrant system | after | 49.2 | 59.9 | 12.22 / 20.94 | 593 |
| Quadrant starmap | before | 54.3 | 60.0 | 4.01 / 16.73 | 592 |
| Quadrant starmap | after | 57.4 | 60.0 | 3.36 / 11.40 | 592 |
| Galaxy system | before | 59.8 | 60.0 | 5.18 / 17.48 | 1164 |
| Galaxy system | after | 59.8 | 60.0 | 4.80 / 11.00 | 1164 |
| Galaxy starmap | before | 58.2 | 59.9 | 10.24 / 20.70 | 1164 |
| Galaxy starmap | after | 55.4 | 60.0 | 8.75 / 13.28 | 1161 |

- Every home-planet view at the default zoom went from 20–25 to 49–50 shown frames at 1882×1043
  (the pacing; the GPU then sustains 54–56). Zoomed out (less ground on screen) they reach 54–59.
- The worst case is the battle: 10.7 → 40.6 shown (GPU-bound, 40.7 rendered; frame max 107 →
  17 ms). Under 45: the 400 units' shadow pass and their models; see the follow-up.
- Star map and system views were already near 60 and stay there (the system views are CPU-bound
  on the render thread in Quadrant, 12 ms a frame, unchanged; Quadrant system shown 52.4 → 49.2
  in one run each, within the presentation noise seen elsewhere; rendered 59.1 → 59.9).
- No frame over 100 ms after (before: 107–189 ms at 1882×1043); simulation at its rate everywhere.

### Kept and dropped from the stopped first attempt

Kept, each with the numbers above: `make_room` (wait for a free pack buffer before drawing,
`readback.rs`, `worker.rs`), the bench's simulation-rate and peak-memory lines (`bench.rs`), and
the shadow map of the casters that stay put (`scene/shadow.rs`), changed to wait for a steady fit.
Nothing was dropped outright; the first attempt's three buffers became two.

### Follow-up (not done here)

At 1882×1043 the HD 530 needs about 18.5 ms of GPU work for the Sandbox frame after these fixes
(terrain 6.0: materials ~4, shadow lookups ~2; feature objects 3.8; grass 2.2–2.9; read-back 2.4;
sky 1.0; HUD 0.8; shadow map 0.6, 2.4 while the light turns at speed 500), so the window shows
49–50 frames, not 55. What would reach 55 changes the picture or the shadow format: cheaper
terrain material programs far from the eye, a single-channel shadow map read with `textureGather`
(4 fetches instead of 9), or a lower render scale on this GPU (`--scale 75` renders 60). The battle
needs a cheaper shadow pass for many units (instanced caster boxes instead of models, or every
other frame). These are follow-up candidates, measured here, not changed.

## GPU memory (task 121)

Owner report (issue #4, 0.0.113 on Windows 11, NVIDIA RTX A2000 Laptop GPU with 4 GB): dedicated
GPU memory climbs for as long as a match runs, at every detail level, never comes down, and the
game turns sluggish once the 4 GB are full (3D load only 15 %).

### How to measure

`--bench` now counts the live GL objects of the render thread's context every 10 s
(`rom-render/src/gpu_census.rs`). The count asks GL itself (`glIsTexture`, `glIsBuffer`,
`glIsFramebuffer`, `glIsRenderbuffer`, `glIsVertexArray`, `glIsProgram` over the names in use, with
the sizes of textures, renderbuffers and buffers), so it sees every object whoever made it. Each
count is a `bench-gpu seconds=…` line on stderr; the `bench` summary line ends with the last count
and a verdict:

```
… gpu_textures=14 gpu_texture_mb=97 gpu_framebuffers=4 gpu_renderbuffers=4 gpu_buffers=5406
  gpu_buffer_mb=68 gpu_vertex_arrays=889 gpu_programs=17 gpu_free_vram_mb=1879
  gpu_instanced_draws=3 gpu_count_us=1942 gpu_grown_mb=0 gpu_flat=yes
```

`gpu_flat=yes` means that from the count after the warm-up (a quarter of the run, at least 30 s)
to the end, the objects' memory grew by at most 16 MiB and no kind of object by more than a tenth
plus eight. `gpu_free_vram_mb` is the driver's free video memory where it reports it
(`GL_NVX_gpu_memory_info`, NVIDIA; 0 elsewhere). `gpu_instanced_draws` is the number of instanced
draws in a frame; `gpu_count_us` what the count cost the render thread (1.6–2.5 ms on the GTX
960M, once every 10 s). `RenderBridge::count_gpu_objects(seconds)` turns the count on for other
uses.

The process's own GPU memory, sampled once a second beside a self-ending run:

```sh
# NVIDIA (GTX 960M here, PRIME offload): the raylib GL context
__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia env -u WAYLAND_DISPLAY \
  target/release/rise-of-man --universe --style map.many_worlds --players 3 --seed 1 \
  --speed 200 --size 1600x900 --script <session> --bench 720 &
nvidia-smi pmon -s m -c 1          # column fb (MB) of the process, once a second
# Intel (i915): every DRM client of the process, the GPUI window's Vulkan device included
cat /proc/<pid>/fdinfo/* | awk '/drm-total-system0/ {s+=$2} END {print s/1024}'
```

Under PRIME offload only raylib's GL context runs on the NVIDIA card; the GPUI window keeps the
Intel GPU (its Vulkan device has the i915 render node open), so `nvidia-smi` shows the renderer
and the i915 `fdinfo` shows the window. `nvidia-smi --query-compute-apps` lists nothing for a
graphics process; `pmon -s m` does.

The session (`docs/tasks/121-session-script.txt`, 12 minutes, game speed 200 so 24 game minutes):
galactic age, two computer players; turn and pan drags, wheel out to orbit, the system and the star
map, fly the star map with the arrow keys, wheel back in toward another body, home, focus, palette
tabs and a palette button, box-select the builders. The in-match menu is left out: it holds the
simulation and the script with it.

### Before and after

GTX 960M (2 GB), 1600×900, one run at a time beside the owner's own game (Intel GPU):

| Build | t = 60 s | Range after 60 s | End (t = 720 s) | Frames shown |
| --- | --- | --- | --- | --- |
| before (0.0.113 code) | 193 MB | 110–193 MB | 110 MB | 42 177, 58.6 fps |
| after (this task) | 195 MB | 112–195 MB | 112 MB | 41 129, 57.1 fps |

Both curves are flat: they step down (193 → 171 → 110 MB) when the view leaves the planet for
space and never climb. The live GL objects are as flat, before (`--bench` count added to the old
drawing code, Intel GPU, the same session) and after (NVIDIA): 13–14 textures (93–97 MB), 3–4 render
targets, 5 277 buffers before and 5 406 after (the 129 more are the reused instance buffers),
889 vertex arrays and 17 programs at every count from 10 s to 720 s. On the Intel GPU (i915 `fdinfo`)
the GPUI window alone held 147 MB at every sample after 60 s of the NVIDIA runs, and window and
renderer together 1 064–1 077 MB over the 12 minutes of the Intel run (a few samples read 148 MB,
missing the renderer's client).

A second session (`docs/tasks/121-switch-script.txt`) switched planets every half minute
(`map.solar`, `visit planet1`/`planet2` and home: each switch rebakes the surface, its minimap and
the system frames), fought task 95's two-hundred-unit battle with projectiles and quick-saved every
third cycle, ten minutes at speed 200 on the GTX 960M with this task's build: 36 surface bakes, GPU
memory 188 MB at 60 s, 101–188 MB after, 181 MB at the end (`gpu_flat=yes`, grown 0 MB). Every count
from 10 s to 600 s: 13 textures, 5 423–5 456 buffers, 888–895 vertex arrays (the higher counts while
another planet is loaded), 17 programs; nothing left behind by a switch.

### What leaked

Nothing grows on Linux, on either GPU, with the 0.0.113 code: the leak the owner sees on Windows
does not reproduce here, so what follows is what the code did that a Windows driver can turn into
one, and the counts that would show it there.

- **Instanced draws made and deleted a buffer each, and left it attached.** Every instanced draw
  went through raylib's `DrawMeshInstanced` (`raylib-sys` `rmodels.c`, `DrawMeshInstanced`): it
  makes a vertex buffer for the transforms, points the mesh's vertex array at it, unbinds the array
  and deletes the buffer. A buffer deleted while attached to a vertex array that is not bound is not
  freed (OpenGL keeps it for that array) until the array is pointed elsewhere at the mesh's next
  instanced draw. Each frame makes such draws for the feature objects, the models near and far,
  their shadow passes and the ships: up to about 40 a frame in these sessions, so up to 2 400
  buffers created and deleted a second, each up to the instances in view times 64 bytes (the
  planet of these sessions holds 18 679 feature instances). Freeing them is left to the driver's
  deferred deletion; on a driver that collects them late (or only at a swap, which the hidden
  renderer window never does) memory climbs with the frame rate and does not depend on the detail
  level, as reported.
- **The GPUI frame atlas** (gpui 0.2.2, not changed): every frame shown is a new picture uploaded
  into a new atlas texture, freed two frames later (`world_view.rs`, `retire_image`). This works;
  the window's memory stays flat. Its one weakness: gpui never frees a tile, only a whole texture
  once nothing in it is used, so a small picture drawn for the first time while a frame's texture
  has room below the frame (frames under 1024 pixels high get a 1024-high texture) keeps that
  texture alive for good. Opening the in-match menu the first time added 4 MB to the window here
  that stayed (its pictures entering the atlas, perhaps into such a texture); it does not grow
  with time.

### Fix

`rom-render/src/instancing.rs` draws instances exactly as raylib does (same uniforms, attributes,
texture slots and state left behind; screenshots of a city, a forest edge and the open ground match
the old drawing within the run-to-run noise) but from instance buffers made once and rewritten: the
draws of a frame take buffers in turn from one of three sets, the sets in turn by frame, so a
buffer is not written while the GPU may still read it, and a buffer only grows (powers of two,
`glBufferData` on the same buffer), never is deleted during the match. The render thread releases
them before its GL context closes. No GL object is made or deleted per frame any more; the count
of buffers is flat by construction (unit tests in `instancing.rs`), and `gpu_instanced_draws`
shows how many per-frame buffers this replaces.

### Not verified

Windows: no Windows machine here. On the owner's machine, `--bench 600` with the same kind of
session should print `gpu_flat=yes` and Task Manager's dedicated GPU memory should level off
after the first minute; if it still climbs while the counts stay flat, the growth is outside the
objects the game makes (the driver, or the GPUI DirectX window), and the next step is to watch
which of the two GPUs grows (the window may run on the Intel GPU and the renderer on the NVIDIA one,
as here) and to measure a window under 1024 pixels high against a taller one.
