# Task 176 — Focusing a place in space keeps the selection

Owner (chat, 2026-11-28), verbatim: "when i doubleckick to focus on a star system ( ar a space
location ( planet, star, group, cluster, etc ... ) i loose the selection ... makes it difficult to
navigate. please fix this"

GitHub issue #179.

## Cause

- In `crates/rom-app/src/selection.rs`, `Controller::click` clears the selection when a plain
  click hits no entity: `let Some(id) = entity … else { if !mods.shift { self.set_selection(Vec::new()) } }`.
- In the system, cluster, quadrant and galaxy views, a double click on a planet, a star, a
  system, a cluster or a quadrant flies the camera there (`space_view.rs`, `universe_view.rs`,
  `rom-render` zoom).
- Its first click reaches `click` with no entity, so the selection is gone before the camera
  moves.

## What changes

1. **A click on a place in space never changes the selection.** Places are planets, moons and
   resource bodies, stars and systems, clusters and their labels, quadrants, and route ends
   ("to … group"). Find where the space views hit-test these and pass that through, as the
   pick already passes `entity`, so `click` can tell "clicked a place" from "clicked empty
   space".
   - A single click on a place: the selection stays, and nothing else happens unless the view
     already does something with it (say what).
   - A double click on a place flies there and keeps the selection.
2. **What still clears the selection:**
   - a plain click on empty space with nothing under it;
   - Escape;
   - clicking another unit;
   - on a planet surface, a click on empty ground, as now.

   With Shift held, nothing is cleared, as now.
3. **Orders are not affected:** a right click on a place still gives the selected units their
   order (move, travel, orbit, land). This is the point: select ships, double-click a far
   system to look at it, right-click there to send them.
4. **The selection panel and the selected units' markers stay** through the camera flight.
   Units out of view stay selected, and the panel lists them.

## Crates

`rom-app` (selection, the space views' picks), and `rom-render` only if the hit-test lives there
(the zoom's pick). No simulation change.

## Done when

- Gate green from `app/` (fmt, clippy `-D warnings`, workspace tests ≥ main plus this card's
  tests); playthrough and ascension hashes unchanged.
- **Tests** in `selection/tests.rs` or beside the views:
  - a single and a double click on a planet, a star, a cluster and a route end keep a
    selection of ships;
  - a click on empty space clears it;
  - Escape clears it;
  - a right click on a far system after a double click there sends the ships.
- **Screenshot** `docs/img/176-*.png`: ships selected, the camera flown by double click to
  another system, the selection panel still showing the ships. Open it with Read.
- Report as AGENTS.md says. Logs only in the scratchpad's `t176/`; never write `gate*.sh`.
