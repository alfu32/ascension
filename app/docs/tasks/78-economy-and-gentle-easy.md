# Task 78 — Computer economies that do not stall; a gentle easy level

Crates: `rom-ai`, `rom-cli` (duel traces), difficulty and economy values
through `app/db/content/generate.py` (the `ai_difficulty` rows and any
economy constant; schema only deliberately, documented in
`app/docs/database.md`), docs. **No change to `rom-sim`** (task 70 runs there
in parallel); stop and report if one is needed. Implementer: Claude (Opus).
Starts after task 71 is merged (0.0.69).

## Why — the owner (2026-11-17)

Task 71 was merged with the duel targets narrowly missed: moderate–easiest
93–1 with 6 draws (target at most 5), mirror 48–39 with 13 draws (target at
most 10). Its traces put the remaining draws on the economy, not combat
(`STATUS.md` row 71, `app/docs/computer-player.md` task 71 section):

- seeds 21 and 90: a single small woodcutter's camp all match (timber +40 a
  period) while metal passed 10 000;
- seeds 15 and 38: no mine placed, so no metal;
- seed 23: no path into the easiest player's town (46 failed attack-moves);
- seed 14: waves lost at the walls, an age behind;
- mirror: both sides stall around the fourth age, armies capped by food and
  timber while metal and knowledge pile up.

The owner also wants **easy kept gentle**: task 71 moved easy's first wave
from about 44 minutes to about 19. Restore it.

## Build

1. **Balanced gathering.** Computer players keep each resource's income in
   proportion to what they are spending it on: they add woodcutter's camps,
   farms and mines (and upgrade them) when a resource caps production, move
   workers from piled-up resources to starved ones, and trade surplus at the
   market when the rules allow. Measure by trace: no seed in the duels ends
   with a single gather site of a resource the player spends.
2. **Always a mine and a camp.** Each city founds the gather sites its
   terrain allows within the first minutes of its life; if the home area has
   no rock or forest in reach, the next city goes where they are.
3. **Reaching a walled town.** When attack-moves fail with `no_path`, the
   army breaks through: siege or melee target the wall, gate or building that
   blocks the shortest way in (from order failures, not keys).
4. **Easy is gentle again.** From `ai_difficulty` data, easy's first wave
   comes no earlier than in 0.0.67 (earliest about 31 minutes, mean about
   45) with small waves; easiest stays as in 0.0.69 or gentler. The
   `rom first-attack` table must show easiest ≥ easy ≥ the hard levels in
   time to first wave, and the hard levels no gentler than 0.0.69.
5. **Targets** (unchanged from task 67): `rom duel` moderate–easiest 1..100:
   moderate at least 90 wins, at most 5 draws, each draw explained; mirror
   at most 10 draws, wins about even; `rom fight-matrix` rule test still
   passing; duels deterministic (a subset twice); run times stated. If missed
   after a serious attempt, report the best result and causes rather than
   relaxing a rule.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules); playthrough complete (explain any hash change);
the tables above (duels before at 0.0.69 and after, first-attack, fight
matrix); commit on the branch (stage by name), do not push or merge; final
message with commands and real output, files changed, data changes,
deviations, anything not verified.
