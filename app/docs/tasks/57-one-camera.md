# Task 57 — One camera, one scene: from the street to the star map

Crates: `rom-render`, `rom-app`, docs (no simulation change). Starts after
task 44 is merged (44 also works in `rom-render` and `rom-app`). May run
beside the model cards 45–51 (they only add `app/db/models/` files and
part-kit shapes in `rom-kit`). Implementer: Claude (Opus).

## Why — the owner (2026-11-15)

"We should use exactly the same camera from the planetary level up into the
orbit, higher on the solar plane and even higher on the galactic plane. The
zooming right now is not seamless, which I find a bit annoying. We could
make it seamless; we could even make it so that the other planets in the
solar system are visible from the planet's surface."

"Spaceships should be much bigger in the planet view (2×–5× depending on
their type) and a bit smaller on the solar plane view (½–⅓, maybe even ¼).
We could use the same model on both planes."

Task 53 joined the levels with hand-overs between separate frames (planet
frame, system frame, star-map frame; `rom-render/src/zoom.rs` `Level`,
`HOME_PLANET_RADIUS`, `SYSTEM_MAX_DISTANCE`, the matched globe/sphere at the
hand-over). Each hand-over swaps scene and projection, which the owner sees
as a jump. Task 56 then re-proportioned the systems.

## Build

1. **One scene, one camera.** A single perspective camera with one position,
   heading and pitch that moves continuously from street level through
   orbit, the whole system and the star map, and back. No frame swap, no
   cross-fade, no change of projection: what changes with distance is only
   what is drawn and in how much detail.
   - Put every level in one nested coordinate space for rendering (the
     planet's surface inside the planet's place in its system, the system
     inside its place on the star map), rendered camera-relative so float
     precision holds at every scale (floating origin, or per-level matrices
     composed at draw time; doubles allowed in rendering only — the
     simulation stays integer).
   - Depth must work from a building at 1 m to systems light-years apart:
     logarithmic or reversed depth, or depth-partitioned passes drawn in one
     frame from far to near, without visible seams.
   - Detail changes by distance (globe chunks, surface objects, labels,
     orbit rings, system markers) fade or morph; nothing pops.
   - The planet the player stands on is the same globe mesh at every
     distance, sitting at its true place in its system: zooming out shows it
     shrink onto its orbit with the star and the other bodies around it,
     zooming further shows the system shrink onto the star map.
   - Keep task 53's controls and behaviour: pan, rotate, zoom to the cursor
     (now also on the planet), flights for focus and follow, the level cap by
     age (planetary / interplanetary / interstellar from task 43), fog per
     level, the minimap per level, hover help, double-click to fly.
2. **The sky from the surface.** From the planet's surface and low orbit the
   star and the other bodies of the system are visible in the sky at their
   true directions (small discs or points with a soft glow; the star as the
   source of the directional light — wording: the directional light that
   represents the sun orbits the planet orb). Other systems as a faint star
   field in their true directions.
3. **Ships at a readable size, one model.** Each ship uses the same model at
   every level; its drawn size is its true size times a factor per level
   that reads well: **2–5× on the planet view** (by ship class, from data:
   e.g. a stat or a per-type presentation row, never by key) and **¼–½ on
   the system plane** compared with today. The scale changes smoothly with
   distance, never jumps. Ships stay pickable (task 53's pick floors).
4. **Do not assume a flat star map.** The owner wants the galactic map to
   become a 3D network of star systems later (task 59). Today systems have
   `x, y` only; build the camera and the star-map drawing so that a third
   coordinate drops in without redesign (free orbit of the camera around
   the star map, depth sorting and picking that work off the plane), even
   though the generator still places every system at height 0.
5. **Wheel steps.** Retune so the whole range, street to whole system, and
   system to star map, takes a comfortable number of wheel steps (task 56
   measured about 31 from the globe to the whole system; aim for clearly
   fewer, and document the count).

## Tests

Camera continuity: sweeping the zoom from street to star map and back, the
projected screen position and size of fixed reference objects (the home
planet, the star, a ship) change smoothly — no frame-to-frame jump larger
than a small bound — at every former hand-over distance. Ship scale per
level from data. Existing pick, hover and level-cap tests keep passing.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
complete and its hash unchanged (no simulation change). Screenshots you
looked at: a strip of frames from street level out to the star map in even
wheel steps, with no visible jump between neighbours (especially at the old
hand-over points); the sky seen from the surface with the star and other
planets visible; a ship in orbit above a city at the new planet-view size;
the same ship on the system plane at the new size. Bench the sphere demo
before and after (with `uptime`) — no regression beyond noise. Commit on the
branch (stage by name), do not push or merge; final message with commands
and real output, screenshots, files changed, deviations, anything not
verified.
