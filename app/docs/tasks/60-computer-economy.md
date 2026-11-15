# Task 60 — Computer opponent, part 1: economy, building, research, production

Crates: `rom-ai` (the computer player), `rom-cli` (a headless command),
docs. `rom-rules` only if a loaded AI row is missing a field. **No change to
`rom-sim`** (task 62 works there in parallel): if the computer player needs a
query or order the simulation does not offer, list it in the report and work
around it. Implementer: Claude (Fable).

## Why

Phase 8 of `app/docs/implementation-plan.md` was never built: `rom-ai` is an
empty crate and every match's other players stand still. The owner has played
the game for weeks against nobody. This card builds the half of the computer
player that runs a civilisation; task 61 adds armies, space and the app.

## Read first

- `research/05-computer-opponents.md` — the design to follow (same interface
  as a human, deterministic, honest difficulty, data-driven personality, no
  knowledge of unit names, bounded cost).
- `app/docs/implementation-plan.md` Phase 8 (8.1–8.4 and 8.6 are this card).
- `app/db/schema.sql`: `ai_profile`, `ai_weight_key`, `ai_profile_weight`,
  `ai_opening_step`, `ai_difficulty`, `player_ai_state`; the seed rows for
  them; `rom-rules` `AiProfile`, `AiProfileWeight`, `AiDifficulty`.
- `rom-sim`: `World::submit` and `Command`/orders (`order.rs`), what a player
  may see (`vision.rs`, `World::visible_entities`, `knows_terrain`),
  placement (`World::check_placement`, `placement.rs`), production,
  research, ages, gathering, cities and borders, the planet surface.
- `rom-cli/src/playthrough_cmd.rs` — a scripted single-player planner that
  already reaches the galactic age; reuse its knowledge of the API, not its
  script.

## Build

1. **Crate shape.** `rom_ai::Brain` per computer player: created from rules,
   player id, profile key and difficulty key; `think(&World, tick) ->
   Vec<Command>` called by the host every tick (it decides itself whether
   this is a think tick). Its own seeded random generator (seed from the match
   seed and the player id), its memory in plain structs that can be written
   to and read from bytes (`player_ai_state.data`, versioned) so a saved game
   continues identically. Integers only, iteration in id order, no clocks,
   no threads (hard rule 1).
2. **Perception (8.1).** Own state summary (resources, rates, workers by
   job, idle workers, buildings, queues, population and cap, age, cities and
   their territory); explored map and last-seen enemies through the fog
   (`ai_difficulty.sees_all` switches the fog off); a coarse threat map.
3. **Opening book and economy (8.2).** Follow the profile's
   `ai_opening_step` rows (blocking / skippable, "have N", abandoned when a
   city is attacked); then an economy manager keeps workers on resources in
   the proportions the profile's weights ask for, trains workers to the
   target, sends idle workers to work, builds gathering sites near deposits.
4. **Construction and research (8.3).** Houses before the population cap,
   production buildings, new cities when allowed (city limit, territory), age
   advances when affordable and weighted by the profile, technologies chosen
   from what they unlock (modifiers, units, buildings) — from data, never by
   key.
5. **Production (8.4).** Keep some military production running in proportion
   to the profile's weights, picking units from what the player can make and
   what counters the enemies it has seen (damage × armour from the rules).
   Units are only trained and gathered at a rally point this card; task 61
   moves armies.
6. **Difficulty (8.6).** Think interval, orders-per-minute budget, reaction
   delay, mistake rate (2nd-best choice) and the income bonus as an ordinary
   modifier — all from `ai_difficulty`. If a weight key the code needs is
   missing from `ai_weight_key`, add the row in the seed deliberately (lint
   views empty, generator check) and document it in `app/docs/database.md`.
7. **Headless match.** `rom ai-match --db <rules.db> --seed N --players
   balanced:moderate,builder:hard[,…] --ticks T [--universe]` sets up a match
   like the playthrough does, runs every player by computer, and prints per
   player: age reached, cities, buildings, units, workers, techs, resources
   gathered; then the state hash. Must run in seconds to a minute for a
   one-hour game on this machine (i7-6700HQ).

## Tests

Unit tests per manager on small built worlds. Determinism: the same
`ai-match` twice gives equal hashes (scenario test under
`crates/rom-ai/tests/`). A brain serialised and restored mid-match gives the
same orders afterwards. Bounded cost: think time per tick measured in a test
or the command's output for 4 players. Every profile in the seed reaches at
least the third age within one game hour on seed 1–5 on its own world, and
none issues an order the simulation rejects more than a few times in a row
(count rejections and print them).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md);
playthrough complete with its hash unchanged (`ac08633603f15f74` — this card
changes no simulation); an `ai-match` table for seeds 1–10 with two and four
players in the report. Commit on the branch (stage by name), do not push or
merge; final message with commands and real output, files changed, any seed
change, the simulation gaps found, deviations, anything not verified.
