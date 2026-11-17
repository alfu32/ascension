# Task 75 — Orbital bombardment that flattens a city, and batteries that hold ships off

Crates: `rom-sim` (orbital fire, area damage where it is missing), weapons,
stats and modifiers through `app/db/content/generate.py` (seed regenerated;
schema only deliberately, documented in `app/docs/database.md`),
`rom-render` (bombardment effects), `rom-ai` (using and answering
bombardment), `rom-cli` (`rom fight-matrix` extended to orbit), docs. No
model file changes (all models are locked). Starts after tasks 71, 74 and 70
are merged (same crates). Implementer: Claude (Opus).

## Why — the owner (2026-11-17)

"Orbital bombardment is unspectacular and weak. A few rounds from a space
frigate, cruiser or destroyer should completely flatten a city and its
surroundings, including units (though some could survive). The ships should
be much more powerful, maybe with bombs that have an area effect. The
defender should have ground batteries that keep the enemy ships at bay."

## Read first

`AGENTS.md`; `app/docs/world-model.md` ("Launch, orbit and ground fire",
"Orbital craft, carriers and ground batteries", "Ships in orbit");
`app/docs/database.md` (weapons: `splash_radius`, `splash_pct`; ship and
battery stats; "Counters and siege" from task 71); the owner's counter rule
in `app/docs/tasks/71-counters-and-siege.md`; `rom-sim` orbital fire
(`space.rs`, `orbital.rs`, combat), `bld.ground_battery` and the starship
rows in `generate.py`.

## Build

1. **Bombardment that devastates.** Starships from the frigate up (space
   frigate, space cruiser, battleship; the carrier through its bombers)
   bombard the ground with area weapons (`splash_radius`, `splash_pct`, from
   data). Target: **three to five volleys from one frigate or cruiser level
   a city of its age and the buildings around it**, and kill most units in
   the blast (units at the edge of the area, or with high armour, can
   survive). The battleship does it faster and wider. Corvettes and the
   survey ship stay light. If orbital fire does not yet apply splash, add it
   to the simulation (integers, id order, tested twice with equal hashes).
   State the numbers: damage, radius, reload, and the measured volleys to
   level a city in each space age.
2. **Ground batteries keep ships at bay.** `bld.ground_battery` (and its
   later ages, if any) outranges and outdamages a bombarding ship of its age,
   so a planet defended by batteries forces ships to hold off or lose. Under
   the owner's counter rule, the answer to a battery (bombers, a heavier
   class, an upgrade) must exist in the same age or one research away; and
   the answer to bombardment is the battery (effective now, or one research
   away). Extend `rom fight-matrix` to orbit fights (ship vs battery, ship vs
   city) and keep `every_strength_has_an_answer` passing.
3. **Spectacle.** Each volley is seen: a beam or shell trail from orbit, a
   bright impact flash, a shockwave ring on the ground scaled to the blast,
   fire and smoke on the ruins, scorched ground that fades over a minute,
   debris; buildings collapse instead of vanishing. Visible from the planet
   view at the new 7–10× ship size of task 72 if merged, and as flashes on
   the globe from the system view. Bench a heavy bombardment (before and
   after, with `uptime` load).
4. **Computer players** bombard cities they cannot take on the ground,
   escort bombarding ships, build batteries on threatened planets and target
   enemy batteries first.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules); playthrough complete (explain any hash change);
the volleys-to-level table per space age and ship class; the orbit rows of
`rom fight-matrix` with the rule test passing; duels not worse than the
latest release; screenshots you looked at: a city before, during (impact,
shockwave) and after a bombardment, survivors at the edge, a battery driving
ships off; commit on the branch (stage by name), do not push or merge; final
message with commands and real output, files changed, data and schema
changes, deviations, anything not verified.
