# Task 47 — Models: Modern and information buildings

Implementer: Claude agent (moved off Codex by the owner, 2026-11-16). Model files in
`db/models/` (and the `rom-kit` part kit, `rom-app` tests, only as needed). Review round 7.

**Read `model-card-common.md` first**: it holds the method, rules, review
material, tests, the done-when list and how to finish. This card lists only
what to build.

Era family **modern** (modern, information): concrete, glass curtain walls,
flat roofs, steel frames, antennas and satellite dishes.

## Buildings

- **First looks** (`@modern`): `bld.bunker` (2×2 low concrete pillbox with
  firing slits) and `bld.fortress` (6×6 concrete fortress: bunkers, walls,
  radar mast, gun emplacements; a landmark).
- **Modern look of every earlier building**, `@modern` (applies through the
  information age): city, farm, woodcutter's camp, library, market, barracks,
  tower, dock, granary, lumber mill, mine, university, temple, stable, siege
  workshop, smelter, fort, bastion, star fort, oil well, refinery, factory,
  vehicle plant, airfield — 24 models.
- Optional, if the information age needs its own accent on landmarks (city,
  university, airfield): `@information` looks with antennas, dishes and
  glass; say in the report which you added.

## Done (task 47)

- 26 building looks in `db/models/*@modern.sql` (`bld.bunker` and `bld.fortress` first looks,
  24 restyles), one optional `bld.city@information` (two glass towers with a spire and dishes;
  the university and airfield keep their modern look through the information age), shared
  `mesh.house_modern` (flat-roofed flats) and `mesh.silo_modern` (concrete stave silo).
- Materials concrete, precast panels, glass curtain wall (the `timber` program in glass blue:
  vertical mullions; floor bands are thin parts), ribbed steel cladding (`timber` in grey),
  painted steel, sandbags and `mat.signal_red` (self-lit warning lights on masts and chimneys).
- No new shapes. Buildings may not move (a test holds every building to one joint), so the
  fortress and airfield radars are fixed antennas, guns do not recoil or flash, and the
  vehicle plant's tank has box tracks rather than the moving `track` shape. The bastion and
  star fort reuse the industrial earthworks as their own `.mesh` copies, in concrete.
- Team colour 14–25 % on every look (roofs, fascias, crowns, stripes and flags). The
  export-all test now only asks that some starters remain (84 after this card).
