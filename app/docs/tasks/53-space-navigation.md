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

Follow-up from the owner the same day: double-clicking a star system on the
star map does enter it, but "I still can't see the ships in transit, I don't
know where they are; navigating between star systems and focusing on other
systems breaks the attention flow of the user." So: (a) ships in transit
must be visible, and (b) every change of place must be a continuous camera
move the eye can follow, never a cut.

## Scope (owner, 2026-11-15)

"What you did until now, interaction-wise, is coherent: double-click focuses
the planet or the star system. The seamless transitioning and the solar and
star maps are a bit less natural; that is what we need to improve,
navigation-wise." So **keep today's interactions** (double-click focuses a
planet or system, selection, orders, hotkeys) and do not redesign them;
improve (a) the transitions between levels, made seamless, and (b) moving
around the solar-system and star-map planes, made as natural as on the
planet (pan, rotate, zoom toward the cursor, tilt with zoom). Items 3–5 below
(ships findable, minimap, visibility) support that.

## Design

0. **No cuts; keep the user's attention.** Every change of level or place is
   an animated camera flight (a short, eased move of focus, distance and
   heading, interruptible by any input), so the player always sees where they
   came from and where they arrive. The existing double-click on a system
   becomes "fly into this system" (the same continuous zoom-in as below), and
   double-click on a planet "fly down to this planet". The camera never jumps.
   The owner confirmed (2026-11-15) that double-click is the coherent way in
   and must stay; what is missing is free movement and a **borderless
   transition**: on every level the player can shift/pan, rotate and zoom,
   and zooming continuously from the planet into space and back shows no seam
   — the planet shrinks into its place in the system plane (and the system
   into its place on the star map) while the surrounding level fades in,
   with the same heading and the focus point kept, so the player never feels
   a mode switch.

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
   (moving along the lane at their real progress, with a direction arrow and
   the destination system highlighted; today they are drawn nowhere — find
   out why first and report it). A ship ordered to another system stays
   selectable for its whole journey, and its departure and arrival show in
   the event line with a click-to-focus. They can be selected at every level and given
   orders there (move, orbit, travel to another system, bombard).
   **Focus**: selecting a ship and pressing the focus key (or double-clicking
   its portrait) moves the camera to it at the right level; a "follow" toggle
   keeps it centred.
3a. **Make it obvious.** The owner found double-click-to-enter on systems and
   planets only by trying ("not obvious"). Hovering a system, planet or ship
   highlights it and shows a tooltip naming it with the actions ("Double-click
   or scroll in to enter", "Double-click to land the camera"); the cursor
   changes over enterable objects; the help line at the bottom of the HUD
   says how to move between levels; the first time a player reaches the
   interplanetary and interstellar tiers, a short hint explains it.
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
