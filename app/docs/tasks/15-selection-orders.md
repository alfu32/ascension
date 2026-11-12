# Task 15 — Selection, orders and the live command palette

Crates: `rom-render`, `rom-app`. Assets: new icons in `app/assets/hud/icons/` if needed.

## Read first

- `AGENTS.md` — all of it, including "Running the application during a task".
- `app/docs/hud.md` — layout and behaviour notes; `research/04-hud-and-screens.md`
  section 1 ("Command palette", "Selection panel", "Selection and control conventions").
- `app/crates/rom-render/src/` and `app/crates/rom-app/src/` — all of it
  (camera, input forwarding, snapshots, HUD modules, simulation thread, demo).
- `app/crates/rom-sim/src/lib.rs`, `order.rs`, `world.rs`, `economy.rs`,
  `production.rs`, `construction.rs` — `Command`, `submit`, `price`, errors.
- `app/crates/rom-rules/src/model.rs` and `lib.rs` — `production` rows by
  producer, `EntityType` components, `display_name`.

## Design

Three threads keep their ownership. The **render thread** knows the camera
and the interpolated entity positions, so it does all *picking*: it turns
mouse input into **pick events** — `Click { world_point, entity: Option<u32>, modifiers }`,
`BoxSelect { entities: Vec<u32>, modifiers }`, `HudClick { region, index, button }`,
`RightClick { world_point, entity: Option<u32>, modifiers }` — and sends them
over a channel to the simulation thread. The **simulation thread** (in
`rom-app`) owns the selection set and the "pending placement" state, turns
pick events into `Command`s for player 1, and publishes the selection details
inside `HudSnapshot`. The render thread never mutates game state.

## Build

1. **Picking (`rom-render`, pure functions where possible).** Ray from the
   camera through the cursor; intersect the terrain height field (march the
   ray, then refine) for the world point; entity hit test against each
   entity's footprint or radius in screen space, nearest to the camera wins.
   Box selection: drag beyond 6 px draws a rectangle (thin cyan outline) and
   selects entities whose projected centres fall inside it. A press that
   starts over a HUD region is a HUD click and never a world pick (use the
   layout rectangles).
2. **Selection rules (simulation thread).**
   - Click on own entity: select it; Shift adds/removes; double-click selects
     all own entities of that type visible on screen (the render thread sends
     the on-screen set with the event).
   - Box: own units only; if the box contains any unit with a weapon, keep
     only units with a weapon unless Ctrl is held; Shift adds.
   - Click on empty ground or a foreign entity: clear (foreign: show it in the panel, no orders).
   - Ctrl+0…9 stores a control group; 0…9 recalls it (twice quickly: centre the camera on it).
   - Dead entities leave the selection automatically.
3. **Right-click orders (simulation thread)** for the selected own entities,
   deciding per entity by its components, never by type key:
   - on an enemy entity → `attack` (units with a weapon);
   - on a ground point → `move` for units (`attack_move` when the `A` key was
     pressed first); for a selected producer building → set its rally point
     (store in the app if the simulation has no rally API yet; list it);
   - on an own gather site → `harvest` for units that can harvest there, others `move`;
   - on an own unfinished site → `repair` (help build) for builders;
   - Shift queues instead of replacing.
   Show a short marker at the target (green for move, red for attack) for half a second.
4. **Command palette** for the current selection, from the rules:
   - **BUILD** tab (selection contains a builder): one button per
     `production` row with mode `construct` for that builder type, paged by
     `ui_page` (economic / military / wonder) with the arrow buttons; greyed
     when locked or unaffordable (ask the simulation: `is_unlocked`,
     availability, `price` vs stock).
   - **TRAIN / RESEARCH** tabs (a producer building selected): `train` and
     `research` rows of that type. Clicking submits the order (left: once;
     Shift+left: five). Right-click on a queued item in the production queue cancels it.
   - **ORDER** tab: stop, attack-move, patrol (if supported by the
     simulation; otherwise omit), and the stance buttons (aggressive,
     defensive, stand ground, hold fire) applied with the world's stance setter.
   - Slot positions from `ui_col`/`ui_row` when set, else in id order.
   - Button face: the tile frame plus a placeholder pictogram per entity class
     and ui group (a generic house, tower, shield, scroll, person — authored as
     SVG in the existing style; **no per-unit art yet**), with the display name
     for the player's faction (`rules.display_name`) shown in the help line on hover,
     and the price on the line below it.
   - Keyboard: the palette slots map to Q W E R T / A S D F G / Z X C V B for the visible page.
