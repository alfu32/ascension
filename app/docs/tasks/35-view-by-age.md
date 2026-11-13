# Task 35 — View range by age, isometric camera tilt, night on the minimap, Library grid

Crates: `rom-render`, `rom-app`. Data: one column in `app/db/schema.sql` and
its values in `app/db/seed.sql` (hand-written `age` rows); `rom-rules` loads it.

## Owner's report (2026-11-13, with screenshots)

1. The player may zoom out **as a function of age**: each age gives a larger
   view of the terrain. Age 0: about the home city and its border (roughly
   the view at the default home distance today). Age 7: about a third of the
   planet's face. Before the space age (age 9): the whole globe, as today.
2. At low zoom the camera must keep the **horizon higher**: a constant
   inclination of about 60° from the vertical over the lower zoom levels (like
   an isometric view), so the player sees more of the domain; only toward the
   globe view does it ease to straight down.
3. The **minimap** shows the night side of the planet: the terminator as the
   sine-shaped mask on the equirectangular map, darkening the side away from
   the directional light that represents the sun (it moves as that light
   orbits the planet orb).
4. The **Library** shows the full research grid again (task 30 cut it to one
   button per track): column 1 the **age advance**, columns 2–5 **Military,
   Civic, Commerce, Science**; each row is an age. Researched items stay
   visible with the gold check badge; the age advance is greyscale (locked)
   until its row is done. Page arrows move three ages per page; opening the
   Library shows the page holding the player's current age.

The data side of item 4 (positions and prerequisites) is done by the
orchestrator in the generator before this task starts: `production.ui_col` =
0 age, 1 military, 2 civic, 3 commerce, 4 science; `ui_row` = age mod 3;
`ui_page` = `research`, `research_2`, … per three ages; an age advance needs all
four track levels of its row.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/world-model.md` (zoom levels, cap before the space age), `app/docs/hud.md`.
- `app/crates/rom-render/src/orbit.rs` (`STREET_TILT`, `tilt`, `max_distance`),
  `zoom.rs` (`ZoomCamera`, `Level`, `l_cap`, `set_cap`), `worker.rs`
  (`zoom_cap`, `current_cap`, minimap draw), `hud_gpu.rs` (`MinimapGpu`),
  `hud_minimap.rs`, `sun.rs`, `graticule.rs` (minimap lines).
- `app/crates/rom-app/src/universe_view.rs` (`max_level` from the age's map
  scale), `sim_thread.rs`, `selection.rs` (`palette`, the Library filter added
  by task 30, page handling).
- `app/crates/rom-rules/src/model.rs` (`Age`), `load.rs`.

## Build

1. **Schema.** `age.view_range INTEGER NOT NULL DEFAULT 0` — the largest
   ground half-width the camera may show, in milli-tiles; 0 = the whole globe.
   Values: ancient 60000, classical 80000, medieval 100000, gunpowder 130000,
   enlightenment 160000, industrial 200000, modern 250000, information 320000,
   automation 450000, fusion and later 0. Note it in `database.md`.
2. **Zoom cap by age.** The app passes the player's current age's
   `view_range` to the renderer with the zoom cap (atomic like `zoom_cap`).
   The orbit and zoom cameras clamp the eye distance so the visible ground
   half-width at the current tilt does not exceed it; 0 keeps today's globe
   cap; the space levels keep their own caps. Raising the age widens the limit
   at once; lowering is not needed. Flat maps: clamp the flat camera's height
   the same way.
3. **Tilt.** Over the zoom range up to `ISO_UNTIL` (a named constant, about the
   distance at which age 4's view range is reached), the tilt is a constant
   `ISO_TILT` ≈ 60° from the vertical; beyond it the tilt eases to 0 at the
   largest distance, as today. The closest zoom may tilt a little more (≤ 70°).
   Picking, edge-pan, drag-pan and the shadow fit must keep working (their
   tests must pass; add one for the new tilt curve).
4. **Minimap night.** Darken minimap pixels whose surface normal faces away from
   the light direction (smooth over a few degrees, night ≈ 45 % brightness),
   updated when the light moves by more than about one minimap pixel. Works on the
   sphere (equirectangular) and is skipped on flat maps and in space.
5. **Library grid.** Remove task 30's per-track filter for producers whose
   research has `track`/`kind = age` rows; show the rows by `ui_page`/`ui_row`/`ui_col`
   with the states from task 30 (`Done` badge, `InProgress`, `Unaffordable`,
   `Locked` greyscale). Default page = the one containing the player's current
   age's row. Other producers are unchanged. Update `content.md` §7.2.

## Tests to add (no window)

- `rom-rules`: `age_view_range_loads`.
- `rom-render`: `view_range_caps_the_eye_distance`, `zero_view_range_keeps_the_globe_cap`,
  `tilt_is_constant_over_the_low_zoom_range`, `minimap_night_follows_the_light`.
- `rom-app`: `library_shows_the_full_grid_with_done_badges`,
  `library_opens_on_the_current_age_page`, `zoom_cap_follows_the_age`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

The orchestrator then runs (write any script you need under `docs/tasks/35-*.txt`):

```
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --zoom-out --screenshot target/view-age0.png --after 2
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --age 7 --zoom-out --screenshot target/view-age7.png --after 2
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --age 9 --zoom-out --screenshot target/view-age9.png --after 2
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --screenshot target/view-home.png --after 2
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 1 --size 1672x941 --script docs/tasks/35-library-script.txt --screenshot target/library-grid.png --after 12
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10
```

Add `--zoom-out` (zoom to the cap at start) and `--age <ordinal>` (start the
player in that age — the simulation has no "set age" API yet; if it cannot be
done through orders, grant and research the age techs in a script, or report
it) to the app's arguments.

## Rules that matter most

- No branching on definition keys or names: view range from the `age` row,
  Library layout from `production` rows and `tech.track`/`kind`.
- `rom-render` does not depend on `rom-sim`/`rom-rules`; `rom-app` does not call raylib.
- Do not modify `rom-sim`, `rom-mapgen`, `rom-cli`. Missing simulation API →
  work around it and list it.
- A parallel task (34) edits terrain and feature drawing in `rom-render`
  (`globe.rs` shading, `scene.rs` terrain passes, new material/feature files);
  keep to the camera, zoom, minimap, HUD and palette paths and expect a merge.
- Never use `xdotool`, `wmctrl` or similar. `env -u WAYLAND_DISPLAY`. No `sudo`.

## Finish

Commit on the current branch (stage files by name) with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge. Final message: commands with real output, files changed,
deviations and why, anything you could not verify.
