# Task 48 — Models: Modern and information units

Implementer: Codex (`gpt-6.1-sol`, high). Crates: seed (and `rom-render`
part kit and motion, `rom-app` tests, only as needed). Review round 8.

**Read `model-card-common.md` first**: it holds the method, rules, review
material, tests, the done-when list and how to finish. This card lists only
what to build.

## Units

Modern (`@modern`):

| Model | Silhouette |
| --- | --- |
| `unit.infantry` | helmet, assault rifle, webbing |
| `unit.heavy_machine_gunner` | heavy machine gun, crew of two |
| `unit.bazooka_team` | shoulder rocket launcher, loader |
| `unit.flak_gun` | towed anti-aircraft gun, raised barrels |
| `unit.howitzer` | towed howitzer, split trail, crew |
| `unit.scout_car` | light four-wheeled scout car |
| `unit.tank` | tracked tank, long gun, cast turret |
| `unit.lorry` | military lorry with canvas back |
| `unit.fighter` | propeller monoplane fighter |
| `unit.bomber` | twin- or four-engine bomber |
| `unit.destroyer` | grey destroyer, gun turrets, funnels |

Information (`@information`):

| Model | Silhouette |
| --- | --- |
| `unit.assault_infantry` | modern kit, optics, helmet with goggles |
| `unit.missile_team` | guided anti-tank missile on tripod |
| `unit.missile_launcher` | vehicle with anti-air missile box |
| `unit.rocket_artillery` | truck or tracked rocket launcher |
| `unit.main_battle_tank` | low angular tank, long gun |
| `unit.gun_drone` | small tracked or wheeled armed drone |
| `unit.recon_drone` | small quadcopter |
| `unit.repair_drone` | drone with arm and toolkit |
| `unit.jet_fighter` | swept-wing jet |
| `unit.strategic_bomber` | large swept-wing bomber |
| `unit.missile_cruiser` | cruiser with missile cells and radar |

Motion: jets fly faster and higher than propeller aircraft, missiles launch
with a flash and a short trail, rotors spin, drones hover and bob.
