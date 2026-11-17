# Task 79 — A star map you fly through: no plane, no grid, only stars and routes

Crates: `rom-render` (`zoom.rs`, `camera.rs`, the star-map part of
`scene/space.rs`), `rom-app` (input, help lines), tests, docs
(`app/docs/world-model.md` "Moving between levels" and "The real sky",
`app/docs/hud.md` or the help lines). No change to `rom-sim`, `rom-ai`,
schema or seed. Starts after task 70 is merged (it draws the routes in the
same files). Implementer: Claude (Opus).

## Why — the owner (2026-11-17)

This is the refinement of task 74's provisional star-map rule.

"Why do we have a galactic plane? The plane should only exist in the solar
view, the solar plane. In the galactic view there should be no plane. It
keeps gluing the camera target to that plane, and since the stars are
relatively distant from it, the navigation gets contorted in unexpected ways.
I should be able to simply navigate a point cloud as you would while flying.
Plus all that cylindrical-coordinates graphing gets mixed up with the actual
route lines in a confusing way for the player."

Where it comes from: task 59 placed the stars at their real 3D positions and
drew the sky's equatorial plane as rings, with a drop line from every system
to a small ring on that plane, to show height. Task 74 then kept the camera
focus on that plane (`ZoomState::onto_reference_plane`, `reference_height`).
Both go.

## Read first

`AGENTS.md`; `app/docs/world-model.md` ("Moving between levels", "One camera,
one scene", "The real sky"); `app/docs/tasks/74-camera-feel.md` (what stays:
wheel coalescing, animated steps, middle drag turns and Shift + middle drag
pans, the invert settings); `app/docs/tasks/70-jumps-and-inhibitors.md` item 6
(how routes are drawn); `rom-render/src/zoom.rs` (focus, pan, turn, the
system and star-map hand-overs, their tests), `rom-render/src/scene/space.rs`
(the star-map overlay: `GRID_LINE`, `DROP_LINE`, the rings and drop lines,
constellation figures), `rom-render/src/camera.rs` (keys).

## Build

1. **No plane on the star map.** Remove the reference-plane rings, the drop
   lines and their foot rings, and the code that keeps the focus on the
   reference plane. Outside a system nothing refers to a plane. The system
   plane stays exactly as task 74 made it, inside a system.
2. **A free focus in 3D.** On the star map the focus is a free point in
   space.
   - **Turn** (middle drag, or as swapped in settings): orbits the camera
     around the focus about the camera's own axes (horizontal drag about its
     up axis, vertical drag about its right axis), so a drag behaves the same
     whatever the orientation. No elevation clamp and no poles: the camera
     can go all the way over. It never rolls by itself. The invert settings
     of task 74 apply.
   - **Pan** (Shift + middle drag): moves camera and focus together in the
     screen plane at the focus depth; the point under the cursor stays under
     the cursor.
   - **Zoom** (wheel, one step per gesture, animated, as task 74): dollies
     toward the pointer. If a star is under the cursor, the focus glides to
     it; otherwise the focus moves along the pointer ray. Zooming in close to
     a system still enters it, at the same distance as today.
   - **Fly** (keyboard): forward and back along the view, sideways, up and
     down in screen space, at a speed scaled to the distance to the focus.
     Pick keys that are free on the star map today, and list them in the help
     line.
   - **Level the view**: one key turns the camera upright again (its up axis
     back to the map's north) with a short ease, for a player who got lost.
   - Clicking or double-clicking a system, flying to a system or a ship, and
     following one work as today (the focus sits on that system).
   - The focus stays inside the map's region (a sphere around the match's
     systems with a margin), so the player cannot fly off into nothing. Task
     76 will add the celestial home beyond the match region: keep the bound
     in one place, computed from the systems shown.
3. **Hand-overs stay seamless.**
   - System → star map: the star map starts from the system camera's exact
     pose; it is then free.
   - Star map → system: entering a system from any orientation, including
     upside down, eases to a pose of that system's plane: heading kept as far
     as possible, elevation into task 74's ±89° range, roll levelled, focus
     on the plane. No jump in the first frame.
   - The round trip planet → system → star map → system → planet returns the
     planet camera unchanged (existing tests).
4. **Depth without a grid.** Height and distance read from the stars
   themselves: stars and their halos smaller and dimmer with distance from
   the camera, labels fading with distance and never upside down, near stars
   drawn over far ones, a slight fade of the farthest stars. Turning gives
   parallax. No new lines.
5. **Routes are the only lines that look like routes.** On the star map,
   route lines (task 70's charted, uncharted and inhibited styles) are the
   only solid lines between systems. Constellation figures stay off by
   default (F3) and, when shown, look nothing like routes: thin, dotted or
   dashed, a hue that no route style uses, under the routes. F2 "Sky from the
   Sun" keeps working.
6. **Tests.**
   - The focus is never moved to a plane on the star map (pan, zoom, turn and
     fly from a system far off the old reference plane leave its height
     where the input put it).
   - Pan keeps the point under the cursor fixed.
   - Turning keeps the focus fixed on screen at any orientation, including
     past where the old ±89° clamp stopped.
   - Wheel zoom toward a star brings the focus onto it; zooming in close
     enters the system.
   - Entering a system from an inverted orientation ends level, within
     ±89°, focus on the system plane, with no first-frame jump.
   - The focus bound holds.
   - The star-map overlay has no reference rings or drop lines (a check on
     the built overlay, not on constant names).
   - Task 74's tests for the system view still pass unchanged; star-map
     tests that assert the old plane rule are rewritten to the new rule
     (say which).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules); playthrough unchanged; screenshots you looked at:
the star map before and after (same pose), the map seen from far above and
from below the old plane, a strip of a flight between two distant systems,
routes with constellation figures shown; commit on the branch (stage by
name), do not push or merge; final message with commands and real output,
files changed, keys added, deviations, anything not verified (live mouse and
keyboard feel cannot be scripted: say so).
