# Task 49 — Models: Future buildings: automation and fusion

Implementer: Claude agent (moved off Codex by the owner, 2026-11-16). Model files in
`db/models/` (and the `rom-kit` part kit, `rom-app` tests, only as needed). Review round 9.

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

## Done (task 49)

- 34 building looks: first looks `bld.auto_farm`, `bld.auto_woodcutter_camp`, `bld.auto_mine`,
  `bld.auto_oil_well`, `bld.auto_university`, `bld.defence_turret` (`@automation`),
  `bld.fusion_plant` and `bld.citadel` (`@fusion`), and the future look of the 26 earlier
  buildings (`@automation`, applies through every later age). Shared `mesh.house_future`
  (housing pod with a glass band, solar panel and roof pod) and `mesh.silo_future`.
- Each automated building keeps its parent's signature in its automated form: the automated
  farm is a tiered vertical farm under pink grow lights beside outdoor crop beds tended by a
  gantry robot (the future farm keeps low crop beds under an irrigation gantry, a vaulted barn
  and a silo); the automated woodcutter camp a vaulted saw hall, conveyor, log piles and a
  gantry grab; the automated mine a drill derrick over its shaft, ore dome and a driverless
  hauler (the future mine keeps the headframe and spoil heaps); the automated oil well a slim
  derrick over a domed wellhead and capsule tanks (the future oil well keeps the beam pump); the
  automated university a glass-domed hall, a dark computing core ringed with light, an
  observatory dome and a dish.
- Materials white composite, composite panels, smart glass (dark teal, mullions), solar
  panels, graphite, composite paving, leafy crops, and three self-lit ones: cyan light strips,
  pink grow lights and violet plasma. Glow uses the `emissive` program (task 44); the smelter
  and the refinery keep `mat.glow` for molten metal and the flare.
- New shape `torus` in `rom-kit` (bounds and triangle test) for the reactor; no new motion
  (buildings stand still). Dishes are a shallow dome facing the sky over a deeper one behind it,
  because a single dome pitched away from the camera shows only its culled inside.
- The bastion and star fort reuse the modern earthworks as their own `.mesh` copies, in white
  composite.
- Team colour 14.5–26 % on every look (bands, fascias, roofs of round ends, domes of corner
  towers, coils, crowns and banners).
