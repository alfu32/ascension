# Task 165 — Turn any building before placing it; put a gate into a finished wall

Owner (chat, 2026-11-26), verbatim: "how do i make gates rotate after the wall ( or rotate in any
case ), for now i have only a single orientation..."

GitHub issue #167.

## Today (task 147b)

- A gate turns along the wall only when it is dropped into a line **while the line is being
  dragged** (G or a right click during the drag, `rom-app/src/selection/lines.rs`).
- A gate armed from its own button and placed with a click always has facing 0
  (`facing: None` on the construct order). No key turns the ghost.
- A finished wall cannot take a gate: the gate's footprint overlaps the segments, so the site
  is refused.
- `rom-sim` already accepts any facing in steps of 45° (`snap_facing`, `turned_footprint` in
  `construction.rs`), saved and hashed since task 147b.

## What changes

1. **Turn the ghost.** While any building is armed for placement:
   - **R** turns the ghost 45° clockwise and **Shift+R** 45° counter-clockwise;
   - so does the mouse wheel with **Ctrl** held. A plain wheel keeps zooming.

   The ghost, its footprint check and the price line show the turned building. The construct
   order carries that facing. The facing stays for the next placement of the same type until
   the tool is disarmed. Line tools (walls, minefields) are unchanged: the drag sets the
   direction there.
2. **A gate snaps to a wall.** With a gate armed and the pointer over an own finished or
   unfinished segment of a matching line, the ghost snaps:
   - onto that wall's line;
   - centred on the tile under the pointer;
   - facing along the wall, diagonals included.

   It shows valid when the three segments it covers are own segments of one straight run.
   Placing it replaces those segments with the gate site:
   - the segments are removed at once, with no refund;
   - the gate is built like any site.

   Do it through orders only (rule 3): a new construct option, or a sim rule that a gate site
   may stand over the owner's own segments of the same fortification line, which it removes
   when placed. Pick one, keep it deterministic, and add a scenario run twice with equal
   hashes. If the existing `delete` order (task 142) and a following `construct` can do this in
   one tick without a gap an enemy could pass, say so and use them.
3. **Which gate fits which wall.** Read it from data: each segment type's `et_fortification`
   row and the gate of the same line, the way 147b's drop already finds it. Never use keys.
4. **Help line.** While a building is armed: "R turns it". Over a wall with a gate armed:
   "Click to put the gate into the wall".
5. **Computer players** are unchanged.

## Crates

`rom-app` (input, ghost, help line), `rom-render` only if the ghost does not already draw a
facing, `rom-sim` for the gate-over-segments rule (with its scenario test). No schema change
expected. If one is needed, stop and report.

## Read first

- `docs/tasks/147-fortifications.md` (147b part);
- `crates/rom-app/src/selection/lines.rs`;
- `crates/rom-app/src/selection.rs` (single placement, `facing: None`);
- `crates/rom-sim/src/construction.rs`;
- the `et_fortification` table in `db/schema.sql`.

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main
  when you start, plus this card's tests).
- Tests:
  - the ghost turns with R, Shift+R and Ctrl+wheel;
  - the order carries the facing;
  - a turned barracks has its turned footprint;
  - a gate over a straight finished wall snaps, turns along it, and replaces exactly three
    segments, on a diagonal too;
  - a gate over a bend or over a foreign wall is refused;
  - own units pass the new gate; enemies do not;
  - a scenario run twice gives equal hashes.
- The playthrough, fight matrix and duel are unchanged from main, or explained.
- **Screenshots** in `docs/img/165-*.png`, from scripts in `docs/tasks/165-*-script.txt`:
  - a barracks ghost turned 45°;
  - a gate ghost snapped onto a finished diagonal stone wall;
  - the same wall with its gate built and own units walking through.
- Report as AGENTS.md says. Log only in your scratchpad folder; do not write `gate*.sh`.
