# Task 68 — Sight, defences that fire, and ships in orbit

Crates: `rom-sim` (target acquisition, sight from stats, orbit movement,
landing, boarding, interplanetary trade), the seed and its generator
(`app/db/content/generate.py`; sight values, sight modifiers, two new
units), `rom-render` (shots from buildings, orbit drawing), `rom-app` (the
Land button and right-click in orbit), the two new models in
`app/db/models/`, `rom-save` for new state, docs. Schema only if a needed
column or stat is missing (then deliberately: lint views empty, seed loads,
documented in `app/docs/database.md`). No change to `rom-ai`, `rom-cli`
(task 67 adapts the computer players afterwards). Runs before task 67 (67
tunes the computer players on top of these rules). Task 49 adds model files
beside it and touches only test lines in `rom-app`. Implementer: Claude
(Opus). Large card: commit each part as it is done.

## Why — the owner's rules (2026-11-16, after playing 0.0.59)

1. A unit's base line of sight depends on the unit, and upgrades extend it.
2. Cities, towers and forts must attack enemies.
3. A building's sight is larger than a unit's, set per building type, grows
   with the building's size; towers, cities and forts see further still, in
   proportion to their attack range.

What the game does today (measured by the orchestrator on 0.0.60):

- `entity_type.los`: nearly every unit sees 6 tiles (infantry 5, scouts and
  ships 9, citizens 2); every building sees 12 tiles whatever its footprint
  (2×2 to 8×8); defences 14, the ground battery 16.
- The only `los` modifier is a race bonus (`race.avian`, +25 %); no
  technology extends sight.
- Armed buildings: city, tower, fort range 10 tiles; bastion, bunker,
  citadel, fortress, star fort, turret 10; ground battery 14.
- `World::acquire_idle` (`rom-sim/src/stance.rs`) only gives targets to
  entities with **no orders**, and training and research are orders on the
  producer (`production.rs`). So a city that is training or researching,
  which is almost always, never fires. Towers and forts do fire in the
  simulation (test `ranged_building_attacks_enemy_in_range_only`), but shots
  between ground objects may not be drawn (the tracers of task 66 are for
  shots to and from orbit only); check what the owner sees.

## Build

1. **Unit sight by type.** Give each unit type a base sight that fits its
   role (for example scouts and aircraft far, cavalry and ranged further than
   melee foot, siege short, ships and spacecraft by size and age); citizens
   no longer near-blind. Values from the content generator, by rules of
   thumb written down in a "Sight" section of `app/docs/database.md`, never
   by key in code.
2. **Sight upgrades.** Technologies that extend sight, as `modifier` rows on
   `los` (pct or add), on fitting existing technologies of each age and line
   (scouting, optics, radar, satellites, sensors…); add technologies only if
   no existing one fits, and say which. Check the stat engine applies `los`
   modifiers to fog of war (it applies the race bonus) with a test.
3. **Building sight by type and size.** Base sight per building type, larger
   than any ground unit's of its age, plus a share that grows with the
   footprint, so a 6×6 sees further than a 2×2. Armed buildings (city,
   tower, fort and the other defences) see at least their weapon's maximum
   range times a ratio from data (for example 150 %), so they spot what they
   can hit; an upgrade that extends their range extends their sight with it
   (modifier rows on both stats).
4. **Defences fire.** An armed building fires at visible enemies in range
   while it trains, researches or is otherwise busy with production; an
   armed building's weapon never waits for its queue. Firing stays
   stand-ground (buildings never move). Keep target choice in id order.
   Draw shots from buildings: an arrow or shot line for the early ages, a
   tracer for guns, matching how units' shots are drawn; check with a
   screenshot that a tower, a city and a fort visibly fire at intruders.

Every simulation change gets run-twice-equal-hash tests (a city training
while an enemy walks in and is shot; a tower with an upgraded range seeing
and hitting further; fog revealing with sight upgrades) and goes into the
snapshot and `rom-save` if it adds state. Explain any change to the
playthrough hash and to `rom duel` results (one 20-seed run before/after).


## Part 2 — ships in orbit (the owner's rules, 2026-11-16)

Read first: `app/docs/world-model.md`, tasks 53 (space navigation), 54
(landing and colonies), 57 (one camera), 58 (orbital combat: the `orbit`
layer, `et_movement.climb_ticks`, craft, carriers, batteries).

5. **Geosynchronous orbit.** A ship or craft in a planet's orbit stays above
   the same surface point; it does not drift around the planet. Rotating the
   planet in the view carries the ships with it.
6. **Right-click in orbit moves within orbit.** A ship in orbit ordered to
   move (right-click on the planet or on the orbit) goes to the orbit point
   above the clicked surface point and stays in orbit; it never descends.
7. **Patrol in orbit.** A ship in orbit ordered to patrol circles the planet
   on the great circle through its current point and the order point,
   continuously, fighting like `attack_move` (task 65's `patrol`), until
   given another order.
8. **Land on the vertical.** A Land button (in the order panel for ships in
   orbit that can land) brings the ship straight down to the surface point
   below it, if that point is passable for it (a clear message and refusal
   otherwise). Take-off keeps its existing button.
9. **Boarding only on the ground.** Troops and personnel embark and
   disembark only while the ship is landed; orders to board a ship in orbit
   are refused with a reason. (Orbital craft and carriers from task 58 keep
   their own launch and recovery in orbit.)
10. **A trade vessel.** A new space freighter (starfaring age, from the
    spaceport) runs a trade route between two of the player's (or allied)
    cities on different planets, like the caravan's route on the ground:
    income by distance from the existing trade stats, travelling by the
    existing space travel, landing and taking off at each end. New unit rows
    in the generator; its model as a first look following
    `model-card-common.md` (it goes into a later review round).
11. **Ship sight.** Ships see much further than ground units: base sight in
    proportion to their altitude (orbit above ground) and their size.
12. **Satellite.** A new space unit, the satellite: built at the spaceport,
    launched straight into orbit above its planet, no weapon, small, very
    large sight (most of the visible horizon — choose a number from the
    planet's size, written down), moves only within orbit (items 6–7), cannot
    land. Its model as a first look like the freighter's.

Determinism tests for every new behaviour (an orbit hold over many ticks;
moving between orbit points; an orbit patrol loop; landing on the vertical
and refusal over water; boarding refused in orbit; a freighter round trip
and its income; satellite sight), each run twice with equal hashes, new
state saved and restored.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
complete; a table of base sight per unit type and per building type with
footprint and weapon range, and the sight technologies with their bonus;
screenshots you looked at (Read the PNG) in `app/docs/img/` with a `sight-`
prefix: a tower, a city (while training) and a fort firing at intruders, and
the fog with a unit's sight before and after an upgrade; and with an
`orbit-` prefix: ships holding over one point after the planet turned, an
orbit patrol path, a ship landing straight down, a freighter on its route,
and a satellite's sight revealing the horizon. Commit on the
branch (stage by name, WIP commits welcome), do not push or merge; final
message with commands and real output, files changed, schema changes,
deviations, anything not verified.
