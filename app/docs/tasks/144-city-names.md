# Task 144 — Cities have names

Owner (chat, 2026-11-23), verbatim: "let the cities have names". The owner's choice: **nation lists, renamable**: each nation
has a list of its real historical city names (invented lists for the Reptilian broods and the
Avian flights); a new city takes the next unused one; the player can rename a city. Names show
over the city, on the selection panel and in alerts.

Crates: `rom-sim` (the name in the city's state, the rename order), `rom-save`, `rom-app` (the
rename field on the selection panel, alerts), `rom-render` (the name label over the city and on
the minimap's tooltip), seed and schema (authorised: a `faction_city_name(faction, ordinal,
name)` table or similar). Implementer: Claude (Opus).

## What changes

- Lists: for each human nation in the seed, about 20 real historical city names of that people
  (public knowledge: no lists copied from a game or a share-alike source; write them yourself);
  for blocs and Humanity the names continue from the player's original nation; for the
  invented peoples, invented names in a consistent style per brood/flight (no names from
  fiction or games). When a list runs out, "New <name>" then numbered.
- The capital keeps the first name. A captured city keeps its name. Network play: the name is
  part of the world state (same on every machine).
- Rename: a field on the selection panel for an own city, a new order `rename_city` (text limit,
  printable characters only; filtered for control characters).
- Shown: over the city (with the capital emblem of task 131 beside it), selection panel title,
  alerts ("Roma is under attack"), the match report.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests; every simulation change adds a scenario run twice with equal
  hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new
  playthrough and ascension hashes and rules fingerprint reported with why they moved (or that
  they did not).
- Screenshots you have Read, saved as `app/docs/img/144-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/crates/rom-sim/src/territory.rs`, `app/crates/rom-sim/src/capture.rs`, `app/docs/hud.md`, the seed's faction rows.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t144/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/144-city-names` on branch `task/144-city-names`; commit there
(stage by name); do not merge, tag or push.
