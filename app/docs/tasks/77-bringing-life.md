# Task 77 — Ascended players bring life to moons and asteroids

Crates: `rom-sim` (the "bring life" orders, body growth, painting, the new
habitable planet), `rom-mapgen` (dormant surfaces made before play),
`rom-render` and `rom-app` (painting tools, re-baking edited terrain on the
render thread), `rom-save`, schema and seed as needed, docs. Starts after
task 76 is merged. Implementer: Claude (Opus).

**Hard rule 9, as the owner confirmed it (2026-11-17):** surfaces are made
before play (rule 9 holds: nothing is generated during a match); painting is
a series of edits made by orders, saved and repeatable, with only the
touched terrain re-baked. That reading is now written into `AGENTS.md`.

## Why — the owner (2026-11-17)

Ascended players "can choose to bring life to inert planets, paint their
geography and resources at no cost whatsoever". Inert planets are the
resource bodies (moons and asteroids); "maybe grow the moon/asteroid radius
if too small".

## Build

1. **Dormant surfaces, made before play.** Every resource body gets a
   dormant surface at generation (rule 9): a cube-sphere of tiles at a size
   large enough to settle (a minimum habitable face size from data; small
   bodies are grown to it, and the system view draws them larger once they
   live), barren rock everywhere, no deposits. Nothing about it is shown or
   simulated while the body is inert.
2. **Bringing life.** An ascended player orders **Bring life** on a resource
   body it has seen. The body becomes a habitable planet: its resource node
   is retired (claims on it end, with an event line), its surface is
   activated, and it appears in the system as a small planet named after the
   body. No cost.
3. **Painting.** On a planet it brought to life, an ascended player paints,
   with brushes on the globe: land and water, elevation (hills, mountains),
   biome (grass, forest, desert, snow), and resource deposits (any kind the
   rules define, at chosen sizes). Each stroke is an order (rule 3), applied
   by the simulation at a tick, deterministic, saved; the renderer re-bakes
   only the touched terrain chunks on the render thread. Painting is
   allowed while no material player has a city on that planet.
4. **Settling.** A planet brought to life is an ordinary habitable planet:
   any player may land, found cities and fight there, and an ascended player
   may descend on it (task 76).
5. **Determinism and save.** Run twice with equal hashes: bringing a body to
   life; a sequence of strokes; save and restore mid-painting; a descent on
   the painted planet.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules); playthrough complete; bench of painting (frame
time while strokes re-bake); screenshots you looked at: an asteroid before
and after it is brought to life, painting land, water, mountains, forest and
deposits, a city founded on it; commit on the branch (stage by name), do not
push or merge; final message with commands and real output, files changed,
schema and save-format changes, deviations, anything not verified.
