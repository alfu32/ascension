# Task 64 — Computer players in the game, and the simulation gaps they found

Crates: `rom-sim` (orders and queries), `rom-rules` (validator), `rom-app`
(computer players in a match, launcher), `rom-save` (brain state), schema and
seed if needed, docs. Starts after task 58 (it changes `rom-sim` and
`rom-app`); `rom-ai` only for the small adaptations the new orders allow
(coordinate with task 61 if it is still running: then leave `rom-ai` alone
and list what it should use). Implementer: Claude (Opus).

## Why

Tasks 60 and 61 build `rom_ai::Brain` headless. The owner's matches still
have players who never move. Task 60 also found gaps in the simulation
(`app/docs/computer-player.md`, the task 60 report in `STATUS.md`).

## Build

1. **Simulation gaps (task 60).**
   - The difficulty's `income_bonus_pct` (and any per-player handicap) needs
     a modifier source per player: a deliberate, data-driven way for a
     player to hold modifiers that come from neither a technology, an age nor
     a faction (for example a granted-at-setup source the validator accepts,
     or a per-player modifier list in the match setup). Decide, document in
     `app/docs/database.md`, test.
   - Implement the `disband` order (in `order_kind`, not executed) and the
     `set_rally` order (rally points on producers; produced units walk there).
   - Make the queries a computer player needs public and cheap:
     `city_limit`, `city_count`, `city_covering`, `is_city_type`, an orders
     view; and make `World::price` and `World::check_placement` stop scanning
     every entity (task 60 measured ~3 ms each in a universe world).
   - Keep hashes: if a change alters existing behaviour, explain any pinned
     hash change.
2. **In the app.** Every non-human player slot is run by a brain: profile and
   difficulty per slot from the launcher (add the choice per slot if it is
   missing; default `balanced:moderate`). The brains run on the simulation
   thread, think on their own ticks and submit orders like the player does;
   their think time shows in the existing performance overlay or log.
3. **Saves.** Brain bytes go into `player_ai_state` through
   `rom_save::save(…, ai_states)` and come back on load (task 62's hook).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
complete (explain any hash change); determinism scenarios for `disband` and
`set_rally`; a save with brains loads and continues with equal hashes.
Screenshots you looked at: a match where the computer player's town has grown
after a few game minutes (`--after` long enough), and its army at its rally
point. Bench the sphere demo with four computer players (with `uptime`).
Commit on the branch (stage by name), do not push or merge; final message
with commands and real output, files changed, schema changes, deviations,
anything not verified.
