# Task 30 — Palette states, research view, site outlines, city limit on screen, drag-pan

Crates: `rom-render`, `rom-app`. Assets: SVG badge glyphs if needed.

## Owner's reports (2026-11-13, after playing)

1. It is unclear where a city's limit is; with one city it was impossible to
   place a second one (data fixed in `0.0.33`: border radius 40 tiles, city
   radius 16, spacing 20). The city radius must be **visible**.
2. It is hard to tell which research is done or available, and which
   buildings or units can be made. Required states:
   - research **done**: shown with a badge (a check mark), not hidden;
   - available by requirements but **unaffordable**: darker/disabled, as now;
   - **locked** by technology or age: **greyscale** (desaturated), with the
     missing requirement in the help line;
   - the same three states for buildings and units (locked → greyscale,
     unaffordable → disabled).
3. When placing a building, show the **future building's contour** on the ground.

Plus, from the full content (`app/docs/content.md` §7.2): the Library shows one
button per research track (the lowest level not yet researched) and the next
age; obsolete unit steps are hidden.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/hud.md`; `app/docs/content.md` (§2 tracks and ages, §5 lines, §7 interface rules).
- `app/crates/rom-rules/src/lib.rs` (`is_unlocked`, `is_obsolete`, `is_available`,
  `cost_of_ramped`, `display_name`, `prerequisites`, `techs` with `track` and
  `level`), `model.rs` (`Tech`, `EntityType`, `Production`).
- `app/crates/rom-sim/src/` — `world.rs` (`check_placement`, `price`, players and
  techs), `economy.rs`, `research.rs`, `production.rs`, `territory.rs`, `lib.rs` (errors).
- `app/crates/rom-app/src/selection.rs` (`palette`, placement), `sim_thread.rs` (HUD snapshot),
  `hud.rs`; `app/crates/rom-render/src/hud_model.rs`, `hud.rs`, `hud_atlas.rs`,
  `overlay.rs`, `globe.rs`, `entities.rs`, `camera.rs`, `orbit.rs`, `worker.rs`.

## Build

1. **Button states.** `HudButton` gains `state: ButtonState` with
   `Ready`, `Unaffordable { missing: Vec<(icon, amount)> }`, `Locked { reason }`,
   `Done`, `InProgress { pct }`. The app computes it per button, in this order:
   researched and not repeatable → `Done`; queued or being researched → `InProgress`;
   not `is_available` for the faction, or not `is_unlocked` → `Locked` with the
   first unmet prerequisite's display name ("needs Classical Age", "needs
   Barracks", "needs Drill"); price above stock → `Unaffordable` listing what is
   short; else `Ready`. The renderer draws: `Done` normal with a gold check-mark
   badge in the corner (new SVG glyph); `InProgress` normal with a thin progress
   bar at the bottom; `Unaffordable` darkened (as today's disabled); `Locked`
   **desaturated to greyscale** and slightly darker (shader or CPU-recoloured atlas
   copy — your choice; keep it cheap). The help line on hover shows name, price
   and the reason for `Locked`/`Unaffordable`.
2. **Library palette** (and any producer with research): the RESEARCH tab shows,
   per track, only the lowest level not yet researched (ready, locked or
   unaffordable), then the next age advance, then other researchable techs at
   that building. A second row of small `Done` badges is not needed: done
   techs are not shown here. Add a **research overview** instead: clicking a new
   data tab "RESEARCH" (replace the inert WONDERS tab for now) fills the data view
   with the four tracks as rows of 13 small squares (done = gold, in progress =
   cyan, next = white outline, locked = grey) and the age pips below. Pure
   data in the snapshot; the renderer draws it.
3. **Unit palettes:** obsolete steps are hidden (`is_obsolete`); the newest
   unlocked step of each line is shown; a locked next step is **not** shown as
   a separate button (the line's current step is what you can train).
   Building pages: locked buildings are shown greyscale (so the player sees what
   exists), ordered by `ui_col`/`ui_row`, with page arrows.
4. **Placement contour.** While placing, draw on the ground, at the hovered tile:
   the footprint outline as a thick line (green/red by validity, as today's
   fill) **and** a translucent ghost box of the building's height in the owner
   colour. After the order is submitted, the **site** (an entity under
   construction) is already drawn by the entity renderer; make sure a fresh site
   shows a dashed outline on the ground plus the ghost, until construction
   starts, so the player sees where it will stand. Keep the sphere path working
   (conform to the curvature as the footprint does today).
5. **City limit on screen.** When a city is selected, or while placing a
   building that needs `in_city_radius`, draw each own city's **economic radius**
   as a dashed circle on the ground (white, thin), and the national border as
   today. When placing a city, also draw the `city.spacing` circle around every
   existing city (red dashed) so the player sees where the next city may go. On
   the minimap, draw the city radii as thin circles when a city is selected.
6. **Drag-pan.** Middle-button drag (and right-button drag without a selection
   on the flat map) pans the camera at ground speed (the point under the cursor
   stays under the cursor on the flat map; on the globe, great-circle drag).
   Middle-drag rotation moves to Shift+middle-drag. Document the controls in
   `app/docs/hud.md` ("Controls").
7. **Event line:** when a submit is rejected for a placement or research reason
   the message names the rule in the player's words ("Too close to another city
   (20 tiles)", "Outside your borders", "Needs Military level 1").

## Tests to add (no window, no GPU)

- `rom-app`: `button_states_follow_rules_and_stock` (one case per state),
  `library_shows_next_level_per_track_and_next_age`, `obsolete_steps_are_hidden`,
  `locked_buildings_are_shown_greyscale`, `research_overview_marks_done_next_and_locked`,
  `rejection_messages_name_the_rule`.
- `rom-render`: `greyscale_keeps_luminance`, `contour_and_ghost_follow_the_footprint`,
  `city_radius_circle_has_the_right_radius`, `drag_pan_keeps_the_point_under_the_cursor` (flat).

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 1 --size 1672x941 --script docs/tasks/30-palette-script.txt --screenshot target/palette-states.png --after 12
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 1 --size 1672x941 --script docs/tasks/30-city-script.txt --screenshot target/city-limits.png --after 12
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

Write the two scripts: the first selects the library after one track tech is
done and another is in progress, with low timber so one button is unaffordable
(use `grant`), and opens the RESEARCH data tab; the second selects the city and
enters city placement at a valid spot. Look at the screenshots: a done badge,
a progress bar, a greyscale locked button, a darkened unaffordable one, the
research overview; the city radius circle, the spacing circles, the contour
and ghost of the future city. Copy them to `app/docs/img/`. Bench at or above
55 fps; report the real number.

## Rules that matter most

- Thread ownership unchanged; `rom-render` may use `rom-geo` but not `gpui`,
  `rom-sim`, `rom-rules`, `rom-mapgen`; `rom-app` does not call raylib.
- No branching on definition keys or names outside script parsing (tracks come
  from `tech.track`/`level`, lines from `upgrades_from_id`, states from the rules API).
- Do not modify `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or
  the seed (a parallel task edits `rom-render` lighting files: `scene.rs`,
  `globe.rs` shading, `worker.rs` sun code — keep your edits to the HUD, overlay,
  pick and camera paths and expect a merge). Missing simulation API → work
  around it and list it.
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--screenshot`
  or `--bench`, or by ending the process you started. `env -u WAYLAND_DISPLAY`.
  No `sudo`. Stage files by name.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output, the bench line, what each screenshot
shows, files changed, missing simulation API, deviations and why, anything you could not verify.
