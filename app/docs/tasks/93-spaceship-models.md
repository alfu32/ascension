# Task 93 — Higher-definition spaceship models

Crates: model files `app/db/models/`, `rom-kit` parts if new shapes are needed, the review page.
Implementer: Claude (Opus). Starts after task 92 phase 2 (which may change the space classes).

## Why — the owner (2026-11-18)

"Higher definition models for spaceships; there is never gonna be a whole lot of them, we can
afford a higher polygon budget."

## Build

1. Rebuild the looks of every ship that flies in space (survey ship, corvette, space frigate,
   space cruiser, battleship, transport, carrier, space freighter, satellite, orbital fighter and
   bomber, and any class task 92 adds) with a **near budget about four times today's** (state
   each model's triangles before and after) and a far version that keeps today's cost.
   Detail that reads at the system view's distance: hull plating breaks, bridge and
   superstructure, engine bells with glow, turrets that face their arcs, hangar mouths on the
   carrier, antenna and sensor masts, running lights. Each class keeps a silhouette you can
   tell apart at a glance (size order corvette < frigate < cruiser < battleship), team colour
   on readable panels.
2. Follow `app/docs/art-direction.md` and `app/docs/models-in-meshcraft.md` (parts, materials,
   briefs); nothing from fiction or other games.
3. These models are approved ones: their fingerprints change, so they go through the review loop
   (`app/docs/art/review-page/`, a new round). Do not edit `approved.toml`; the orchestrator
   asks the owner.
4. Measure the system view with 60 ships in sight (`--bench`), before and after.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/art-direction.md`,
`app/docs/models-in-meshcraft.md`, `app/docs/art/review-page/HANDOVER.md` (if present) and the
build script, the current ship model files.

## Done when

- Gate green from `app/` (the approved-model test will list the changed ships: that is
  expected; say so).
- The gallery manifest and screenshots of every ship near and far, plus a system view with a
  mixed fleet, in `app/docs/img/`, looked at.
- Bench numbers before and after.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, triangle counts, deviations, anything not verified.
