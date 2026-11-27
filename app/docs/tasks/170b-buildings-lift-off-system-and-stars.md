# Task 170b — Buildings lift off: through the star system (age 11) and to other stars (age 13)

The second half of task 170 (`170-buildings-lift-off.md`, owner's words there). Task 170a
(merged in 0.157.0) lets an upgraded building lift off, fly across its own planet and land.
This card takes it into space.

Ages are counted from 0:
- **within the star system** from age 11, the Starfaring age (`age.expansion`);
- **to other stars** from age 13, the Age of Galactic War (`age.galactic`). Jumps between
  clusters also need the galactic tier (task 163), which is open by then.

GitHub issue #173 (shared with 170a; it closes when 170b is released).

## What 170a left (read its report section in `docs/database.md`, "Buildings that lift off (task 170)")

- `crates/rom-sim/src/airlift.rs`: `BuildingFlight`, saved in `entity_lift` (save format 28).
- Orders `lift_off`, `fly` and `touch_down`, and `World::check_touch_down`, which applies the
  construction placement rules and task 172's forest rule.
- The tag `liftable`, the stat `lift_off`, and one `tech.lift_engines.*` research per
  building line.
- The kit part `kit.lift_engine`, with exhaust and a dust ring.
- The computer player's escape rule in `crates/rom-ai/src/airlift.rs`.

## What changes

1. **Stages.** `BuildingFlight` gains a stage: air (as now), climbing to orbit, orbit, system
   space, in transit (charging and jumping). The save format bumps.
2. **Reach, from data.** A new stat `lift_reach`, on the tag `liftable`:
   - 0 means the planet only;
   - 1 means the star system: +1 from `tech.age.expansion`;
   - 2 means other stars: +1 from `tech.age.galactic`.

   No key branches.
3. **System travel** (reach ≥ 1):
   - the building climbs to orbit (reuse `begin_orbit`) and flies slowly in the system plane,
     like a slow transport, to another planet's orbit;
   - it comes down with `touch_down` there. It may land only where the owner has a city on
     that planet, or on a planet nobody has a city on;
   - the landing check is the same as on the home planet.
4. **Jumps** (reach ≥ 2): along routes, with the same rules as starships:
   - `route_plan` and `jump_allowed` (the cluster tier);
   - the charge;
   - inhibitors (task 70);
   - the arrival strike;
   - refused while the building is below half its hit points.
5. **In space:**
   - space weapons can hit it, and ground weapons cannot;
   - its queue stays paused, as in the air;
   - it shows in the system view and on the star map like a ship (icon and label);
   - the selection panel says where it is going.
6. **Interface:**
   - the landing ghost works on another planet: select the building, open that planet and
     click;
   - a right click on a planet or star in the space views sends it there.
7. **Computer players:** if a planet is lost, they move a production building to a safe own
   planet in the same system (reach 1) or, with reach 2, the nearest safe own system. They never
   send buildings into combat.

## Crates

`rom-sim` (`airlift.rs`, orbit, transit), `rom-save`, seed (the stat and modifiers),
`rom-render` (a building in orbit and in space), `rom-app` (orders from the space views, the
ghost on another planet), `rom-ai`.

## Done when

- **Gate green from `app/`:**
  - fmt, clippy `-D warnings`, workspace tests (≥ main plus this card's tests);
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views;
  - playthrough and ascension hashes unchanged, or explained.
- **Tests** (scenarios run twice with equal hashes):
  - refused beyond its reach;
  - flight to another planet in the system, landing there;
  - landing refused on a planet with another player's city;
  - a jump that respects the cluster tier and is refused below half hit points;
  - save and load during each stage.
- **Screenshots** `docs/img/170b-*.png`, each opened with Read:
  - a building in orbit;
  - a building crossing the system;
  - one landing on another planet.
- Report as AGENTS.md says. Logs only in the scratchpad's `t170b/`; never write `gate*.sh`.
