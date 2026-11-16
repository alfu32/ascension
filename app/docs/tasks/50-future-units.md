# Task 50 — Models: Future units: automation and fusion

Implementer: Claude agent (moved off Codex by the owner, 2026-11-16). Model files in
`db/models/` (and the `rom-kit` part kit and motion, `rom-app` tests, only as needed).
Review round 10.

**Read `model-card-common.md` first**: it holds the method, rules, review
material, tests, the done-when list and how to finish. This card lists only
what to build.

## Units

Automation (`@automation`):

| Model | Silhouette |
| --- | --- |
| `unit.worker_drone` | small hovering or legged worker robot with tool arm |
| `unit.combat_robot` | humanoid combat robot, rifle arm |
| `unit.hovercraft` | fast hovercraft, skirt, fans |
| `unit.robot_tank` | unmanned low tank, sensor mast |
| `unit.cargo_drone` | large cargo quadcopter or crawler |
| `unit.drone_fighter` | small unmanned jet |
| `unit.drone_bomber` | flying-wing drone |
| `unit.drone_cruiser` | unmanned surface warship |

Fusion (`@fusion`):

| Model | Silhouette |
| --- | --- |
| `unit.exo_trooper` | powered exoskeleton, heavy rifle |
| `unit.pulse_gunner` | energy weapon, backpack power cell |
| `unit.railgun_team` | long railgun on a tripod, two figures |
| `unit.railgun_battery` | vehicle-mounted railgun |
| `unit.laser_battery` | anti-air laser emitter on a vehicle |
| `unit.fusion_tank` | heavy hover or tracked tank with glowing parts |
| `unit.interceptor` | sleek atmospheric interceptor |
| `unit.hover_frigate` | hovering warship over water |

Motion: hover units float and tilt, robot legs walk, energy weapons fire a
beam or bolt (short-lived glowing part), drones hover.

## Done (task 50)

- 16 unit looks in `db/models/unit.*@automation.sql` and `unit.*@fusion.sql`, in white composite
  and graphite with cyan light strips; fusion-age weapons and drives glow violet (`mat.plasma`).
  New self-lit material `mat.laser` (red) for the laser beam.
- No new shapes or motion knobs. Robots are figures (they walk and work as figures do): the
  worker drone a stocky robot with a box head, cyan visor, antenna, battery pack and a tool lance
  it swings at work; the combat robot a taller one with a red visor, shoulder plates and a rifle
  arm it levels and fires in bursts (`attack` 6). The exo-trooper is a figure of heavy build in a
  team-coloured yoke, box helmet and tall power pack with a heavy rifle; the pulse gunner wears a
  power cell on the back and fires a burst of three plasma bolts.
- Hover units carry their own height and bob and tilt on `bank`: the hovercraft (skirt, cabin,
  two ducted fans: `torus` rings round `propeller`s, rudders), the fusion tank (lift pads as flat
  `wheel`s of plasma, twin plasma cannons that recoil and fire two bolts), the hover frigate (a
  hull on two pontoons with glowing undersides, radome, railgun turret, plasma drives). Drones
  hover like task 48's: the cargo drone a large quadcopter with guarded rotors and a slung
  container.
- The robot tank is a low tracked tank with no hatches and a sensor mast; the drone cruiser an
  unmanned warship with a stealthy pyramid superstructure, integrated mast, vertical launch cells
  and a drone pad. The drone fighter is a small graphite jet with no canopy (3.6 tiles up, gun
  bursts); the drone bomber a graphite flying wing (4.3 up, bombs on the flash joint); the
  interceptor a white needle-nosed jet with canards and plasma drives, flying highest (4.6 up,
  `bank` 28) and firing plasma bolts.
- Railguns fire on the flash joint: a plasma flash and a bright streak ahead of the muzzle (the
  railgun team's tripod gun, crewed by a standing loader with a power cell and a seated gunner;
  the battery's long rails on a tracked chassis, which recoil). The laser battery's beam is a
  thin red cylinder from its emitter, shown only at the shot. Shot effects stay within about
  2.5 tiles of the muzzle, because a model's bounds set its selection and health-bar size.
- Team colour 16–23 % on vehicles, ships and aircraft; 29–36 % on robots and foot figures.
