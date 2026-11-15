# Task 62 — Save and load a match

Crates: `rom-save` (writing and reading a save file), `rom-sim` (exporting
and restoring the complete simulation state), schema PART B (the state
tables) and docs. **No change to `rom-ai`, `rom-cli` or `rom-app`** (tasks 60
and 57 work there in parallel); the save and load screens follow in a later
card. Implementer: Claude (Sonnet).

## Why

`rom-save` is an empty crate: a match cannot be saved. Plan step 11.4
(`app/docs/implementation-plan.md`). The schema's PART B (`app/db/schema.sql`
from `-- PART B — STATE`) was designed early, before round planets, star
systems, bodies, routes, ships in transit, orbits, carried units, landing and
colonies, wealth, territory growth and victory timers; parts of it no longer
match the simulation.

## Build

1. **Complete state out and in.** In `rom-sim`, a way to export everything
   the simulation needs to continue (players and their resources, techs and
   knowledge; every entity with its components' live values, orders, queues,
   cooldowns, effects, carried units, landing and orbit state; surfaces and
   their terrain changes; the universe: systems, bodies, routes, claims;
   territory, cities, wealth, market, diplomacy, victory timers; the random
   generator; the tick and next ids) and to rebuild a `World` from it plus the
   rules. Derived caches (spatial index, paths, visibility, territory maps)
   are rebuilt, not stored, unless rebuilding them would change the result.
   Keep it in a new module (e.g. `rom-sim/src/snapshot.rs`) with plain
   public structs; no SQL in `rom-sim`.
2. **Save file.** One SQLite file per save in the PART B tables. Update
   PART B deliberately to match the simulation of today (add, change or drop
   tables and columns; keep `player_ai_state` with an opaque versioned
   `data` blob that the caller passes in per player, for task 61). A save
   records the rules version (and a fingerprint of the rules database) and
   the match settings so loading can rebuild the same rules and map; a save
   from different rules is refused with a clear error. Schema changes keep
   both lint views empty and the seed loading; document PART B in
   `app/docs/database.md`.
3. **API.** `rom_save::save(path, &World, &MatchInfo, ai_blobs)` and
   `rom_save::load(path, &Rules) -> (World, MatchInfo, ai_blobs)` (names
   yours), typed errors (`thiserror`).

## Tests

The defining test: for each scenario, run to tick N, save, load, then run
both the original and the loaded world M more ticks with the same orders —
the state hashes must be equal after the load and after the M ticks. Scenarios
(under `crates/rom-save/tests/`): the flat battle; a sphere economy with
gathering, construction and research in progress; borders and wealth; a
universe match with ships in transit on a route; a transport carrying units,
one descending and one landed; a colony on another planet; a player near a
victory timer; a late-game state in the galactic age (built in the test the
way `rom playthrough` does, without depending on `rom-cli`). A save of a mismatched rules version is refused. File size and save /
load time for the late-game playthrough state in the report.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md);
playthrough complete with its hash unchanged (`ac08633603f15f74`; if a
simulation change was unavoidable, explain it). Commit on the branch (stage
by name), do not push or merge; final message with commands and real output,
files changed, the schema changes, deviations, anything not verified.
