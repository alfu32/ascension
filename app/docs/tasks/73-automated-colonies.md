# Task 73 — Automated colonies, the automated research lab, the survey ship kept

Crates: `rom-sim` (colony placement and limits, wealth), `rom-ai` (the
colony governor), `rom-rules` if a column is added, schema and seed through
`app/db/content/generate.py` (documented in `app/docs/database.md`),
`rom-save`, `rom-app` (colony panel, placement), first-look model files for
the new buildings, review-page round, docs. Starts after tasks 59 and 71 are
merged (they hold `rom-sim` and `rom-ai`). Implementer: Claude (Opus).

## Why — the owner (2026-11-17)

- "In the abundance age we should get an upgrade of the city to an automated
  colony facility that uses game AI to gather resources from a planet. It can
  maintain up to 5 automated farms, 5 woodcutter camps and 5 automated labs.
  Automated colonies do not count towards the city limit, can be placed
  anywhere and will add to wealth."
- "I would eliminate automated universities, but replace them with automated
  research labs that gather some amount of knowledge and can be placed in the
  area of an automated colony."
- "The scout ship should not be superseded by the next upgrade; we should
  still be able to build it."

## Build

1. **Automated colony** (`bld.auto_colony`, abundance age). Unlocked by a
   city research in the abundance age ("Automated colonies"). Built by
   workers or worker drones anywhere on a planet's land that is not inside
   another player's borders, on any planet where the player has units.
   - It is a city for borders and supply, but it does **not** count towards
     the city limit (a component or flag in data, not a key test) and it has
     no citizens to train.
   - It adds wealth: a steady income from data, plus a share of what its
     sites gather (state the values).
   - **Its governor** maintains up to 5 automated farms, 5 automated
     woodcutter camps and 5 automated research labs inside its area: it
     places them on the best free sites, rebuilds them when destroyed, and
     pays their cost from its owner's stores while it can. The governor is
     part of `rom-ai` and runs for every owner, people included; it changes
     the game only through ordinary orders (rule 3), issued in the colony's
     owner's name, never through a back door. The caps (5/5/5) are data on
     the colony type.
   - Its panel shows each kind's count against its cap, a switch to pause
     the governor, and what it is waiting for ("Needs 200 metal").
2. **The automated university becomes the automated research lab.** The
   owner revised this (2026-11-17): "don't remove the automated
   university, just change its name (automated lab) and maybe model".
   - Keep `bld.auto_university` (its key, its rules, its upgrade from the
     university and its saved state); rename it to **Automated Research Lab**
     everywhere a player reads it (names, help, tooltips, docs).
   - It can also be built directly, without a university first, inside the
     area of an automated colony (the colony's governor builds up to five);
     elsewhere it still comes from upgrading a university. It gathers
     knowledge by itself at its present income unless measurement says
     otherwise.
   - **Model:** the approved model stays and stays locked. Offer one
     alternative first look that reads as a lab (a separate model id in a
     review round, not wired as the default); the owner decides in review
     whether it replaces the current one.
3. **The survey ship stays buildable.** Today it is the first of the
   starship line (`survey_ship → corvette → frigate → cruiser → battleship`),
   so the corvette replaces it. Give it a line of its own so it can always
   be trained at the spaceport; its sight and role are unchanged.
4. **Computer players** build automated colonies on their planets when they
   pay, and keep their governors on.
5. **Models.** A first look for `bld.auto_colony@abundance` and the
   alternative lab look (item 2), following `app/docs/tasks/model-card-common.md`
   (a later review round; do not touch any other locked model).
6. **Balance.** The owner's counter rule (task 71, `rom fight-matrix`) must
   still pass; duels after the change must not be worse than 0.0.67 or the
   task 71 result, whichever is newer.

Every simulation change gets run-twice-equal-hash tests and goes into the
snapshot and `rom-save`.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules); playthrough complete (explain any hash change);
lock test green (no locked model changed); screenshots you looked
at: an automated colony on a second planet with its 15 sites built by its
governor, the colony panel, an automated research lab beside a city and one built by a colony's governor, the survey ship
in the spaceport's training grid next to the corvette; duel tables. Commit on
the branch (stage by name), do not push or merge; final message with
commands and real output, files changed, data, schema and save-format
changes, deviations, anything not verified.
