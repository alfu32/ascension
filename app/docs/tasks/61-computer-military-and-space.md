# Task 61 — Computer opponent, part 2: armies, space, and opponents in the game

Crates: `rom-ai`, `rom-cli`, `rom-app` (computer players in a real match),
docs; `rom-sim` only for small queries task 60 listed as missing. Starts
after task 60 (same crate) and task 57 (both touch `rom-app`). Implementer:
Claude (Opus).

## Why

Task 60 builds a computer player that runs an economy. It still never fights,
never leaves its planet, and the app does not run it. This card finishes
Phase 8 (`app/docs/implementation-plan.md` 8.5 and 8.7) and puts computer
opponents into the owner's matches.

## Build

1. **Military manager (8.5).** Armies as groups with roles (defend, attack,
   raid, escort); gather, attack when the profile's attack ratio is met
   against what it has seen, retreat when losing (if the difficulty
   `retreats`), defend cities under attack, siege for walls and cities,
   formations if the difficulty `uses_formations`. Targets chosen from
   data (value, threat, counters), never by key.
2. **Space.** From the starfaring ages: survey ships claim resource bodies,
   warships escort and contest claims, transports (task 54) carry an army
   and found colonies on other planets, ships attack enemy orbits. Orbital
   craft and carriers if task 58 has merged by then; otherwise say so.
3. **Duel harness (8.7).** `rom duel --db <rules.db> --profiles a:diff,b:diff
   --seeds 1..100 [--ticks-limit N]` → win-rate table; every match ends with a
   winner (victory rules from task 26) inside the tick limit; hashes repeat.
   Target: "moderate" beats "easiest" in at least 90 of 100.
4. **In the app.** Every non-human player slot is run by a brain (profile
   and difficulty from the launcher; add the choice per slot to the launcher
   if it is missing). The brains run on the simulation thread and submit
   orders like the player does. Their state goes into saves when task 62's
   saving exists (`player_ai_state`).

## Done when

Gate green; playthrough complete (hash unchanged unless the simulation
changed; explain); the duel table for 100 seeds; determinism of duels; a
screenshot strip you looked at of a computer army attacking the player's city
in a real match, and one of a computer colony on another planet. Commit on
the branch (stage by name), do not push or merge; final message with commands
and real output, files changed, deviations, anything not verified.
