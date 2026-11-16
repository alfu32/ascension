# Task 67 — Capture that can be won, and armies that win it

Crates: `rom-sim` (capture rule), `rom-ai` (spending and attack waves),
`rom-cli` (duel), schema and seed (the rule values and any new
`ai_difficulty` columns, documented in `app/docs/database.md`), docs.
No change to `rom-app`, `rom-render`, `rom-kit` (model cards run there).
Starts after task 65 is merged. Implementer: Claude (Opus).

## Why

Tasks 61 and 65 both missed the duel target (65: moderate 13 – easiest 7,
80 draws of 100; mirror 7 – 7, 86 draws). Task 65's diagnosis (task 65 row in
`STATUS.md`, `app/docs/computer-player.md`): a city is captured only at 0 hp
with no armed enemy unit in its radius, and citizens are armed, so a capital
holds 50–110 armed defenders, and workers walking back keep a struck city
contested. Even economies never field an army that can clear that, so
matches end in stand-offs. The owner chose (2026-11-16) to change the rule
and to make computer players spend their surplus.

## Build

1. **Workers do not hold a city.** Units whose role is work (they gather or
   build; take the test from component rows, never keys) no longer count as
   armed defenders for capture, and no longer stop a struck city from
   falling. They still fight back. Same rule for every player, in data where
   a value is needed.
2. **A held-down city falls.** A city kept at 0 hp with attackers in its
   radius for a hold time (a rules value, for example 2 game minutes; from
   data) is captured even if armed defenders remain. Show the countdown to
   the owner of the city through what the simulation exposes (an event or
   query the HUD can read later; no `rom-app` change in this task).
3. **Computer players spend surplus.** Wealth and knowledge that pile up go
   to upgrades, unit technologies and age advances; when resources pile up,
   the army target may grow past `mil.army_share_pct` up to the population
   cap; the main army holds a decisive wave until it clearly exceeds the
   defence it measured, and raids workers before besieging.
4. **Difficulty that decides.** If the existing `ai_difficulty` columns
   cannot open a large enough gap, add columns (for example an economy or
   skill multiplier) deliberately: lint views empty, seed loads, documented.

5. **The easiest level must be beatable by a person learning the game.**
   The owner (2026-11-16, version 0.0.59, universe match) was beaten
   thoroughly by `easiest` and could not build what they wanted because its
   army kept destroying their buildings. From `ai_difficulty` data (new
   columns if needed, never names): a grace period with no attack on a
   person's buildings for the first minutes (longest at `easiest`), small
   first waves that grow slowly, slow reactions, and no raids on a person's
   town early. Easier levels may keep their military small relative to the
   person's; harder levels may not. Measure it: a scripted or idle
   opponent standing in for a person (for example `--ai none` for player
   one in `rom ai-match`/`duel` with a fixed defence) shows when each level
   first attacks and with how many units; report the table.

Every simulation change gets run-twice-equal-hash tests and goes into the
snapshot and `rom-save` (bump the save format if needed). Explain any change
to the playthrough hash.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
complete; `rom duel --profiles balanced:moderate,balanced:easiest --seeds
1..100`: moderate wins at least 90, at most 5 draws, each explained; the
mirror duel `balanced:moderate,balanced:moderate` has at most 10 draws and
each side wins about half; the first-attack table shows `easiest` waiting
longest with the smallest waves; run times stated; duels deterministic. Commit on
the branch (stage by name), do not push or merge; final message with
commands and real output, the tables, files changed, schema and save-format
changes, deviations, anything not verified.
