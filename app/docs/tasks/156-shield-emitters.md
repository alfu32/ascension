# Task 156 — Shield emitters: the fortification of the space ages

Owner (chat, 2026-11-25), verbatim: "it would be interesting if later after the fusion age we
get a shield emitter."

Orchestrator's reading (owner to correct before this starts): the fortification line of task
147 continues after the Fusion age. Palisade → stone wall → defensive line (Industrial to
Fusion) → **shield emitters** (from the Spacefaring age, task 155's name for age 10, to the
last age): pylons laid along a dragged line exactly like walls (147b), with an energy wall
between neighbouring pylons that blocks enemy units; a **shield gate** (3 tiles wide, oriented,
as 147b's gates) lets the owner's side and allies through. The emitters draw energy
(`res.energy`) while they stand; without energy the field drops and only the pylons block.

Crates: seed and schema (a fortification kind with an energy upkeep and a field state, if the
147 components cannot express it: authorised), `rom-sim` (field up/down by energy, blocking),
`rom-render` (the energy wall between pylons: a translucent, softly glowing plane following the
line's orientation; down when the field is off), model files (pylon, shield gate pylons), sounds
(hum, field up/down; `app/tools/sounds/README.md`, "Baking only new types"), `rom-ai` (uses them
as it uses walls after 147b). Implementer: Claude (Opus). Starts after 147b and 155 merge.

## What changes

- Defensive lines standing when the player reaches the Spacefaring age become shield emitters
  (as earlier kinds convert, task 147a), keeping their line and orientation.
- Numbers per age (hit points, armour class, energy upkeep per period, build time, price): say
  them; artillery and orbital bombardment break pylons; the field itself has no hit points.
- The counter rule (card 71) and the fight matrix keep `unanswered=0`; the computer players do
  not wall themselves in.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests: a field blocks enemies and lets the owner through its gate;
  the field drops without energy; conversion from the defensive line; each scenario run twice
  with equal hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views
  empty; playthrough and ascension hashes and rules fingerprint reported with why they moved.
- Screenshots you have Read, saved as `app/docs/img/156-*.png`, through the game's own paths: a
  shield line with its gate, up, and the same line down without energy; model sheets.
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/tasks/147-fortifications.md` (both parts and
their reports in `STATUS.md`), `app/docs/database.md` ("Fortifications and minefields (task
147)"), `app/crates/rom-sim/src/fortification.rs`, `app/docs/models-in-meshcraft.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t156/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/156-shield-emitters` on branch `task/156-shield-emitters`;
commit there (stage by name); do not merge, tag or push.
