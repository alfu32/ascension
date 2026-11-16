# Task 68 — Sight by type and size, and defences that fire

Crates: `rom-sim` (target acquisition, sight from stats), the seed and its
generator (`app/db/content/generate.py`; sight values and sight modifiers),
`rom-render` (shots from buildings drawn), docs. Schema only if a needed
column or stat is missing (then deliberately: lint views empty, seed loads,
documented in `app/docs/database.md`). No change to `rom-ai`, `rom-cli`,
`rom-app` beyond test lines. Runs after task 65 merges and before task 67
(67 tunes the computer players on top of these rules). Implementer: Claude
(Opus).

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

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
complete; a table of base sight per unit type and per building type with
footprint and weapon range, and the sight technologies with their bonus;
screenshots you looked at (Read the PNG) in `app/docs/img/` with a `sight-`
prefix: a tower, a city (while training) and a fort firing at intruders, and
the fog with a unit's sight before and after an upgrade. Commit on the
branch (stage by name, WIP commits welcome), do not push or merge; final
message with commands and real output, files changed, schema changes,
deviations, anything not verified.
