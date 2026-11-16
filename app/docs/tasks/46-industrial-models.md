# Task 46 — Models: Industrial

Implementer: Claude agent (moved off Codex by the owner, 2026-11-16). Model files in
`db/models/` (and the `rom-kit` part kit and motion, `rom-app` tests, only as
needed). Review round 6.

**Read `model-card-common.md` first**: it holds the method, rules, review
material, tests, the done-when list and how to finish. This card lists only
what to build.

Era family **industrial**: red brick, iron, glass skylights, sawtooth roofs,
chimneys, corrugated iron, rail.

## Buildings

- **First looks** (`@industrial`): `bld.oil_well` (2×2 timber or iron
  derrick, pump, storage tank), `bld.refinery` (4×4 distillation columns,
  tanks, pipes, flare stack), `bld.factory` (4×4 sawtooth-roof hall,
  chimneys), `bld.vehicle_plant` (4×4 assembly hall, large doors, crane),
  `bld.airfield` (6×6 runway strip, hangar, control tower, windsock).
- **Industrial look of every earlier building**, `@industrial`: city, farm,
  woodcutter's camp, library, market, barracks, tower, dock, granary, lumber
  mill, mine, university, temple, stable, siege workshop, smelter, fort,
  bastion, star fort — 19 models.

## Units (`@industrial`)

| Model | Silhouette |
| --- | --- |
| `unit.rifleman` | bolt rifle, peaked cap, greatcoat |
| `unit.machine_gunner` | crew-served machine gun on a tripod, two figures |
| `unit.anti_tank_rifleman` | very long anti-tank rifle with bipod |
| `unit.medic` | armband, satchel, stretcher or kit |
| `unit.armoured_car` | four-wheeled armoured car with turret |
| `unit.light_tank` | small tracked tank with riveted turret |
| `unit.field_gun` | wheeled field gun with shield, crew |
| `unit.biplane` | two-wing propeller fighter |
| `unit.ironclad` | low steam warship, funnel, turret |
| `unit.supply_truck` | early truck with canvas cover |

## Kit and motion (first vehicles and aircraft)

New shapes as needed (each tested): `track` (tank track run with road
wheels), `turret`, `wing`, `propeller`/`rotor`, `funnel`. Motion: tracks and
wheels roll with movement, turrets traverse and guns recoil on attack,
propellers spin, aircraft fly at altitude with a gentle bank while moving
(check how the renderer places air units today and report it), funnels may
smoke later (not required). A machine gun fires bursts (repeated flashes).

## Done (task 46)

- 24 building looks and 10 units in `db/models/*@industrial.sql`, shared
  `mesh.house_industrial` and `mesh.silo_industrial`; materials red brick,
  corrugated iron, cast iron, glass, tarred felt, olive drab, khaki, rubber,
  tarmac, cinders, rusted iron and crude oil.
- Kit: shapes `track` and `propeller`; part knobs `traverse` (turret),
  `bank` (aircraft) and `flash` 2–5 (burst); figure `attack` 7 (brace at a
  mounted gun) and `work` 2 (tend). Turrets, wings and funnels are boxes and
  cylinders; no separate shapes were needed.
- The renderer draws non-spacecraft units on the ground, so the biplane's
  model carries its own height (2.7 tiles).
- A crew of foot figures (the machine gunner) has the figure budget per figure.
