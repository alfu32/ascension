# Task 115 — Ice has its own bluish colour

Data only (`db/seed.sql`, appended). Implementer: the orchestrator (a few seed lines).

## Why — the owner (2026-11-21)

Task 114 found that Ice and Snowfield look the same: both used `mat.ice` (#e8eef2). Asked whether
ice should get its own bluish colour, which also changes the polar caps on every planet, the owner
answered: "ice get its own bluish colour, which also changes the polar caps on every planet".

## Build

- New material `mat.snow` (program `snow`, base colour #e8eef2, the old white).
- `ter.snow` and the snow on mountain peaks (`mat.mountain` `snow_material`) use `mat.snow`.
- `mat.ice` (Ice brushes and polar caps) becomes #b3d3e8, a pale glacier blue.

## Done when

Gate green; playthrough hashes unchanged (flat maps have no ice); a screenshot shows ice and
snow apart (`app/docs/img/115-ice-snow.png`).
