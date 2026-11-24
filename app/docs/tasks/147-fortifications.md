# Task 147 — Fortifications: walls and gates, defensive lines, minefields

Owner (chat, 2026-11-23), verbatim: "i want a new type of building: fortification: from palisades, walls from classical to the
enlightenment age (inclusive). from industrial to fusion (inclusive) they become defensive line
(complex layers of trench, tank traps and barbed wire), lets also have minefields (own units are
immune to own minefields, enemy units take damage)." The owner's choice: **drag a line, with
gates**: the player drags from point to point and the wall is built as a chain of segments that
follows the ground, with a gate building that lets own units through; defensive lines the same
way; **minefields from the Industrial age**, placed as an area, hidden from enemies until they
trip one or detect it.

Crates: `rom-sim` (segment buildings, blocking, gates, mines), `rom-app` (the drag placement),
`rom-render` (drawing a joined line of segments), `rom-ai` (builds walls and mines through data
if small, else a note), seed and schema (authorised), model files, sounds. Implementer: Claude
(Opus). The biggest card of this batch: plan it in steps and commit each.

## Split in two parts (orchestrator, 2026-11-24, to run beside tasks 151 and 152)

- **147a — simulation, data, models, sounds** (this run): `rom-sim`, `rom-rules` if a new
  component needs loading, seed and schema, `app/db/models/` (one model file per segment type
  and age, gates, mines; `rom-models`/`rom-kit` only if a new part shape is needed), sounds
  (`app/tools/sounds/README.md`, "Baking only new types"; existing files byte-identical).
  A new order `build_line` (or the existing `construct` with a list of points: say which) takes
  the polyline a drag would give and lays the segment and gate sites along the ground; the
  minefield order takes a rectangle. Everything is tested in the simulation (scenarios run twice
  with equal hashes). Screenshots: model sheets via `rise-of-man --model <id>` (existing paths)
  and, if the existing script commands can give the orders, an in-match picture; otherwise say
  so (147b takes the in-match pictures). **Do not touch** `rom-app`, `rom-render`, `rom-ai`:
  other agents work there. The computer player must not be harmed: it ignores the new
  buildings until 147b (its build plan picks by score; make sure walls and mines score nothing
  for it, through data if possible, else report).
- **147b — the drag placement and drawing, the computer player** (after 151 and 152 merge):
  `rom-app` (drag a line, gates on the line, drag a rectangle for mines), `rom-render` (joined
  segments, minefield shown to its owner only), `rom-ai` (builds walls and mines through data;
  never walls itself in), the in-match screenshots.

## What changes

- **The fortification line**: palisade (Ancient? the owner says "from palisades": give the
  Ancient age a palisade if it fits), stone walls Classical to Enlightenment, defensive lines
  Industrial to Fusion (trench, tank traps, barbed wire: a segment model per age), each a chain
  of segment buildings placed by dragging; segments join at their ends and turn to follow the
  drag; they block movement for enemies and own units alike, except through **gates**; gates
  open for own and allied units only. Hit points, armour classes and build time per age;
  siege weapons and artillery break them; walls upgrade their look by age as other buildings do.
- **Minefields** (Industrial and later): an area placed by dragging a rectangle (or a fixed size:
  say which); each mine deals damage to an enemy unit that enters (own and allied units immune),
  then is spent; hidden from enemies unless a detector sees them (say what detects them: e.g.
  engineers or a later sensor; at least a research); visible to their owner.
- The counter rule (card 71) and the fight matrix keep `unanswered=0`; the computer players must
  not wall themselves in.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests; every simulation change adds a scenario run twice with equal
  hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new
  playthrough and ascension hashes and rules fingerprint reported with why they moved (or that
  they did not).
- Screenshots you have Read, saved as `app/docs/img/147-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/crates/rom-sim/src/placement.rs`, `app/crates/rom-sim/src/construction.rs`, `app/crates/rom-sim/src/pathing.rs`, `app/docs/content.md`, `app/docs/computer-player.md`, `app/docs/models-in-meshcraft.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t147/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/147-fortifications` on branch `task/147-fortifications`; commit there
(stage by name); do not merge, tag or push.
