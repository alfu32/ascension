# Task 70 — Charted routes, charged jumps, inhibitors and interstellar trade

Crates: `rom-sim` (departure, transit), schema and seed via
`app/db/content/generate.py` (the rule values, one new building and its
component rows, documented in `app/docs/database.md`), `rom-save` (charge
state), `rom-render` and `rom-app` (showing a charge, the inhibitor's reach,
refusal messages), one first-look model file for the new building, docs.
`rom-ai` only as far as charting needs (a later card teaches computer
players to use inhibitors). Starts after tasks 59 and 67 are merged
(they touch `rom-sim`). Implementer: Claude (Opus).

## Why — the owner (2026-11-16)

The owner likes how Reference D handles travel between stars and chose
part of it: **ships keep flying freely inside a star system** (as today);
**the routes between systems** get a jump that must be charged first, and
**inhibitors** that hold enemy ships in a system. Nothing else from
Reference D comes in (no gravity wells, no lanes inside a system, no orbital
slots, levelled capital ships, culture or pirates). Do not use Reference D's
own vocabulary in names; say "jump", "route", "inhibitor".

The owner added (2026-11-17): the **first voyage** along a route to a new
star system takes much longer, because the ship making it is charting the
route. In the expansion age the spaceport offers **enhanced sensors**, which
reveal the routes ahead and allow a first voyage without the delay. In the
galactic age a spaceport research, **automated route survey**, reveals every
route. (The owner says "hyperspace lanes"; in names use "route" or
"hyperspace route", never another game's term.)

And (2026-11-17): freighters also trade **between star systems** along the
routes, so a player with only one planet in a system still has trade; this
comes as an upgrade in the age of abundance.

## Read first

`AGENTS.md`; `app/docs/world-model.md` ("The universe", "Moving between
levels", task 68's "Ships in orbit"); `app/docs/database.md` (space rules,
`route`, `transit`); `app/crates/rom-sim/src/space.rs` (`depart_ships`,
the `"depart"` blocked status, `space.route_ticks_per_unit`, transits);
`app/docs/tasks/59-3d-star-map.md` (3D route lengths, merged before you);
`app/docs/tasks/model-card-common.md` for the first-look model.

## Build

1. **Charting routes.** Each player knows a route once one of its ships has
   completed a voyage along it, or a research reveals it. Unknown routes are
   not drawn on that player's star map, except as a faint stub leaving the
   systems it has seen. A voyage along a route the player has not charted
   takes several times its normal transit (a rule value from data, about 5×;
   state the value) and charts it on arrival; a ship lost on the way charts
   nothing. Two new spaceport researches, data only:
   - **Enhanced sensors** (expansion age): reveals every route leaving a
     system where the player has a ship or a planet, as they come into view,
     and removes the charting delay;
   - **Automated route survey** (galactic age): reveals every route in the
     universe.
   Known routes are per player, in the state hash, snapshot and save.
   Computer players chart routes too (a minimal change in `rom-ai` so they
   still reach other systems; the full use of inhibitors is a later card).
2. **Charging a jump.** A ship ordered along a route first charges at the
   edge of its system for a time taken from data (a stat on the entity type,
   larger ships longer; a rule default where a type has none). While charging
   it cannot move or fire, it can be hit, and any new order cancels the
   charge. Departure starts when the charge completes; transit time keeps
   coming from the route's length. A group ordered together departs together
   when the slowest has charged.
3. **Inhibitors.** A new space-age building, `bld.jump_inhibitor` (built on a
   habitable planet's surface; its reach is the whole system, from a
   component row, not code). While one stands, enemy ships in that system:
   may depart only back along the route they arrived by, charge longer
   (a percentage from data), and are told why when another route is refused
   (a typed `SimError`, shown as a refusal line). Several inhibitors do not
   stack. Own and allied ships are unaffected. Destroying or capturing the
   building lifts the effect at once.
4. **Interstellar trade.** A spaceport research in the abundance age,
   **Interstellar trade** (data only), lets a space freighter run a route
   between an own city and an own or allied city in **another star system**.
   - It flies the legs in order: lift off, cross the system to the route,
     charge the jump, transit, cross the far system, land beside the city,
     deliver; then back. It uses charted routes only when the player has
     charted them; otherwise the first trip charts them at the slow rate
     (item 1). The shortest known path through several systems is allowed,
     chosen deterministically.
   - Income per arrival grows with the whole path length (route lengths plus
     the distances in each system), at a rate from data, and is worth the
     longer trip: state the wealth per minute of a typical interstellar route
     against a typical route between planets.
   - Enemy inhibitors apply to freighters like any ship; a freighter that is
     destroyed delivers nothing. Pirating is not added.
   - Computer players use it when they own cities in two systems.
5. **Determinism and save.** Integers only, iteration in id order; the charge
   state goes into the state hash, the snapshot and `rom-save` (bump the save
   format). Scenarios run twice with equal hashes: a first voyage charting a
   route (slow) and a second voyage on it (normal); enhanced sensors
   revealing routes ahead and removing the delay; the route survey revealing
   all; a freighter on an interstellar route (before the research: refused
   with a reason; after: a full round trip, paid); a single ship charging and
   departing; a charge cancelled by a new order; a group waiting for its
   slowest ship; an enemy held by an inhibitor (refused route, allowed
   retreat); the inhibitor destroyed mid-charge; save and restore during a
   charge.
6. **Seeing it.** A charging ship shows a charge ring or glow that fills,
   at the system and star-map levels; the selection card says "Charging jump:
   N s". A system under an enemy inhibitor shows a mark on the star map and a
   faint boundary in the system view; hovering it explains. The help line
   and refusal messages say what to do.
7. **Model.** A first-look model for `bld.jump_inhibitor@starfaring`
   following `model-card-common.md` (it joins a later review round; do not
   touch any locked model in `app/docs/art/approved.toml`).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md, release
build, build-rules/check-rules); playthrough complete (explain any hash
change); screenshots you looked at: a fleet charging, a fleet in transit
after charging, a first voyage charting a route (the star map before and
after), the star map after enhanced sensors and after the route survey, a
freighter on an interstellar route (its route card), a system held by an
enemy inhibitor with a refused route
message, the inhibitor model; commit on the branch (stage by name), do not
push or merge; final message with commands and real output, files changed,
schema and save-format changes, deviations, anything not verified.
