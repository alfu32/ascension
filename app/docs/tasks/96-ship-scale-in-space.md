# Task 96 — Ships in space that scale with the zoom, and stay clickable

Crates: `rom-render` (space scene, picking). Implementer: Claude (Opus). Starts after task 93
(both change how ships look in space) or with its agreement.

## Why — the owner (2026-11-18)

"Spaceships visualisation in space is a bit odd because zooming doesn't affect the relative size
of the ship … let's try to be more realistic without sacrificing too much the visibility and
clickability of the ships."

Today ships in system space keep a fixed size on screen (the ship view scale), so zooming in
does not bring them closer and zooming out leaves them as large as planets.

## Build

1. **True size when near, a floor when far.** A ship is drawn at a world size that follows its
   class (corvette smallest, battleship largest, from data: the model's own size), so zooming in
   makes it grow like the planets do. When that would make it smaller than a minimum on screen
   (about 14 px for the smallest class, scaling with class), it is drawn at the minimum instead,
   so it stays visible; below a second threshold it becomes a team-coloured class marker (a
   small chevron or diamond by class, facing its heading) instead of a shrunken model.
   Transitions are smooth (no popping): blend over a band of zoom.
2. **Relative scale.** Pick the world-size factor so ships read as ships beside planets in the
   system view (a battleship smaller than a moon, larger than a fighter), and state it.
3. **Clicking**: the pick box is at least the drawn size, and never smaller than a minimum on
   screen (about 20 px), at every zoom; a box drag selects ships at any zoom. Fleets close
   together: the marker of a group shows a count when ships overlap.
4. Same rules for ships in orbit seen from the globe, for transit lanes on the star map, and for
   orbital craft.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md`, tasks 53, 57 and 79's cards,
`rom-render/src/scene/space.rs`, `zoom.rs`, `pick.rs`, `universe.rs`.

## Done when

- Gate green from `app/`; playthrough hash unchanged.
- Tests: the drawn size follows zoom above the floor, sits at the floor below it, becomes a
  marker below the second threshold; the pick box is never under the minimum; repeat zooms give
  the same sizes.
- Screenshots you looked at (`app/docs/img/`, scripts `app/docs/tasks/96-*-script.txt`): a fleet
  beside a planet zoomed in close, mid, far, and the whole system; a group showing a count.
- A scripted click on a ship at the far zoom selects it.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the sizes chosen, deviations, anything not verified.
