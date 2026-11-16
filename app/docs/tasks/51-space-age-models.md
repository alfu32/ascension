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

## Done (task 51)

- 11 looks in `db/models/`: `bld.spaceport@starfaring`; `unit.marine`, `unit.grav_skimmer`,
  `unit.survey_ship`, `unit.corvette` (`@starfaring`); `unit.beam_trooper`, `unit.grav_tank`,
  `unit.space_frigate` (`@expansion`); `unit.shock_marine`, `unit.space_cruiser`
  (`@abundance`); `unit.battleship` (`@galactic`). No new shape, motion knob or material.
- Earlier buildings keep their future look (from the automation age) through the space ages:
  one look per era family, as the era table says.
- The spaceport (7.3 × 7.3 on its 8 × 8 footprint, 5.9 tiles to the gantry light) keeps a rocket
  with boosters on its pad beside a tall gantry with service arms as its signature, with two
  vaulted hangars, two landing pads (a shuttle on one), a control tower, a dish and an operations
  block with a solar roof.
- Starships share the task 58 craft's look (steel hulls, iron engines, orange `mat.glow`
  drives) and grow with the ages, each with its own plan from above: survey ship (dish, sensor
  boom, solar wings; 2.5 tiles), corvette (narrow dart, one twin turret, canted fins; 2.8),
  frigate (two outrigger nacelles on pylons, two turrets; 3.4), cruiser (team-coloured
  hammerhead bow, bridge tower, radiator wings, three triple turrets; 3.9), battleship (stepped
  wedge, spinal plasma cannon, four turrets, five engines; 5.1). Turrets swing onto the target
  and flash across their muzzles with a bolt ahead of each barrel; the battleship's cannon
  recoils and fires a large plasma bolt. Seen in the planet view in orbit
  (`target/review11/orbit_planet*.png`, script `51-orbit-script.txt`) and bombarding a factory
  (`orbit_bombard.png`, `51-bombard-script.txt`), and in the system view (`orbit_system*.png`).
- Ground units: marine (round helmet, gold visor, life-support pack, carbine with a cyan bolt),
  beam trooper (crested helmet, capacitor ring on the back, long beam rifle, thin cyan beam),
  shock marine (heavy build, team-coloured breastplate, tall shield, plasma pistol in bursts),
  grav skimmer (hover bike on two glowing lift discs, seated rider, twin guns; banks), grav tank
  (wide floating hull on cyan lift rings, domed turret, beam lance that recoils; banks).
- Team colour 15–23 % on the spaceport, ships and vehicles (spaceport 16.5, survey ship 16.9,
  corvette 19.3, frigate 16.9, cruiser 16.5, battleship 15.4, skimmer 18.5, grav tank 23.4);
  33–36 % on the foot figures, whose armour covers part of the tunic. Budgets: spaceport 1,286 /
  290 triangles, ships and vehicles 226–760 / 110–150, figures 290–296 / 60–78.
- **Not done, needs the renderer:** the system view draws every ship as the generic box hull of
  task 53 (`Hull::push` in `rom-render/src/entities.rs`, called from `scene/space.rs`), so ships
  are told apart there only by size (`et_view_scale.system_pct`). Resource bodies
  (`node.ore_asteroid`, `node.crystal_asteroid`, `node.tar_moon`) are drawn as textured spheres
  in the resource's colour with its icon; they have no `entity_visual` and the space scene draws
  no node models. Both would need the space scene to draw composed models: the model meshes
  uploaded for the space pass (or shared with the world scene's `ModelGpu`), a model program lit
  by the star, the model per ship kind and per body (through `rom-app`'s looks, never by key) and
  a size rule for bodies. A renderer card, not a model card.
