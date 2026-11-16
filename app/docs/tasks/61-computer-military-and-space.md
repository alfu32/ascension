# Task 61 — Computer opponent, part 2: armies, space, duels

Crates: `rom-ai`, `rom-cli`, docs. **No change to `rom-sim`, `rom-app`,
`rom-render`** (task 58 works there in parallel); simulation gaps go to task
64, list any new ones. Starts after task 60. Implementer: Claude (Opus).

## Why

Task 60 built a computer player that runs an economy
(`app/docs/computer-player.md`, `rom_ai::Brain`, `rom ai-match`). It never
fights and never leaves its planet. This card finishes Phase 8 of
`app/docs/implementation-plan.md` (8.5 and 8.7) headless; task 64 then puts
the brains into the owner's matches.

## Build

1. **Military manager (8.5).** Armies as groups with roles (defend, attack,
   raid, escort); gather at the rally point, attack when the profile's attack
   ratio is met against what it has seen, retreat when losing (if the
   difficulty `retreats`), defend cities under attack, bring siege for walls
   and cities, formations if the difficulty `uses_formations`. Scouting
   (task 60 skips the `scout` opening steps). Targets chosen from data
   (value, threat, counters), never by key. Naval units where the map needs
   them.
2. **Space.** From the starfaring ages: survey ships claim resource bodies,
   warships escort and contest claims, transports (task 54) carry an army and
   found colonies on other planets, ships attack enemy orbits. Orbital craft,
   carriers and ground batteries if task 58 has merged by then (it adds them);
   otherwise say so and leave the hook.
3. **Duel harness (8.7).** `rom duel --db <rules.db> --profiles a:diff,b:diff
   --seeds 1..100 [--ticks-limit N] [--universe]` → a win-rate table with
   victory rules on (task 26); every match ends with a winner inside the tick
   limit; hashes repeat. Target: "moderate" beats "easiest" in at least 90 of
   100. Keep it fast enough to run 100 seeds in a reasonable time on this
   machine (state the time); a smaller map style is fine.
4. **Brain state.** Whatever new memory the military and space managers keep
   goes into the brain's versioned bytes (`Brain::save` / `restore`).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
hash unchanged (`rom-sim` untouched); the duel table for 100 seeds with its
run time; duels deterministic (same seed twice, equal hashes); a test that a
brain saved and restored mid-war issues the same orders. Commit on the
branch (stage by name), do not push or merge; final message with commands
and real output, files changed, the simulation gaps found, deviations,
anything not verified.
