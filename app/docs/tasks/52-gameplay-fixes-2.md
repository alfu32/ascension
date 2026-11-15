# Task 52 — Gameplay fixes from the owner's second long game

Crates: `rom-sim`, `rom-mapgen`, `rom-app` (only where the app shows what
the simulation does), seed and docs. Runs **right after task 43** is merged,
before cards 44–51. Implementer: Claude (Sonnet; Opus for item 5 or 6 if they
turn out to be pathfinding work) — the owner chose Claude while Codex is at
its weekly limit.

Read `AGENTS.md` (determinism: every simulation fix gets a scenario run twice
with equal hashes), `app/docs/world-model.md`, `app/docs/database.md`
(placement rules, map styles, garrison/containers, production and rally),
`app/docs/tasks/41-gameplay-fixes.md` (what was already fixed) and
`app/docs/tasks/HANDOVER.md` (known open items).

## The owner's report (2026-11-15), and what to do

1. **"Not enough oil wells or other resources — I was not able to find a
   single one."** Today oil is `map_style_feature` `ter.oil`: 1–2 clumps of
   4–6 tiles per player, 8–14 tiles from the start, coverage 1 %. Find out
   whether oil is actually generated on a sphere universe planet
   (`--sphere 256`, several seeds), where, and whether the player can see it
   (task 41 hides it before the industrial age; task 44 will give it a
   readable look). Then make resource deposits (oil, and the terrain other
   gatherers need: mountains for mines, forests for woodcutters) reliably
   present near every start and spread over the planet so expansion finds
   more, fairly per player (mapgen fairness tests). Report counts per seed
   before and after.
2. **"Mines, woodcutter camps and oil wells can be built outside cities; oil
   wells outside the nation's borders; other harvesting buildings anywhere."**
   Task 41 added `in_own_territory` to every building that had
   `in_city_radius` (in 0.0.43). Reproduce with the current build first:
   check every gather building's placement rows (`placement_rule`) and the
   app's placement preview and the simulation's check for each one; find any
   building that lacks a city-radius or territory rule, any path that skips
   the check (preview vs order, queued construction, automated upgrades
   `bld.auto_*`, sites started by computer players), and close it. Rule the
   owner wants: every harvesting building inside a city's radius **and**
   inside the owner's borders. Scenario tests for each building type.
2b. **Owner decision (2026-11-15): free colonisation from the enlightenment
   age.** "Colonisation since the 1600s has been done in spite of any
   national borders: if it is free, you can build cities." From the
   enlightenment age on (ordinal 4; data, not names: an age flag or a
   placement rule that only applies before that age, never a key in code), a
   player may found a **city** anywhere on any planet that is not inside
   another player's territory; their own borders no longer limit where a
   city goes. Before that age today's rules stay. Only cities change: every
   other building keeps its rules (harvesting buildings inside a city's
   radius and the owner's borders, item 2). City spacing still applies.
   The placement preview must also refuse, with a clear message, a site no
   builder of the player can reach (for example a city placed on another
   planet while the selected worker is on the home planet — carrying units
   there is task 54). Scenario tests: before and after the age, on free
   land, on own land, on foreign land.
3. **"We do not need that much forest nor that many mountains."** Forest
   coverage is 24 % and mountains 7 % of land in both map styles. Reduce
   them (start around forest 12 %, mountains 4 %; tune by screenshots of the
   home region and the whole globe on several seeds) while keeping every
   start with enough wood and stone nearby (fairness tests).
4. **"Units that are working inside buildings should be garrisoned inside."**
   Workers assigned to a building that employs them (gather slots, research
   or production workers, whatever the rules call working *in* a building)
   should be inside it: not drawn, not targetable, counted in the building,
   leaving it when ordered elsewhere. Use the existing container/garrison
   mechanism (`entity_container`) if it fits; report what you chose.
   **How the player reaches them (owner asked; proposed design):** selecting
   the building shows the workers inside as portraits in the selection
   panel's unit strip with a count ("Workers 4/10"); clicking a portrait
   selects that worker (it stays inside until it gets an order, then walks
   out to carry it out); shift-click adds, double-click selects every worker
   of that type inside; a "Release all" button (with a hotkey) sends everyone
   out to the rally point, or beside the door; an order given to the building
   itself (right-click a forest from a woodcutter's camp) applies to everyone
   inside; the idle-citizen button keeps skipping working citizens. The
   simulation needs a public way to list a building's occupants and an
   order to release them (one way in: orders only).
5. **Docks: ships stay at 100 % in the queue, and appear on land.** "Ships are
   not cleared from docks; they stay completed at 100 % in the queue. If the
   dock's gate points the wrong way (towards land) I had ships commissioned on
   land, inside the dock, unable to move. Make them get out to the rally
   point, and in the case of docks, onto water." Fix: a finished unit always
   leaves the queue; a produced unit spawns on the nearest free tile its
   movement class can use (water for ships), searching around the whole
   footprint, not only in front of the gate; it then moves to the rally
   point (set the rally point on water for docks by default). If no such tile
   exists, keep the unit queued and say why in the HUD, never spawn it
   stuck. Scenario tests: a dock with water only behind it; a full queue.
6. **"I had citizens trapped inside the building they had built."** Builders
   standing on the footprint when construction completes (or when a site is
   placed over them) must be moved out to the nearest free walkable tile.
   Scenario test: builders inside the footprint at completion are all outside
   and can path afterwards.

## Done when (from `app/`)

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint); rom CLI build-rules,
check-rules, playthrough (report the hash and explain changes — they are
expected); determinism scenarios for items 2, 4, 5 and 6 run twice with equal
hashes. Screenshots you looked at: the home region and the whole globe on
three seeds (resources and less forest and mountain), a dock with its ship
on water, workers inside a building (selection panel shows them). Commit on
the branch (stage by name), do not push or merge; final message with commands
and real output, causes found for each item, files changed, deviations,
anything not verified.
