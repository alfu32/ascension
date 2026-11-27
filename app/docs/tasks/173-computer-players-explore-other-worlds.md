# Task 173 — Computer players explore other worlds, invade and drop

Found by task 169 (orchestrator, 2026-11-27): in every universe `ai-match`, no computer player
ever learns of an enemy building on another planet. Their space manager does not scout enemy
worlds, so they never invade, never use the orbital drop, and never take the war off their home
planet. Task 167 found a similar gap on one planet: the sea search over 16 fixed points is
coarse.

GitHub issue #177.

## What changes

1. **Scouting other worlds.** From the age a player can see its own system (Fusion, age 9),
   the computer player keeps one cheap space unit scouting.
   - **Which unit:** a probe, survey ship or satellite, whatever the rules offer at that age,
     found through components and tags, never keys.
   - **Order of visits:** planets of its own system, nearest unseen first; then, with the jump
     tier, systems along routes in the order the route plan gives.
   - **What it remembers:** enemy cities, buildings, orbits held and defences seen, in its
     memory as it already remembers ground sightings. Saved, as the memory is now.
   - **Replacing scouts:** a lost scout is replaced with a pause, scaled by the pace, as task 166
     does for ground scouts.
2. **Choosing a target world.** Once an enemy world is known, the military manager weighs it
   like a ground target. It reaches that world by:
   - **ground invasion:** the space transport lands troops (task 72);
   - **orbital drop:** task 169, when researched;
   - **taking orbit first** with warships when the enemy holds it.
3. **Sea search on one planet (task 167):** replace the 16 fixed points with a coverage search:
   unseen coast in order of distance, remembered as seen.
4. **Strength check.** No invasion is sent without enough force: compare with what was seen,
   the way ground attacks do now.

## Crates

`rom-ai` (scouting, memory, the invasion plan), and `rom-cli` only for ai-match output lines.
No simulation change is expected; if one is needed, stop and report.

## Done when

- **Gate green from `app/`:**
  - fmt, clippy `-D warnings`, workspace tests (≥ main plus this card's tests);
  - fight matrix `unanswered=0`;
  - duel 20–0 (the duel is a planet match and must not change much: explain any change);
  - playthrough and ascension hashes unchanged, or explained.
- **ai-match on the Solar and Cluster styles** (separate home planets, start age 11, two
  balanced players, 40 000+ ticks), reported:
  - the first enemy world seen;
  - the first invasion;
  - the first drop;
  - the winner.

  Run twice with equal hashes.
- **On an island planet seed** (task 167 found seeds 2 and 8 of the Planet style), both players
  find each other.
- **Tests:**
  - a scout visits every planet of the system;
  - an enemy city seen is remembered across a save;
  - an invasion is planned only with enough force.
- Report as AGENTS.md says. Logs only in the scratchpad's `t173/`; never write `gate*.sh`.
