# Task 53 — Navigable space: one continuous camera from street to star map

Crates: `rom-render` (camera, space scenes), `rom-app` (input, selection,
minimap, focus), docs. Starts after task 43 is merged (tiers, ships in orbit).
Implementer: Claude (Opus: camera and input work is in the "hard" column).

## The owner's report (2026-11-15)

- "The player should be able to pan, zoom and rotate in the solar plane; they
  can't see where my ships are."
- "Once the player sends a ship to another solar system, it goes away from the
  (fixed) solar plane view; I cannot find it any more."
- "Space should be navigable just like the planetary space. It is a flat 2.5D
  space (just like the planetary space: a quasi-flat surface over a sphere,
  with different rendering by zoom level). When I exit the planet space the
  camera should stay the same direction-wise, and the same when exiting the
  solar plane."
- "It is unclear how I go and visit other solar systems and planets. Moving
  the focus point by panning and zooming in to another solar system would
  also solve landing on other planets and actually taking them in the view."

## Design

1. **One camera, three planes.** The planet (cube-sphere surface), the star
   system (a flat plane around the star) and the star map (a flat plane of
   systems) are all 2.5D grounds under the same orbit-style camera: a focus
   point on the current plane, a heading (yaw), a tilt that follows the zoom
   (as on the planet today) and a distance. On every plane the player can
   **pan** (drag, edge scroll, arrow keys), **zoom** (wheel, toward the
   cursor) and **rotate** (the same rotate input as on the planet).
2. **Continuous transitions keep direction.** Zooming out past the planet's
   limit hands over to the system plane with the focus on that planet and the
   **same heading**; zooming out past the system hands over to the star map
   with the focus on that system and the same heading. Zooming **in** works
   the other way round, toward what is under the cursor: zooming in on any
   known planet in a system view lands on that planet (its surface camera,
   heading kept); zooming in on a system in the star map enters that system.
   So visiting another system is: zoom out to the star map, pan to it, zoom
   in; the same for another planet. The tier caps of task 43 still apply
   (a player in the interplanetary tier cannot leave the home system view).
3. **Ships are always findable.** Ships are drawn at every level: models
   above a planet in orbit (task 43), models or icons on the system plane,
   icons on the star map, including ships **in transit along a route**
   (moving along the lane). They can be selected at every level and given
   orders there (move, orbit, travel to another system, bombard).
   **Focus**: selecting a ship and pressing the focus key (or double-clicking
   its portrait) moves the camera to it at the right level; a "follow" toggle
   keeps it centred.
4. **Minimap and HUD follow the level**: the minimap shows the planet, the
   system or the star map you are in, with ship markers and the camera's view
   box; clicking it pans there. The level indicator shows where you are.
5. **Visibility**: only systems and bodies the player knows are drawn in
   detail (fog per level, as in the world model); unknown systems stay
   markers on the star map.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`); window-free tests for the camera hand-over (heading kept
across planet → system → star map and back; zoom-in on a body or system
lands there; tier caps respected) and for focus on a ship in orbit, in system
space and in transit. Screenshots you looked at, by script (`--script`) or
options: the system plane panned and rotated; the star map with a ship in
transit on a lane; the camera after zooming into a second system and onto one
of its planets; the same heading before and after each hand-over. Bench the
sphere demo twice with `uptime`. Commit on the branch (stage by name), do not
push or merge; final message with commands and real output, screenshots,
files changed, deviations, anything not verified (live mouse input cannot be
driven; say what was only tested without a window).
