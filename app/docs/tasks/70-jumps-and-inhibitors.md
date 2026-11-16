# Task 70 — Charged jumps and inhibitors on the routes between systems

Crates: `rom-sim` (departure, transit), schema and seed via
`app/db/content/generate.py` (the rule values, one new building and its
component rows, documented in `app/docs/database.md`), `rom-save` (charge
state), `rom-render` and `rom-app` (showing a charge, the inhibitor's reach,
refusal messages), one first-look model file for the new building, docs.
No change to `rom-ai` beyond keeping its tests green (a later card teaches
computer players to use inhibitors). Starts after tasks 59 and 67 are merged
(they touch `rom-sim`). Implementer: Claude (Opus).

## Why — the owner (2026-11-16)

The owner likes how Reference D handles travel between stars and chose
part of it: **ships keep flying freely inside a star system** (as today);
**the routes between systems** get a jump that must be charged first, and
**inhibitors** that hold enemy ships in a system. Nothing else from
Reference D comes in (no gravity wells, no lanes inside a system, no orbital
slots, levelled capital ships, culture or pirates). Do not use Reference D's
own vocabulary in names; say "jump", "route", "inhibitor".

## Read first

`AGENTS.md`; `app/docs/world-model.md` ("The universe", "Moving between
levels", task 68's "Ships in orbit"); `app/docs/database.md` (space rules,
`route`, `transit`); `app/crates/rom-sim/src/space.rs` (`depart_ships`,
the `"depart"` blocked status, `space.route_ticks_per_unit`, transits);
`app/docs/tasks/59-3d-star-map.md` (3D route lengths, merged before you);
`app/docs/tasks/model-card-common.md` for the first-look model.

## Build

1. **Charging a jump.** A ship ordered along a route first charges at the
   edge of its system for a time taken from data (a stat on the entity type,
   larger ships longer; a rule default where a type has none). While charging
   it cannot move or fire, it can be hit, and any new order cancels the
   charge. Departure starts when the charge completes; transit time keeps
   coming from the route's length. A group ordered together departs together
   when the slowest has charged.
2. **Inhibitors.** A new space-age building, `bld.jump_inhibitor` (built on a
   habitable planet's surface; its reach is the whole system, from a
   component row, not code). While one stands, enemy ships in that system:
   may depart only back along the route they arrived by, charge longer
   (a percentage from data), and are told why when another route is refused
   (a typed `SimError`, shown as a refusal line). Several inhibitors do not
   stack. Own and allied ships are unaffected. Destroying or capturing the
   building lifts the effect at once.
3. **Determinism and save.** Integers only, iteration in id order; the charge
   state goes into the state hash, the snapshot and `rom-save` (bump the save
   format). Scenarios run twice with equal hashes: a single ship charging and
   departing; a charge cancelled by a new order; a group waiting for its
   slowest ship; an enemy held by an inhibitor (refused route, allowed
   retreat); the inhibitor destroyed mid-charge; save and restore during a
   charge.
4. **Seeing it.** A charging ship shows a charge ring or glow that fills,
   at the system and star-map levels; the selection card says "Charging jump:
   N s". A system under an enemy inhibitor shows a mark on the star map and a
   faint boundary in the system view; hovering it explains. The help line
   and refusal messages say what to do.
5. **Model.** A first-look model for `bld.jump_inhibitor@starfaring`
   following `model-card-common.md` (it joins a later review round; do not
   touch any locked model in `app/docs/art/approved.toml`).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md, release
build, build-rules/check-rules); playthrough complete (explain any hash
change); screenshots you looked at: a fleet charging, a fleet in transit
after charging, a system held by an enemy inhibitor with a refused route
message, the inhibitor model; commit on the branch (stage by name), do not
push or merge; final message with commands and real output, files changed,
schema and save-format changes, deviations, anything not verified.
