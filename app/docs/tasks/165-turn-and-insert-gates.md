# Task 165 — Turn buildings before placing them; drag gates like walls; put gates into finished walls; short wall hints

Owner (chat, 2026-11-26), verbatim: "how do i make gates rotate after the wall ( or rotate in any
case ), for now i have only a single orientation..."

Then, on the instructions for dropping a gate while dragging a wall: "add somehow these
instructions in the game, while constructingthe wall maybe, make them shorter in formulation
however (will 'press g to drop a gate' do ?) / or maybe we should build gate chains the way we
build wall chains ...."

GitHub issue #167.

## Today (task 147b)

- A gate turns along the wall only when it is dropped into a line **while the line is being
  dragged** (G or a right click during the drag, `rom-app/src/selection/lines.rs`).
- A gate armed from its own button and placed with a click always has facing 0
  (`facing: None` on the construct order). No key turns the ghost.
- A finished wall cannot take a gate: the gate's footprint overlaps the segments, so the site
  is refused.
- The game does say how: arming a wall calls `say` ("…: press and drag along the ground, G
  drops a gate, Escape to stop", `selection.rs`), and the drag's help line ends "Release to
  build; G or right click drops a gate". The owner did not notice either: the first passes as
  a message, the second is long and appears only once the drag has started.
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
3. **Gates are dragged like walls.** With a gate armed, a press and drag lays gates along the
   ground the way a wall line does (`lines.rs`), each three tiles wide and turned along the
   drag, diagonals included:
   - a drag shorter than one gate lays one gate turned towards the drag, so a short flick sets
     its direction;
   - a longer drag lays gates side by side;
   - where the drag runs over the player's own segments of the matching line, those gates go
     into the wall as in point 2;
   - a plain click still places one gate with the ghost's facing (point 1).
4. **Which gate fits which wall.** Read it from data: each segment type's `et_fortification`
   row and the gate of the same line, the way 147b's drop already finds it. Never use keys.
5. **Short instructions that stay on screen.** While a line or gate tool is armed, a hint stays
   visible (beside the pointer or in the HUD's help line, whichever 147b's line help uses; not
   a message that fades), and it is short:
   - wall armed, not dragging: "Drag to build the wall · Esc stops";
   - wall dragging: "Press G to drop a gate · release to build";
   - gate armed: "Drag to lay gates · R turns · click a wall to put one in";
   - any other building armed: "R turns it".

   Keep the price and the segment count where the drag line shows them now. Replace the
   current `say` text when the tool is armed with the same short line.
6. **Computer players** are unchanged.

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
  - dragging with a gate armed lays gates turned along the drag; a short drag turns one gate;
  - the short hints show while each tool is armed, before and during the drag;
  - own units pass the new gate; enemies do not;
  - a scenario run twice gives equal hashes.
- The playthrough, fight matrix and duel are unchanged from main, or explained.
- **Screenshots** in `docs/img/165-*.png`, from scripts in `docs/tasks/165-*-script.txt`:
  - a barracks ghost turned 45°;
  - a gate ghost snapped onto a finished diagonal stone wall;
  - the same wall with its gate built and own units walking through;
  - a stone wall being dragged with the hint "Press G to drop a gate · release to build".
- Report as AGENTS.md says. Log only in your scratchpad folder; do not write `gate*.sh`.
