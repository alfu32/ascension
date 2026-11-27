# Task 170 — Buildings lift off: across the planet, through the system, to other stars

GitHub issue #173.

Owner (chat, 2026-11-25), verbatim: "i want buildings to have an 'upgrade to ship' that enbles
them to take off and fly: on the same planet on age 9, in the solar system on age 11 and
through hyperspace in age 13."

Ages counted from 0, as the owner set them on 2026-11-26:
- **across the planet** from age 10, the Spacefaring age (`age.starfaring`). The owner
  corrected this from 9.
- **within the star system** from age 11, the Starfaring age (`age.expansion`).
- **to other stars** from age 13, the Age of Galactic War (`age.galactic`). Jumps between
  clusters still follow task 163's rule (Age of Abundance, rank 3), so at 13 this is already
  open.

## What changes

1. **The upgrade.** Each eligible building gets an upgrade, **Lift-off engines**, researched at
   the building itself. Express it as a modifier or component row, not by key.
   - **Eligible buildings:** production and military buildings (barracks, factory, airfield,
     spaceport, research buildings, and others like them).
   - **Excluded:** cities, wonders, walls, gates, shield emitters, mines, minefields and
     resource buildings that stand on a deposit. Pick the set by tags.
   - **Cost:** a substantial price, about a third of the building's own price.
2. **Lift off** (command on an upgraded building). The building stops producing (its queue is
   paused) and rises into the air layer as a slow flying unit:
   - **Its numbers:** its hit points and armour stay. It moves at about 1 tile per second on the
     planet.
   - **Landing:** it lands at a free spot that fits its footprint (the placement check of
     construction) and resumes its queue.
   - **While flying:** it cannot be garrisoned, cannot produce, and can be shot by anti-air.
3. **Within the system** (age 11). An upgraded building may climb to orbit and travel to
   another planet of the same system, like a slow transport, and land there. Its owner must
   have a city on that planet, or the planet is unclaimed.
4. **To other stars** (age 13). It may jump along routes like a starship, under the same jump
   rules (charge, inhibitors, cluster tier). It is slow and cannot jump while damaged below
   half its hit points.
5. **Saved state.** A flying building keeps its id, upgrades and queue. The save format bumps.
6. **Look.** The building's own model with a small set of engine parts added below it, plus
   exhaust while flying. Add the engine parts to the part kit; locked models are unchanged
   when on the ground.
7. **Computer players.** They lift a production building only when its planet is lost or under
   siege, and move it to a safe own planet. They never fly buildings into combat.

## Recent work to fit with (orchestrator, 2026-11-27)

- **Buildings in forests (task 172):** `crates/rom-rules/src/cover.rs`; landing placement must
  apply the same rule. `crates/rom-sim/src/footing.rs` moves units that cannot stand where they
  are; a building landing must never trap units under its footprint (move them with the same
  search).
- **Turned buildings (task 165):** a building keeps its facing when it lands, or the landing
  ghost turns with R like any placement.
- **Cluster jumps (task 163):** jumps between clusters need the galactic tier.
- **Jetpacks and orbital drop (task 169):** `entity_leap` shows how in-flight state is saved;
  follow the same pattern.
- **Sea transports (task 167):** a building's garrison or queued units travel inside it; say what
  happens to them.

## Crates

Seed (the upgrade, the tags), schema if a building-in-flight needs a column (stop and report
first), `rom-rules`, `rom-sim` (lift off, flight, land, orbit and jump for buildings: reuse
the unit flight, orbit and jump code), `rom-save`, `rom-render` (engine parts and exhaust),
`rom-app` (commands, landing ghost), `rom-ai`.

This is a large card. If it does not fit one pass, split it into 170a (across the planet) and
170b (system and stars). Report the split rather than shipping half of a stage.

## Read first

- `AGENTS.md`, `CLAUDE.md`;
- `docs/world-model.md` (orbit, routes, jumps, task 163's cluster tier);
- `docs/database.md`;
- `docs/hud.md` (the Lift off button the space transport already has);
- `crates/rom-sim/src/` (flight, orbital transport, jumps, construction placement);
- `docs/models-in-meshcraft.md`.

## Done when

- Gate green from `app/`:
  - fmt, clippy `-D warnings`, workspace tests (≥ main plus this card's tests);
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views.
- Playthrough and ascension hashes unchanged, or explained.
- **Tests:**
  - lift off refused without the upgrade or in a too-early age;
  - landing refused where the footprint does not fit;
  - the queue pauses and resumes;
  - orbit travel to another planet;
  - a jump that respects the cluster tier;
  - save and load mid-flight;
  - each scenario run twice with equal hashes.
- **Screenshots** `docs/img/170-*.png`, each opened and checked:
  - a barracks lifting off;
  - a factory flying over terrain;
  - a building in orbit;
  - the landing ghost.
- Report as AGENTS.md says. Logs only in the scratchpad's `t170/`; never write `gate*.sh`.
