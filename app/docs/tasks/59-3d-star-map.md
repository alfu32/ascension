# Task 59 — A 3D network of star systems

Crates: `rom-mapgen` (universe layout), `rom-sim` (route lengths, transit
positions), `rom-render` and `rom-app` (star-map drawing and navigation),
schema only if a system position is stored there, docs. Starts after tasks 57
and 58 are merged (they touch the same crates). Implementer: Claude (Sonnet).

## Why — the owner (2026-11-15)

"Ultimately the galactic/universal map could be 3D (a 3D network of star
systems)."

Today `rom-mapgen/src/universe.rs` places systems on a disc (`x`, `y`
integers); routes join them and their travel time comes from their length
(`space.route_ticks_per_unit`). Task 57 builds one camera from the street to
the star map and keeps it ready for a third coordinate.

## Build

1. **Systems in a volume.** Systems get an integer height `z` (seeded,
   deterministic, rule 1): a thick disc or a lens, so the network reads as
   3D from an angle yet stays navigable; home systems keep their fairness
   (comparable route distances and neighbours for every player). Routes are
   chosen and measured in 3D (integer length; travel ticks from it).
2. **Simulation.** Route lengths, transit progress and anything that uses a
   system's position use the 3D position. Integers only; iteration in id
   order; determinism scenarios run twice with equal hashes.
3. **Seeing and moving.** The star map is drawn in 3D with task 57's camera:
   orbit around the map freely, routes as lines in space, depth cues (size,
   fade, a faint grid or halo per system's height so the player reads which
   systems are near), picking and hover that work off the plane, ships in
   transit along their 3D routes, fog per system as today. The minimap shows
   a readable projection.
4. **Data.** If the system position is stored in the schema or the save,
   add the column deliberately (lint views empty, seed loading, documented
   in `app/docs/database.md`).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); universe
generation deterministic (run twice, equal hashes) and fair; playthrough
complete (explain any hash change). Screenshots you looked at: the star map
seen from above and from a low angle (depth visible), a ship in transit on a
route that climbs between systems, the zoom from a system out to the 3D
map. Commit on the branch (stage by name), do not push or merge; final
message with commands and real output, screenshots, files changed, schema
changes, deviations, anything not verified.
