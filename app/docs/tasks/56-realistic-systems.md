# Task 56 — Star systems that look like star systems

Crates: `rom-mapgen` (universe layout), `rom-render` (system scene, zoom
scale), `rom-app` only if a test needs it, docs. Starts after task 53 is
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

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`); universe generation deterministic (run twice, equal
hashes); re-pinned hashes explained; screenshots you looked at: the home
system whole, a system with a belt and moons, the zoom from the planet out to
the whole system in steps showing the hand-over. Commit on the branch (stage
by name), do not push or merge; final message with commands and real output,
screenshots, files changed, deviations, anything not verified.
