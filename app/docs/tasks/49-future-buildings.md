# Task 49 — Models: Future buildings: automation and fusion

Implementer: Codex (`gpt-6.1-sol`, high). Crates: seed (and `rom-render`
part kit and motion, `rom-app` tests, only as needed). Review round 9.

**Read `model-card-common.md` first**: it holds the method, rules, review
material, tests, the done-when list and how to finish. This card lists only
what to build.

Era family **future** (automation and later): white composite, solar panels,
domes, light strips, smooth curved forms.

## Buildings

- **First looks**: `bld.auto_farm@automation` (hydroponic or robotic farm;
  keep the crop-row idea), `bld.auto_woodcutter_camp@automation`,
  `bld.auto_mine@automation`, `bld.auto_oil_well@automation`,
  `bld.auto_university@automation` (each recognisable as the automated form
  of its earlier building), `bld.defence_turret@automation` (2×2 turret on a
  base), `bld.fusion_plant@fusion` (4×4 torus reactor, cooling, glow),
  `bld.citadel@fusion` (6×6 domed fortress with shield pylons; a landmark).
- **Future look of every earlier building**, `@automation`: every building
  that exists before the automation age (city, farm, woodcutter's camp,
  library, market, barracks, tower, dock, granary, lumber mill, mine,
  university, temple, stable, siege workshop, smelter, fort, bastion, star
  fort, oil well, refinery, factory, vehicle plant, airfield, bunker,
  fortress) — 26 models.

Glow: if task 44 (emissive material) is merged, use it for light strips and
the reactor; otherwise bright lit colours, and say so.
