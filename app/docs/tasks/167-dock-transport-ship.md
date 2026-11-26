# Task 167 — The dock transport ship: carry land units across water from the Medieval age

GitHub issue #170.

Owner (chat, 2026-11-25), verbatim: "we need a dock transport ship that can take personnel
( units ) onboard since age 2 or 3." Ages are counted from 0; the owner accepted age 2, the
Medieval age (`age.medieval`), on 2026-11-26.

## Today

- No ship at sea carries land units.
- The containers in `et_container` are the city garrison and the starships. The space
  transport (`unit.transport_ship`, capacity 20, tag `passenger`) lands from orbit and lifts
  off again (task 72, `world-model.md`).
- The war galley, carrack and later ships fight but carry nothing.

## What changes

1. **A new sea transport line** trained at the dock (`bld.dock`), one class that changes its
   look with the age like other unit lines (`docs/unit-series.md`):
   - **Medieval:** a cog;
   - **Discovery:** a carrack-sized transport;
   - **Industrial:** a steam transport;
   - **Modern:** a landing ship.

   Name and model each look; the names must be ordinary naval vocabulary. Movement class
   `ship`. It is unarmed or lightly armed.
2. **Capacity.** Use an `et_container` row with a tag for what it accepts: land units (foot,
   wheeled and tracked), not ships, aircraft or buildings. Capacity counts in `pop_cost` units,
   as for the space transport: about 10 in the Medieval age, rising by age through upgrades or
   later looks. Add the tag if it is missing, as data.
3. **Orders**, the same ones the space transport and the garrison use where they already
   exist; add only what is missing:
   - **Board:** right-click own land units onto a transport near the shore, or select units and
     right-click the transport.
   - **Unload:** at a coast tile; the units step onto the nearest free land tiles.
   - **Show what is on board:** the selection panel lists the passengers.
4. **Losing the ship.** The passengers are lost with it, deterministically, and the loss shows
   in the event line.
5. **Computer players** use transports to reach land they cannot walk to: an island, or another
   continent on the same planet. They do this at least when the enemy's city is not reachable on
   foot. Show it in an `ai-match` on an island map style.
6. **The counter rule** (card 71): the transport is answered by the warships of its age.

## Crates

Seed and models (new unit rows, looks, container row, tag), `rom-sim` (board and unload
orders at sea, if the space transport's code does not already cover a ship), `rom-ai`
(transport planning), `rom-app` (orders and the passenger list), `rom-render` (the passenger
count badge, if one is missing). Schema change only if the container cannot express it: stop
and report.

## Read first

- `AGENTS.md`, `CLAUDE.md`;
- `docs/database.md` (containers, movement classes);
- `docs/unit-series.md`;
- `docs/world-model.md` (the space transport's landing, task 72);
- `crates/rom-sim/src/` (containers, boarding, the transport landing);
- `crates/rom-ai/src/`;
- `docs/models-in-meshcraft.md`.

## Done when

- Gate green from `app/`:
  - fmt, clippy `-D warnings`, workspace tests (≥ the count on main plus this card's tests);
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views.
- Playthrough and ascension hashes unchanged, or explained.
- **Tests:**
  - board, sail, unload on another shore;
  - capacity refused when full;
  - passengers are lost with a sunk ship;
  - the transport refuses ships and aircraft;
  - each scenario run twice with equal hashes.
- **Screenshots** `docs/img/167-*.png`, each opened and checked:
  - a loaded cog leaving the dock;
  - units stepping ashore on another coast;
  - the selection panel with the passenger list;
  - the model sheets.
- The new model ids are listed for the owner's review round.
- Report as AGENTS.md says. Logs only in the scratchpad's `t167/`; never write `gate*.sh`.