5. **Build placement.** Clicking a BUILD button enters placement: the render
   thread draws the footprint on the terrain under the cursor, green when the
   simulation says it is valid, red otherwise (the simulation thread answers a
   `CheckPlacement` request for the hovered tile, at most every 3 ticks);
   left-click places (submits `construct` for the nearest selected builder,
   others `repair` it), Shift keeps placing, right-click or Escape cancels.
6. **Selection panel** (`HudSelection` variants): single own or foreign
   entity → portrait tile (placeholder pictogram), name, HP, attack, armour,
   range, garrison and workers rows when applicable (hide the rest);
   multiple → grid of up to 12 type tiles with health bars and ×n stack
   counts, "n selected", clicking a tile narrows the selection to that type.
7. **Production queue** panel: for a single selected producer, up to three
   entries with progress bars from the order queue; more as "+n".
8. **Selection feedback in the world:** owner-coloured ring under selected
   units, outline on selected buildings, a health bar for every selected entity.
9. **Help line:** the hovered button's name, price and blocking reason.
10. **Minimap input:** left-click moves the camera there; right-click issues the right-click order at that point.
11. Remove `--demo`'s random moves when the player issues any order (keep the demo for the other players).

## Tests to add (no window, no GPU)

- `rom-render`: `ray_hits_terrain_at_the_expected_point`, `box_select_uses_projected_centres`,
  `press_on_hud_region_is_a_hud_click`, `nearest_entity_wins_pick`.
- `rom-app` (headless simulation thread logic): `click_and_shift_click_selection`,
  `box_prefers_military_unless_ctrl`, `control_group_store_and_recall`,
  `right_click_enemy_attacks_and_ground_moves`, `right_click_gather_site_harvests`,
  `right_click_site_helps_build`, `shift_queues_orders`,
  `palette_lists_production_rows_for_selected_type` (types chosen by component, not key),
  `locked_and_unaffordable_buttons_are_disabled_with_reason`,
  `train_button_submits_and_queue_shows_progress`, `cancel_from_queue_refunds`,
  `placement_check_reports_valid_and_invalid`, `dead_entities_leave_selection`.

## Rules that matter most

- Ownership as in "Design". `rom-render` must not depend on `gpui`,
  `rom-sim`, `rom-rules`, `rom-mapgen`; `rom-app` must not call raylib.
- No branching on definition keys or names; behaviour from components and production rows.
- Do not touch `rom-sim`, `rom-rules`, `rom-mapgen`, `rom-cli`, the schema or the seed.
  Missing simulation API (rally points, patrol, a placement check without
  submitting) → work around it in `rom-app` where possible and list it.
  (A parallel task is changing `rom-sim` placement rules; use `submit`
  results as the source of truth, not your own rule logic.)
- **Never use `xdotool`, `wmctrl` or similar.** You cannot click in the
  window from a script; verify interaction through the headless tests and
  through screenshots with a scripted selection: add a hidden option
  `--script <file>` that replays pick events (one per line, with tick
  numbers) on the simulation thread, so a screenshot can show a selection,
  an open palette and a placement preview. End runs with `--screenshot` or
  `--bench`. `env -u WAYLAND_DISPLAY`. No `sudo`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 1 --size 1672x941 --script docs/tasks/15-demo-script.txt --screenshot target/shot-select.png --after 12
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```

Write `app/docs/tasks/15-demo-script.txt` yourself: select the five builders
by box, open BUILD, enter placement for the first building, hover a valid
spot. The screenshot should show selection rings, the multi-selection panel,
a populated palette and a green placement footprint. Copy it to
`app/docs/img/hud-selection.png`. Bench at or above 55 fps; report the real number.

## Out of scope

Formations and facing drag, garrison/unload buttons, abilities, diplomacy,
data-tab views, minimap filters, tooltips with rich text, sound, real unit art.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; stage files by name;
do not push or merge. Final message: commands with real output, the bench
line, what the screenshot shows, files changed, missing simulation API,
deviations and why, anything you could not verify.
