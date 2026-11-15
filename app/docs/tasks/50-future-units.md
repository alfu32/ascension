# Task 50 — Models: Future units: automation and fusion

Implementer: Codex (`gpt-6.1-sol`, high). Crates: seed (and `rom-render`
part kit and motion, `rom-app` tests, only as needed). Review round 10.

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
