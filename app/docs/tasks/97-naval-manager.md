# Task 97 — Computer players at sea: a naval manager

Crates: `rom-ai` only. Implementer: Claude (Opus). Starts after task 92 phase 2 merges.

## Why — the owner (2026-11-18)

Task 92's survey found that computer players never train warships. With the sea split into
classes (frigates, destroyers, line ships, strike ships; `app/docs/unit-series.md` sections 3
and 10), the owner chose a naval manager as its own card.

## Build

1. **When to go to sea**: a player whose home planet has water reachable from its shore and an
   enemy (or the enemy's buildings or ships) reachable by water builds a harbour building and
   a small navy, scaled by the profile's military share (data: `ai_param` rows, rule 2).
2. **Which ships**: choose among the classes by counter value against the ships and shore
   targets it has seen (the exchange measure `production::value` already uses), never by key.
3. **What they do**: guard own coast and transports, hunt enemy ships, bombard shore targets
   with line ships when the enemy fleet is beaten; retreat damaged ships to repair.
4. Easiest profiles may stay ashore (a data knob).

Read first: `AGENTS.md`, `CLAUDE.md`, `research/05-computer-opponents.md`,
`app/docs/unit-series.md`, `rom-ai` (`production.rs`, `fleet.rs`, `military*.rs`).

## Done when

- Gate green from `app/`; repeat → same hash; playthrough hash change explained.
- Tests: a sea map where both sides train ships; the class chosen answers the enemy's ships.
- Both 100-seed duels reported against the release before; a 10-seed duel on a map with
  water reported (ships trained, ships sunk, coast bombarded).

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, numbers, deviations, anything not verified.
