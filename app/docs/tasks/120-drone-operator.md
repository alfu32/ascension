# Task 120 — The drone operator: one infantry unit with a cloud of kamikaze drones

Issue #3 (owner, 2026-11-21), part 5. Crates: `rom-sim` (the drone cloud), `rom-render` (drawing
the cloud), `app/db/schema.sql` + seed (a component table; schema change authorised here), one
model file, `rom-ai` through data, docs. Implementer: Claude (Opus). Starts after tasks 117 and
118 are merged (both touch `rom-sim`).

## Why — the owner, verbatim (issue #3)

"For the infantry, I would have drone units. By that I mean that the unit is the operator and it
also has a cloud of drones, let's say 16. And they will be a bit similar to archers in the sense
that they, well, when you move a unit or attack an unit, the drone cloud part will get there
first and actually the unit will stay behind at the maximum distance possible. So we will have a
maximum distance between the command unit and the drone unit. Of course they are not separate
units. They are a single unit from our point of view. And a cloud of drones will attack. Each
drone in the cloud can attack. One drone can kill or severely injure an enemy infantry unit and
it will take, let's say, ten drones to destroy an armored vehicle. Also, the drones will be
replaced continuously. So basically the drones themselves are kamikaze drones and the unit, once
one of the drones gets destroyed, will issue another drone that will join the cloud to destroy a
drone unit, you will have to destroy the command unit, the, well, the little man that commands
the drone cloud unit"

## Design

- **One unit**, `unit.drone_operator`, foot infantry, **information age** (the first age with
  drones in the game; confirmed by the owner in issue #3: "Yes, agreed"). Its hit points are the
  operator's; killing the operator kills the unit and its cloud.
- **The cloud** is part of the unit's state: `drones` (count, max `et_drone_cloud.capacity` =
  16), the cloud's position (milli-tiles), a leash `et_drone_cloud.leash` (e.g. 12 tiles): the
  cloud may be at most that far from the operator.
- **Moving and attacking**: on a move order the cloud flies ahead to the destination, the
  operator follows; on an attack order the cloud goes to the target and the operator stops at
  the leash distance from it (like a ranged unit keeping range), never closer than needed.
- **The attack**: each strike launches one drone from the cloud at a target within the cloud's
  reach (`et_drone_cloud.strike_range`, short), every `et_drone_cloud.strike_ticks`. The drone
  is spent (kamikaze) and deals `et_drone_cloud.strike_damage` through the normal damage path
  (armour classes, modifiers). Tune damage so **one drone kills or nearly kills an infantry unit
  of the same age** and **about ten destroy an armoured vehicle** (main battle tank of the
  information age) — show the numbers in the report.
- **Replenishing**: the operator issues a new drone every `et_drone_cloud.rebuild_ticks` while
  below capacity (free, or a small resource cost per drone as a stat if balance needs it). A new
  drone joins the cloud at the operator and flies to it.
- **Shooting drones down**: the cloud can be hit by weapons that hit aircraft (anti-air,
  flak, missile teams, machine guns if they are flagged so): each hit destroys drones (one per
  `et_drone_cloud.drone_hp` of damage), not the operator. Ground weapons that cannot hit air
  must find the operator.
- **Counter rule** (card 71): the operator outranges infantry through its cloud, so it needs an
  answer in its own age: anti-air that thins the cloud, artillery or snipers that reach the
  operator. Add it to the fight matrix and make it pass.
- All of this is integer, deterministic, data-driven (a component table `et_drone_cloud`, no
  branching on the key), saved and hashed.

## Build

1. Schema: `et_drone_cloud` (entity type → capacity, leash, strike_range, strike_ticks,
   strike_damage, rebuild_ticks, drone_hp, drone_speed); save columns for the cloud state; lint
   views empty; `database.md` updated.
2. `rom-sim`: cloud state, movement with the leash, strikes, replenishing, being shot down;
   unit tests and a twice-run hash scenario.
3. Snapshot and `rom-render`: draw the cloud as up to 16 small craft swirling around the cloud
   point (presentation offsets may be derived from the tick and the unit id, no randomness
   outside the sim), a streak for each strike; the operator uses an infantry model with a
   controller.
4. Seed and model file; `rom-ai` uses it through army-composition data.
5. Docs: `docs/` unit lists, `computer-player.md` (fight matrix row).

## Done when

- Gate green; fight matrix `strengths=255 unanswered=0`; duel 20–0; new hashes reported with
  what moved.
- A self-ending scripted fight (operators against riflemen, against a tank, against anti-air)
  with the outcome numbers and a screenshot you looked at: `app/docs/img/120-drone-cloud.png`.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`,
`app/docs/computer-player.md` (counter rule), `rom-sim` combat (projectiles, damage, armour,
ranged units keeping range, aircraft targeting), the seed rows of `unit.gun_drone`,
`unit.recon_drone`, `unit.missile_team`, `unit.main_battle_tank`.

Memory rules, logs and worktree as in card 117 (`t120/`, `../rise-of-man-worktrees/120-drone-operator`,
branch `task/120-drone-operator`). Do not merge, tag or push.
