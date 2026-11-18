# Task 92 — Unit series: ships and spaceships as classes; stable and siege workshop evolve

Crates: content data (`db/content/generate.py`, `seed.sql` as generated), `rom-ai` (choosing
among parallel classes), docs. `rom-sim` only if a rule is missing (stop and report).
Implementer: Claude (Opus). **Two phases: a proposal for the owner, then the build.**

## Why — the owner (2026-11-18)

"Infantry evolves from hoplites to riflemen, scouts evolve towards special forces and
equivalent, fighters and bombers (these are serialised correctly), light and heavy cavalry
evolve into light and heavy mechanised … They belong to a series that have the same function;
an upgrade will indeed replace the obsolete unit. Ships and spaceships are however not in a
series: I want to be able to keep building frigates, ironclads and other categories of ships; on
the space side I want to keep the same distinction. … The stable should evolve to an autoplant
and the siege factory should evolve into a factory."

Today (`tech_effect.upgrade_type` chains):
- Sea: one chain `trireme > war_galley > carrack > frigate > ironclad > destroyer >
  missile_cruiser > drone_cruiser > hover_frigate`, so each new ship retires all earlier ones.
- Space: one chain `corvette > space_frigate > space_cruiser > battleship`.
- Buildings: the stable trains the cavalry chains whose later members the vehicle plant trains;
  the siege workshop trains the artillery chain whose later members the factory trains; neither
  building has a successor.

## Phase 1 — proposal (stop after it)

Write `app/docs/unit-series.md`:
1. **Every series as it is today** (the chains above and all the others), with each member's
   age, producer and role.
2. **Sea classes.** Split the single sea chain into classes by role that coexist, each a short
   series of its own where a later ship truly does the same job better. The owner named
   frigates and ironclads as different categories: they must not replace each other. Suggested
   starting point (adjust with reasons): escorts (fast, cheap, anti-air later), line or capital
   ships (heavy guns, shore bombardment), and the missile and drone ships of the late ages.
   For each class: members, ages, cost, role, what it counters and what counters it (the
   counter rule: every unit that outranges and beats its target type has a countermeasure
   available now or one upgrade away; `rom fight-matrix`).
3. **Space classes.** Corvette, frigate, cruiser and battleship become size classes that
   coexist, each built for its role (stats, cost, counters), carriers and orbital craft beside
   them. Say whether any class should gain a later member.
4. **Orbital craft.** The owner asked whether orbital fighters can be used away from their
   planet. Today a craft climbs to its own planet's orbit and never leaves it on its own; a
   carrier can take up to 10 elsewhere. Describe exactly what works now (launching from a
   carrier at another planet, in system space, at the system's edge) and propose options.
5. **Buildings.** Stable → Vehicle Plant (the "autoplant"), Siege Workshop → Factory, as
   building upgrades (`upgrade_type` on a technology, like Farm → Automated Farm), at the age
   where the first unit of the successor appears; what happens to queues and rally points.
6. **The other series** checked for the same inconsistency (an upgrade that retires a unit of
   a different function), each with a verdict.

Report and stop. The orchestrator shows it to the owner; phase 2 starts on the owner's answer.

## Phase 2 — build what the owner approved

- The data changes in `generate.py` (generated seed, never hand-edited), the counter rule
  holding (`rom fight-matrix` unanswered=0), the computer players choosing among parallel
  classes by their counter logic (data-driven, rule 2), building upgrades working with queues.
- Model looks: a class whose members lose or gain ages may need a look for an age it did not
  have; list them for the model review loop, do not draw them here.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `research/01-reference-data-model.md`,
`research/05-computer-opponents.md`, `db/content/generate.py` (units, upgrades), `rom-ai`
(production choice), `rom-cli/src/fight_matrix_cmd.rs`.

## Done when (phase 2)

- Gate green from `app/` (fmt, clippy, all tests, generator check, lint, release build).
- Fight matrix unanswered=0; playthrough reaches the last age (its hash will change: report the
  new one and why); both 100-seed duels run and reported against 0.0.83's (96–0 with 4 draws;
  53–42 with 5 draws); a change is explained.
- Tests: after the frigate's age an ironclad's research does not retire frigates; space classes
  coexist; Stable → Vehicle Plant and Siege Workshop → Factory upgrade existing buildings with
  their queues; repeat → same hash.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the series table as built, deviations, anything not verified.
