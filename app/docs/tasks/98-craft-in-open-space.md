# Task 98 — Orbital craft launched in open system space

Crates: `rom-sim` (craft state, orders), `rom-ai` (using it) — or split in two if they must run
in parallel with other work on those crates. Implementer: Claude (Opus). Planned; not started.

## Why — the owner (2026-11-18)

"I can't use orbital fighters outside of a planet (potentially, or can we)?" Today craft fight
away from home only aboard a carrier, launched into the orbit of a habitable planet
(`app/docs/unit-series.md` section 5). The owner chose option 1 for task 92 and option 2 as
this card.

## Build

Option 2 of section 5: a carrier in free system space launches its craft beside it; they fight
in the system plane within a leash of their carrier (a game constant or an `et_container`
column, about 15 tiles; a schema change goes through the orchestrator first) and come back
aboard there. When the carrier jumps or dies, craft outside are left: they may land on a
habitable planet of that system or are lost. Validation, save round trip, the computer
players' use (fighter cover for a fleet charging at the edge and over resource bodies).

## Done when

- Gate green from `app/`; repeat → same hash; save round trip; fight matrix unanswered=0.
- Screenshots you looked at: craft launched in open space around a carrier.

Commit on the branch (stage by name); do not push or merge.
