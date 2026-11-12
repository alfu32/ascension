# Task 11 — HUD chrome, live top bar and minimap

Crates: `rom-render`, `rom-app`. Assets under `app/assets/hud/`.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task" and "Naming".
- `app/docs/hud.md` — **the specification for this task** (style, layout rectangles, how the HUD is produced).
- The reference pictures: open `app/docs/img/01.HUD.png`, `02.HUD.UNITS-SELECTED.png`
  and the three asset sheets **as images** and study the chrome: panel fills,
  gold bevelled frames with cut corners, cyan accents, icon style. They are
  references for look and layout; do **not** cut pixels out of them.
- `research/04-hud-and-screens.md` section 1; `research/02-rendering-ui-stack.md` section 7.
- `app/crates/rom-render/src/` and `app/crates/rom-app/src/` — all of it
  (render thread, snapshots, simulation thread, screenshot and bench modes).
- `app/crates/rom-sim/src/lib.rs`, `economy.rs`, `income.rs`, `world.rs` — what the simulation can report.

## Build

1. **Vector assets** (`app/assets/hud/`), authored by you as SVG:
   - `chrome/*.svg`: the top bar, the bottom-left palette group, the minimap
     frame, the bottom-right selection/data group — panel fills, bevelled gold
     frames with chamfered corners, inner slots, tab shapes, bar tracks, cyan
     glow accents, and the title emblem ("ASCENSION" over "RISE OF MAN" with a
     laurel). **No dynamic text or numbers in the artwork**; static captions
     (FOOD, TIMBER, PRODUCTION QUEUE, SELECTION, tab names …) are drawn live
     as text too, so the artwork is language-neutral.
   - `icons/*.svg`: one glyph per file, 64×64 view box, gold on transparent:
     the seven resources (food, timber, metal, wealth, knowledge, oil,
     energy), commerce, population, cities, research, clock, diplomacy, chat,
     menu, pause, help, objectives, zoom in, zoom out, centre, flag, the stat
     icons (hit points, attack, armour, range, garrison, workers), the five
     data-tab icons, the four palette tabs (build, order, train, research),
     arrows left/right, and command glyphs: move, attack, attack-move, stop,
     hold, patrol, guard, follow, rally, garrison, unload, repair, build,
     cancel. Simple, bold, readable at 24 px; in the spirit of the sheets'
     command rows. No fantasy subjects.
   - A shared tile frame (`chrome/tile.svg`) used behind command and build buttons.
2. **Atlas baking** (`rom-render`, new module `hud_atlas`): at start-up
   rasterise every SVG with the `resvg` crate (exact version; default
   features off except what is needed) at the current HUD scale into one RGBA
   atlas image plus a name → rectangle table; upload once as a texture on the
   render thread; re-bake when the HUD scale changes by more than 10 %.
   Embed the SVG sources in the binary with `include_str!`/`include_bytes!`
   (list them in one generated or hand-written table).
3. **Text** (`rom-render`, new module `hud_text`): bundle one libre font file
   in `app/assets/fonts/` (DejaVu Sans Bold from
   `/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf` — its licence
   permits redistribution; copy the licence text next to it) and bake a glyph
   atlas for Basic Latin + Latin-1 at two sizes with the `fontdue` crate
   (exact version). Provide `measure` and `draw` with left/centre/right
   alignment, colour, and tabular digits behaviour good enough that changing
   numbers do not jitter (right-align numeric fields).
4. **Layout** (`rom-render`, new module `hud_layout`, pure functions): the
   rectangles of `hud.md` in design space, the uniform scale and the anchoring
   rules, producing screen rectangles for a given frame size.
5. **HUD snapshot** (`rom-render`, plain data): `HudSnapshot { game_ticks, ticks_per_second, resources: Vec<HudResource { icon, label, amount, rate, capped, visible }>, commerce: (i64, i64), population: (i64, i64), cities: (i64, i64), research: Vec<HudResearch { label, progress_pct }>, events: Vec<String>, help: String, selection: HudSelection::None }`
   (selection variants come in the next task). `RenderBridge::publish_hud(Arc<HudSnapshot>)`.
   The simulation thread in `rom-app` fills it each tick **for the first
   human-controlled player (player 1 for now)** from the world: stock and
   `world.income(...)` per base resource in the rules' id order (hide a
   resource whose `first_age` is later than the player's age), commerce cap
   from the same stat and constant the simulation uses, population used and
   cap, city count (entities whose type has a city territory component) with
   the limit from stat `city_limit` and constant `player.city_limit.base`,
   research orders in progress (name of the tech, percent), game time. Icon
   names for resources come from a small table keyed by resource abbreviation
   or id order — **not** by branching on definition keys in logic.
