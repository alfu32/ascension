# Task 51 — Models: Space age

Implementer: Claude agent (moved off Codex by the owner, 2026-11-16). Model files in
`db/models/` (and the `rom-kit` part kit and motion, `rom-app` tests, only as needed).
Review round 11.

**Read `model-card-common.md` first**: it holds the method, rules, review
material, tests, the done-when list and how to finish. This card lists only
what to build.

Starts after task 43 (ships in orbit, drawn in the planet view and the
system view, fire from orbit) **and** card 50 are merged.

## Buildings

- `bld.spaceport@starfaring` (8×8: launch pad with gantry, landing pads,
  hangars, control tower; the largest landmark).

## Units

Ground (`@<their first age>`): `unit.grav_skimmer` (starfaring, hover bike or
skimmer), `unit.marine` (starfaring, space marine with helmet visor),
`unit.beam_trooper` (expansion, beam rifle), `unit.grav_tank` (expansion,
floating tank), `unit.shock_marine` (abundance, heavy armour).

Starships: `unit.survey_ship` and `unit.corvette` (starfaring),
`unit.space_frigate` (expansion), `unit.space_cruiser` (abundance),
`unit.battleship` (galactic). Each larger than the last, told apart by
silhouette in the planet view (in orbit) and in the system view; engines
glow (task 44's emissive material if merged); ships that bombard from orbit
(task 43) show the shot.

Space resource nodes (`node.ore_asteroid`, `node.crystal_asteroid`,
`node.tar_moon`) if the space renderer draws node models; otherwise report
what it would need.

Check every starship in both the planet view (in orbit) and the system view.
