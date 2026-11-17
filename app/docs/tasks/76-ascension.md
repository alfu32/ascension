# Task 76 — Ascension: leave the material plane, live at the galaxy's heart, descend again

Crates: `rom-sim` (ascension project, ascended state, descent, victory
rules), schema and seed through `app/db/content/generate.py` (rule values,
the project, documented in `app/docs/database.md`), `rom-save` (bump the
format), `rom-ai` (when to ascend and descend), `rom-mapgen` (the celestial
home), `rom-render` and `rom-app` (the ascended view, the celestial home,
the descent pick, the ascension countdown), docs (`app/docs/world-model.md`,
a new `app/docs/ascension.md`). Starts after tasks 75 and 73 are merged.
Implementer: Claude (Opus).

## Why — the owner (2026-11-17)

"The player can choose to ascend when they have researched all that is at the
library. Ascended players lose all their possessions. In exchange they can
explore the universe freely. They have their celestial home at the centre of
the galaxy, near the black hole there; they live eternally on a mythical
planet where they can interact with other ascended players. They can bring
life to inert planets [task 77]. They can descend (recorporealise) in the
material plane at any point on any habitable planet, any time. When they
descend they get a fair amount of resources and a single city, and restart
at the first age. Players that face defeat can choose to ascend if the
conditions are met. Ascension takes 2–3 times the usual time of an age
research; descending takes no time at all."

The owner's answers the same day:
- The match is **sandbox-ish**: it has no end condition, but a player must
  **complete** the ascension before being defeated. A player defeated while
  the ascension is still running is defeated; the long project brings drama.
- Resources on descent: **slightly lower than what was spent to reach
  ascension**: the sum of all library research costs, plus a fair amount of
  the resources the library does not use. ("Even 20 000 is enough to get very
  far.")
- No 3D-model making for ascended players (dropped).

This replaces the decision "the game ends at age 13" (CLAUDE.md, updated).

## Build

1. **The ascension project.** Once a player has researched every technology
   the library offers (a data test over the library's production rows, not
   a key list), the library offers **Ascension**: a project whose duration
   is a multiple of the player's last age-advance research time (data,
   2.5×), with a cost from data. Everyone is told when a player starts it
   (event line, a countdown on the star map and in the diplomacy panel), so
   rivals can try to stop it by defeating that player first. Losing the
   library cancels it; it can be restarted.
2. **Ascending.** When the project completes, the player's possessions
   leave the material plane: units, buildings, cities, claims and stores are
   removed (buildings leave ruins that decay; nothing passes to another
   player), and the player becomes **ascended**: never defeated, not counted
   by any victory or defeat check, keeping its name, colour, race and
   diplomatic relations.
3. **The ascended view.** An ascended player sees the whole universe with no
   fog and no tier caps, at every level from planet to star map, and moves
   freely; it cannot give orders to anything material.
4. **The celestial home.** A special system at the galaxy's centre, in the
   real direction of the Milky Way's centre (Sagittarius) on task 59's map,
   beyond the match region and joined to nothing by routes: a black hole
   with a glowing accretion disc and a single mythical planet, generated
   before play (rule 9). Ascended players live there: each has a residence
   (its emblem and colour) on the planet, and the planet's panel lists the
   ascended players with a chat channel among them (computer players
   included, with short lines). Material players can look at it on the star
   map but cannot reach it.
5. **Descending.** An ascended player may descend at any moment, instantly,
   on any open land of any habitable planet (not on buildings, not inside
   another player's borders — say so if the owner's "any point" needs
   more). It gets a single city there, a few citizens (data), the first age
   with no technologies, and resources per kind computed from data: for
   each resource the library's research consumes, 90 % of the sum of all
   library research costs in it; for each other resource a fixed share of
   that total (data). State the resulting numbers. It can ascend again later
   by the same route.
6. **The match without an end.** Remove the end at the last age: the match
   goes on while anyone plays. When one material player or alliance remains
   and every other player is defeated or ascended, it is told so (a notice,
   not an end); it may keep playing, ascend, or leave.
7. **Computer players** start the ascension when they have researched
   everything and either face defeat (the military manager's own measure)
   or have nothing left to gain; an ascended computer player descends after
   a while (data) on a planet far from strong enemies.
8. **Determinism and save.** Integers only, id order; the project, the
   ascended state and descents are in the state hash, the snapshot and
   `rom-save` (format bump). Scenarios run twice with equal hashes: the
   project started and completed; the project interrupted by defeat (the
   player is defeated); ascending removing everything; descending with the
   computed resources; ascending a second time.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules); playthrough complete (explain any hash change;
add a scripted ascension and descent to the playthrough or a second one);
duels unchanged in outcome distribution unless explained; the owner's
counter rule test still passing; screenshots you looked at: the Ascension
project with its countdown, the moment of ascension, the celestial home
with the black hole and the residences, the ascended view of a planet, a
descent and the new city; commit on the branch (stage by name), do not
push or merge; final message with commands and real output, files changed,
schema and save-format changes, the descent resource numbers, deviations,
anything not verified.
