# Task 56 — Star systems that look like star systems

Crates: `rom-mapgen` (universe layout), `rom-render` (system scene, zoom
scale), `rom-app` (snapshot and event line for item 5), docs. Starts after task 53 is
merged (both work on the system view). Implementer: Claude (Sonnet).

## Why — the owner (2026-11-15)

"Make the planetary plane look more realistic, with wider orbits,
proportionally more accurate." Today `rom-mapgen/src/universe.rs` spaces
orbits evenly (`ORBIT_STEP` 1000 units, ring i at roughly 1000·(i+1)), and the
system scene draws planets and the star large relative to their orbits, so a
system looks like a tight target of rings.

## Build

1. **Orbits spaced like real systems:** radii grow roughly geometrically
   (each orbit about 1.4–1.9 times the previous, seeded), inner rocky
   planets close, gas or ice giants far out, an asteroid belt between them
   where the generator places resource bodies, moons on small orbits around
   their planet (not around the star). Keep the integer, seeded,
   deterministic generation (rule 1) and the fairness of home systems.
2. **Sizes in proportion:** the star and the planets much smaller relative
   to their orbits than today (closer to real proportions, compressed only as
   much as play needs); every body keeps a **minimum on-screen size** and a
   soft halo or marker when it would be smaller than a few pixels, so it can
   still be seen, hovered and double-clicked (task 53's interactions).
3. **Continuity with the planet view:** the hand-over between the planet and
   the system plane (task 53) must stay seamless: the planet shrinks into its
   true place and size in the system; the zoom scale (`ZoomScale`,
   `HOME_PLANET_RADIUS`) is retuned so the larger system still zooms out in a
   comfortable number of wheel steps, and the star map hand-over still works.
4. Orbit rings faint and thin; orbits may be slightly elliptical if it reads
   well, and bodies move along them slowly (presentation only).

5. **Say what a resource body holds (owner, 2026-11-15: "it's not clear
   what types of resources the moons hold … surprise").** Claiming a tar
   moon with a survey ship silently added oil income. Make it readable
   before and after:
   - each resource body is drawn in a look of its resource (the resource's
     own colour and its HUD icon beside the body, from the rules, not by
     key) at every zoom where the body is visible, in the system view and
     in the minimap;
   - the hover tooltip (task 53's `hover_help`, today "(resource body)")
     names the body's type and resource and its yield per income period
     as the player would get it (base income with the player's modifiers,
     e.g. "Tar Moon · Oil +42 / 30 s"), and its claim state: unclaimed,
     yours, another player's, or contested;
   - the event line reports changes for the player: "Tar Moon claimed: Oil
     +42 / 30 s", "Tar Moon lost", "Tar Moon contested";
   - the selected ship's panel, when it holds a claim, says which body and
     what it yields.
   The yield shown is whatever the simulation pays (publish it in the
   snapshot; never recompute it in the renderer with its own rules). This
   item may touch `rom-app` (snapshot and event line) as well as
   `rom-render`.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`); universe generation deterministic (run twice, equal
hashes); re-pinned hashes explained; screenshots you looked at: the home
system whole, a system with a belt and moons, the zoom from the planet out to
the whole system in steps showing the hand-over, and a resource body hovered (type, resource, yield, claim state) with the event line after a claim. Commit on the branch (stage
by name), do not push or merge; final message with commands and real output,
screenshots, files changed, deviations, anything not verified.