6. **Drawing** (render thread, after the world, into the same frame): chrome
   pieces as textured quads; live top bar (all three blocks, clock; the four
   buttons drawn but inert); message line showing the last event; help line
   with a default text; palette tabs and an empty 5×3 grid; empty production
   queue; selection panel in its empty state with the data tabs (political
   active, inert). Everything at the positions of `hud.md`.
7. **Minimap** inside its frame: a small texture built once from the
   `TerrainScene` (colour per tile kind, shaded by height), entity dots in
   owner colours from the current snapshot (buildings larger), and the camera
   view footprint as a white outline (project the four screen corners onto
   the ground plane). No input yet.
8. **World view inset.** The camera's look-at should treat the area between
   the top bar and the bottom groups as the visible world area (shift the
   projection centre accordingly) so the selected point is not hidden under the HUD.
9. **Options.** `--no-hud` hides it; `--hud-scale <percent>` overrides the
   automatic scale. The GPUI stats overlay moves to a corner that does not
   cover HUD text, or is hidden unless `--stats` is given.
10. **Window title**: "Ascension: Rise of Man".
11. **Record.** Bench lines with the HUD on, in `app/docs/performance.md`.

## Tests to add (no window, no GPU)

- `hud_layout`: `regions_scale_uniformly_and_anchor_to_edges` (two aspect
  ratios), `no_two_regions_of_a_group_overlap`, `minimap_view_lies_inside_its_frame`,
  `command_grid_has_15_slots_with_the_specified_pitch`.
- `hud_atlas`: `every_listed_svg_parses_and_rasterises` (CPU only),
  `atlas_rectangles_do_not_overlap`, `icon_names_are_unique`.
- `hud_text`: `measure_matches_sum_of_advances`, `right_aligned_numbers_end_at_the_anchor`.
- `rom-app`: `hud_snapshot_reports_stock_rates_and_caps` (a world with a
  worker on a farm: food rate 10 per period shown as the per-minute or
  per-period figure you choose — state which), `hidden_resource_before_its_age`,
  `research_progress_appears_while_researching`.
- `minimap_colours_follow_tile_kinds` (pure function).

## Rules that matter most

- Raylib only on the render thread; `rom-render` must not depend on `gpui`,
  `rom-sim`, `rom-rules`, `rom-mapgen`; `rom-app` must not call raylib.
- The HUD reads only `HudSnapshot`, `EntitySnapshot` and `TerrainScene`.
- Do not copy pixels from the reference pictures. Do not add fantasy imagery.
  No names of other games anywhere.
- Do not touch `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or
  the seed. Missing simulation API → list it and work around it.
- **Never use `xdotool`, `wmctrl` or similar.** End runs with `--bench` or
  `--screenshot`, or by ending the process you started. `env -u WAYLAND_DISPLAY`. No `sudo`.
- New dependencies: `resvg` and `fontdue` only (exact versions), plus what they pull in.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1672x941 --screenshot target/shot-hud.png --after 25
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x720 --screenshot target/shot-hud-720.png --after 5
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

Open both screenshots **and** `app/docs/img/01.HUD.png` as images and compare
them side by side: the same arrangement, the same kind of chrome (dark navy
panels, gold bevelled chamfered frames, cyan accents), readable live numbers
in the top bar, a working minimap. Iterate on the SVGs until the ensemble
reads like the reference from afar. Copy the first screenshot to
`app/docs/img/hud-current.png`. The bench should stay at or above 55 fps;
report the real number.

## Out of scope

Clicking anything on the HUD, selection, the command palette contents, the
production queue contents, data-tab views, tooltips, unit and building tiles,
chat input, key bindings.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; stage files by name;
do not push or merge. Final message: commands with real output, the bench
line, an honest comparison of your screenshot with the reference (what
matches, what does not), files created, missing simulation API, deviations
and why, anything you could not verify.
