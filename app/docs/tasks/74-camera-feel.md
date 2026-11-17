# Task 74 — A camera that feels right: low angles in space, smooth zoom, one way to turn

Crates: `rom-app` (input, camera control, settings), `rom-render`
(`camera.rs`, `zoom.rs`, the space and star-map passes where they depend on
elevation), tests, docs (`app/docs/world-model.md` "Moving between levels",
`app/docs/hud.md` or the help lines). No change to `rom-sim`, `rom-ai`,
schema or seed. Starts after task 59 is merged (it changes `camera.rs`,
`zoom.rs` and the star map). Implementer: Claude (Opus).

## Why — the owner (2026-11-17)

Screenshot reference: a system seen from about 10–15 degrees above its
plane, the planet with its rings in front, the star to the side, orbits as
long ellipses. "It offers a more spectacular perspective."

1. Leaving the planet surface into orbit and the system plane should come
   out at a **low elevation, about 10–15 degrees**, as the **default** (today
   it is near 90 degrees, looking straight down). Remembering the last
   system-plane angle the player set stays. The turn control must reach
   **about ±89 degrees** of elevation, so the camera can also go **under**
   the plane.
2. The star map the same way (it takes its elevation from what the player
   has set; keep that, with the same ±89 degree range and the low default
   the first time).
3. "Zooming happens in steps that are too big": one wheel step leaves the
   low orbit for the system plane, one more crosses the system, a third or
   fourth is already the star map. "Sometimes the scroll wheel misfires and
   the player doesn't know where they are anymore." Smooth it: more steps,
   animated zoom, or both, and at most one step per wheel gesture however
   many events the wheel fires.
4. "In the planet view we rotate with the middle button, in the system and
   star map with Shift + middle. It is confusing."
6. "The target of the camera could always remain on the solar plane; that
   would prevent weird panning or zooming when the target strays too far
   from it. The player's focus is on the plane, and the planets' orbits all
   lie on one common plane." (added the same day)
5. "In the system and star map the left–right rotation is
   counter-intuitive: as if we rotate the target around the camera. I am
   used to rotating the camera position around the target."

## Read first

`AGENTS.md`; `app/docs/world-model.md` ("Moving between levels",
"One camera, one scene", "The real sky" from task 59); `app/crates/rom-render/src/camera.rs`,
`zoom.rs`, `scene/space.rs`; `rom-app`'s input handling and settings
screen; the existing camera tests (seams, continuity, round trip back to
the planet camera).

## Build

1. **Elevation.**
   - Pitch range about −89° to +89° in the system view and on the star map
     (below the plane allowed); on the planet keep today's limits unless
     they block item 2.
   - The first hand-over from the planet to the system plane eases to an
     elevation of 12° (a constant, documented); after that the system view
     and the star map remember the last elevation the player set, per match.
   - Keep the hand-over seamless (task 57/53 rules): the space camera starts
     from the planet camera's exact pose and eases to the plane pose; the
     star map shares the system's heading and elevation.
   - At low and negative elevations the system must still read: orbit
     rings, body halos, labels, picking, the star's glow and the ship models
     (task 69) all correct from below; screen-space text never upside down.
2. **Smooth zoom.**
   - Finer steps: about three to four times as many wheel steps across the
     same range, so leaving orbit, crossing the system and reaching the star
     map each take several steps.
   - Animated: each step eases the camera distance over about 150–250 ms
     (a constant); a new step during an animation retargets smoothly.
   - One step per gesture: wheel events arriving within a short window
     (about 80 ms, constant) count as one step, so a misfiring or
     free-spinning wheel moves one step, not five. Touchpad smooth
     scrolling (fractional deltas) still zooms continuously.
   - Level changes (planet → system → star map) happen at the same
     distances as today, only reached more gradually; the minimap names the
     level and the help line says where you are at every hand-over.
3. **One way to turn (owner's question; the recommendation adopted unless
   the owner says otherwise).** Middle drag *turns the view* at every level:
   on the planet it turns the globe under the camera (as today); in the
   system view and on the star map it orbits the camera around the focus
   (heading left–right, elevation up–down). Shift + middle drag does the
   other job at every level (pan on the plane and the map; turn the heading
   on the planet). Settings get a choice "Middle drag: turn / pan" that
   swaps the two everywhere.
4. **Orbit around the target.** Left–right turning moves the camera's
   position around the focus point, never the focus around the camera;
   dragging right swings the camera so the scene follows the cursor (as in
   most 3D strategy games and editors). Settings get "Invert horizontal
   turn" and "Invert vertical turn". Check the keyboard turn keys and the
   F2 "Sky from the Sun" look (task 59) keep their own sensible directions.
5. **The focus stays on the plane.** In the system view the camera's focus
   point always lies on the system plane: panning moves it within the plane
   (ray from the pointer to the plane, also when looking from below), zoom
   toward the pointer anchors on the plane point under the cursor (or on the
   body under it, projected to the plane), turning orbits around that plane
   point, and no input can lift the focus off the plane. Near the horizon
   (pointer ray almost parallel to the plane) panning and zoom-to-pointer
   are clamped to a sensible distance instead of shooting off. On the star
   map (3D since task 59) the focus lies on the map's reference plane (the
   one task 59 draws its rings and drop lines to), except while flying to or
   following a system, when it sits on that system; any pan returns it to the
   reference plane.
6. **Tests.** Camera unit tests for: the first hand-over elevation, the
   remembered elevation, the ±89° clamp, wheel coalescing (a burst of 6
   events is one step), the animated zoom reaching its target, the round
   trip planet → system → planet returning the planet camera unchanged, and
   orbit-around-focus (the focus point stays fixed on screen while turning).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules/playthrough unchanged); screenshots you looked at:
the system at the default 12° (compare with the owner's reference), from
below the plane, the star map at a low angle, and a strip of six
consecutive wheel steps from low orbit to the star map; a short note of the
step distances before and after; commit on the branch (stage by name), do
not push or merge; final message with commands and real output, files
changed, settings added, deviations, anything not verified (live mouse feel
cannot be scripted: say so).
