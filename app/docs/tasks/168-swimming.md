# Task 168 — Swimming: citizens cross shallow water from the Industrial age, infantry from the Fusion age

GitHub issue #171.

Owner (chat, 2026-11-25), verbatim:
- "citizens can go(swim) through shallow bodies of water since age 5 or 6 (research swimming
  in docks )";
- "infantry units can swim from age 9".

Ages are counted from 0; the owner accepted the following on 2026-11-26:
- citizens from age 5, the Industrial age (`age.industrial`), through a research at the dock;
- infantry from age 9, the Fusion age (`age.fusion`).

## What changes

1. **Swimming (citizens).** A research at the dock, available from the Industrial age, lets
   citizens (`unit.citizen`, through its tags, never its key) cross shallow water:
   - **Where:** `ter.coast` and the shallow water terrain, whatever the terrain rows call it.
     Not deep water.
   - **Speed:** slowly, at about 40 % of their land speed.
   - **Limits:** they cannot gather, build or fight while in water. Attacked there, they take
     damage as usual.
2. **Swimming (infantry).** A research available from the Fusion age, at whichever building
   researches infantry upgrades, lets every unit tagged `infantry` cross the same shallow water,
   at about 50 %. They may fire from the water, with the usual accuracy. Vehicles still cannot
   swim.
3. **How, in data.** Today a unit's movement class fixes its terrain
   (`movement_class_terrain`). Express swimming as data:
   - preferred: a research that grants an extra terrain row for tagged units (a modifier on the
     movement, or a second movement class the unit switches to);
   - if the schema cannot express it, add the smallest schema piece needed (authorised), keep
     both lint views empty, and document it in `database.md`.

   No key branches.
4. **Pathing.** Paths may use shallow water only for units that can swim. The path cost uses
   the swim speed, so a long swim loses to a short walk round. A unit already in water when it
   loses the ability does not occur (researches are not lost); a save stores nothing new
   besides the research state.
5. **Look.** A swimming unit is drawn waist-deep: lower its model to the water surface, with no
   new model. A small ripple is optional.
6. **Computer players** research swimming when their map has shallow water between their land
   and targets, and send citizens across to gather on islands within swimming range.

## Crates

Seed (the two researches), schema only if needed, `rom-rules` (load the new rows), `rom-sim`
(movement and pathing), `rom-render` (waist-deep drawing), `rom-ai` (when to research and
use it).

## Read first

- `AGENTS.md`, `CLAUDE.md`;
- `docs/database.md` (movement classes, terrains, modifiers);
- `db/seed.sql` (`movement_class_terrain` rows);
- the pathfinder in `crates/rom-sim/src/`;
- `research/05-computer-opponents.md`.

## Done when

- Gate green from `app/`:
  - fmt, clippy `-D warnings`, workspace tests (≥ main plus this card's tests);
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views.
- Playthrough and ascension hashes unchanged, or explained.
- **Tests:**
  - a citizen cannot enter shallow water before the research and can after;
  - deep water stays closed;
  - infantry swims only after its research;
  - vehicles never swim;
  - the path prefers a short walk to a long swim;
  - each scenario run twice with equal hashes.
- **Screenshots** `docs/img/168-*.png`, each opened and checked:
  - citizens swimming across a strait;
  - infantry wading ashore.
- Report as AGENTS.md says. Logs only in the scratchpad's `t168/`; never write `gate*.sh`.
