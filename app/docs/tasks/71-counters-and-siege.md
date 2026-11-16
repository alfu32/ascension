# Task 71 — Every strength has an answer; armies that can break a city

Crates: `rom-ai`, `rom-cli` (a fight matrix and the duels), unit and
building stats and modifiers through `app/db/content/generate.py` (seed
regenerated; schema only if a value has no column, deliberately, documented
in `app/docs/database.md`), docs. **No change to `rom-sim`**: task 59 works
there in parallel. If a fix truly needs the simulation, stop and report it
with the measurement that shows why. No change to `rom-render`, `rom-app`,
`rom-mapgen` or any model file. Implementer: Claude (Opus).

## Why

Task 67 (0.0.67) made capture winnable and easiest gentle, but missed the
duel targets: moderate–easiest 85–3 with 12 draws (target 90 wins, at most
5 draws), mirror 37–36 with 27 draws (target at most 10). Its traces
(`STATUS.md` row 67, `app/docs/computer-player.md` task 67 section): equal
armies cannot break a defended city now that cities, towers and forts fire;
some computer players stall in their first age (workers without a path, the
opening); units get boxed in by their own buildings. The owner chose to merge
67 and follow up here (2026-11-16).

## The owner's rule (2026-11-16) — applies to every change in this task

> "Even if one unit outranges and outbeats the direct target type (like
> cannons vs buildings), its countermeasure should either be effective or be
> one upgrade away of being effective."

Read it as a property of the whole rules database, at every age:

- For every unit or armed building **A** that outranges and beats some
  target class **T** (siege against buildings, archers against infantry,
  artillery against defences, bombers against ground, and so on), there is a
  countermeasure **C** available to the side that owns T in the same age
  (cavalry, sallying units, fighters, anti-air, counter-battery, a building
  upgrade) that is **effective against A now**, or becomes effective with
  **one** research the player can take in that age.
- "Effective" is measured, not asserted: at equal resource cost, C wins a
  scripted fight against A (more than half the time across seeds, or more
  than half of A's cost destroyed before C is lost; state the measure you
  use and keep it fixed).
- Add a command, `rom fight-matrix` (deterministic, seeded), that runs these
  equal-cost fights for every pair the rule names, per age, and a test that
  fails when a strength has no answer. The table goes in
  `app/docs/computer-player.md`; fix the data until the test passes. Data,
  not names: the pairs come from component rows, tags and stats (armour
  classes, damage bonuses), never from definition keys.

## Build

1. **Breaking a defended city.** In every age, a prepared army must be able
   to take a defended city of the same age: siege that outranges the city's
   and towers' weapons (and, under the owner's rule, defenders that can sally
   out and beat that siege, or are one upgrade away from it). Tune stats and
   modifiers; tune `city.capture_hold_ticks` if measurement shows it matters.
2. **Computer players use it.** Waves include siege and an escort that
   protects it; defenders answer siege with the measured countermeasure; the
   main army raids workers before besieging (task 67 left this to the
   existing raiders).
3. **First-age stalls.** Trace each stalled seed in task 67's list (and any
   new ones) to its cause and fix it in `rom-ai` (worker paths, the opening,
   site choice).
4. **Not boxing in own units.** Computer players place buildings so units
   and workers keep a way out (lanes between buildings, the rally point kept
   clear) instead of tearing buildings down after the fact.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md, release
build, build-rules/check-rules); playthrough complete (explain any hash
change); `rom fight-matrix` table per age with the rule's test passing;
`rom duel --profiles balanced:moderate,balanced:easiest --seeds 1..100`:
moderate at least 90 wins, at most 5 draws, each draw explained; mirror
`balanced:moderate,balanced:moderate` at most 10 draws, wins about even;
`rom first-attack` table no gentler than 0.0.67's at the hard levels and no
harsher at easiest; duels deterministic (a subset run twice); run times.
Commit on the branch (stage by name), do not push or merge; final message
with commands and real output, the tables, files changed, data and schema
changes, deviations, anything not verified. If the targets are still missed
after a serious attempt, report the best result and the causes rather than
relaxing a rule.
