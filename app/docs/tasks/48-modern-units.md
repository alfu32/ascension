# Task 48 — Models: Modern and information units

Implementer: Claude agent (moved off Codex by the owner, 2026-11-16). Model files in
`db/models/` (and the `rom-kit` part kit and motion, `rom-app` tests, only as needed).
Review round 8.

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

## Done (task 48)

- 22 unit looks in `db/models/unit.*@modern.sql` and `unit.*@information.sql`; materials
  haze grey (warships, jets), dark green (information-age vehicles) and black composite
  (carbines, sights, drones). The strategic bomber is anti-flash white.
- No new shapes or motion knobs. Rotors and the missile launcher's and cruiser's search radars
  are `propeller` parts pitched −90°. A missile, rocket or bomb shows only at the shot, on the
  `flash` joint: a flash, the missile ahead of the tube and a smoke trail widening back
  towards it (a burst of 4 for the rocket salvo, vertical from the cruiser's cells). Drones
  carry their own height (recon 1.15, repair 1.55 tiles) and hover with a small `bank`.
- Aircraft carry their own height: fighter 2.7, bomber 2.9, jet fighter 3.8, strategic
  bomber 4.2 tiles; jets bank harder (24° against 18°). The rules give every aircraft the
  same speed (`et_movement.speed` 4000), so "faster" is not shown; that is a rules change.
- A gun's `recoil` stays within the drift the attached-parts test allows (0.15 tiles).
- Figures keep to 300 triangles each: one six-sided helmet dome, a stock and receiver as one
  box. Helmets of crew left out of the far version are left out with them.
- Team colour 15–24 % on vehicles, ships and aircraft; 29–38 % on foot figures and crews
  (tunics, partly covered by webbing and plate carriers).
