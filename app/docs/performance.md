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
| `frame_ms_avg` / `_max` | render thread, frame start → frame in the mailbox: draw calls, waiting for the GPU, read-back and conversion |
| `readback_ms_avg` / `_max` | read-back (`LoadImageFromTexture`, which first waits for the GPU to finish the frame) plus RGBA→BGRA and row flip |
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

`--sphere 192` draws a generated planet of 6 × 192² = 221,184 tiles on a globe of radius
122 tiles: 216 land chunks and 156 sea chunks (at most 32 × 32 tiles each, 372 meshes in
all), about 440,000 land triangles. Chunks wholly beyond the horizon or outside the view
pyramid are not drawn (`ChunkBounds::may_be_seen`). The simulation is not started in sphere
mode, so the numbers are rendering only (HUD on, 1280×800, scale 100, 10 s, seed 1).

```sh
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --bench 10 --orbit 0,10,12,30   # street
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --bench 10 --orbit 0,10,90,0    # mid
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --bench 10 --orbit 0,10,330,0   # globe
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
machine was busy with other tasks' builds and runs; the unchanged `--sphere 192 --demo`
bench measured in the same session is listed for comparison. The frame time is dominated
by the read-back in every row (11-15 ms), not by drawing.

| view | bench line |
| --- | --- |
| globe | `bench size=1280x800 scale=100 frames=521 fps=52.1 frame_ms_avg=13.31 frame_ms_max=42.65 readback_ms_avg=11.38 readback_ms_max=31.99` |
| system | `bench size=1280x800 scale=100 frames=466 fps=46.6 frame_ms_avg=17.32 frame_ms_max=70.81 readback_ms_avg=14.52 readback_ms_max=42.25` |
| star map | `bench size=1280x800 scale=100 frames=453 fps=45.3 frame_ms_avg=18.39 frame_ms_max=60.01 readback_ms_avg=15.54 readback_ms_max=37.50` |
| `--sphere 192 --demo` (reference) | `bench size=1280x800 scale=100 frames=532 fps=53.2 frame_ms_avg=12.18 frame_ms_max=45.41 readback_ms_avg=10.64 readback_ms_max=30.76` |

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
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --sun fixed --size 1280x800 --orbit 0,10,12,30 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --sun fixed --size 1280x800 --orbit 0,10,90,0 --bench 10
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --sun fixed --size 1280x800 --orbit 0,10,330,0 --bench 10
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
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --sun fixed --size 1280x720 --orbit 0,10,8,0 --no-hud --after 1 --screenshot target/sky-day.png
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --sun night --size 1280x720 --orbit 0,10,8,0 --no-hud --after 1 --screenshot target/sky-night.png
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
| `--sphere 192 --demo` (home view) | `bench size=1280x800 scale=100 frames=501 fps=50.1 frame_ms_avg=19.40 frame_ms_max=45.98 readback_ms_avg=17.08 readback_ms_max=28.17` |
| `--sphere 192 --demo --look-at ter.forest,8` (closest zoom over a forest edge) | `bench size=1280x800 scale=100 frames=501 fps=50.1 frame_ms_avg=19.34 frame_ms_max=44.80 readback_ms_avg=16.90 readback_ms_max=25.98` |
| `--demo` (flat) | `bench size=1280x800 scale=100 frames=566 fps=56.6 frame_ms_avg=13.82 frame_ms_max=26.81 readback_ms_avg=12.90 readback_ms_max=22.90` |
| parent commit, `--sphere 192 --demo`, alternating runs | 53.0 / 52.0 fps, frame 16.7 / 15.8 ms (new build in between: 51.5 / 47.2 fps, 19.0 / 20.4 ms) |

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
