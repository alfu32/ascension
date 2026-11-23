# Task 140 — Support units: healing, repair, attrition relief, and selection with the army

Owner (chat, 2026-11-23), verbatim: "medics and supply wagons are considered civilian units and i cannot select them
together with military units, lets change that." "supply wagons repair vehicles and medics heal
people, priests heal personnel units and citizens too." "citizens can repair buildings at no
cost." "supply wagons cancel attritional damage around them on a certain area."

Crates: `rom-sim` (healing, repair, attrition), `rom-app` (selection rules), seed and schema
(authorised here: say every column you add), `rom-ai` through data. Implementer: Claude (Opus).

## What changes

1. **Selection**: a box or double-click selection that holds military units also takes the
   medics and the supply wagons (the whole support line of each age: find them by data —
   healers, supply carts — not by key), so they move with the army; citizens and other workers
   still stay out of a military selection as now. Control groups keep any mix.
2. **Who heals whom** (all data, `et_healer` and tags): medics heal biological soldiers (and
   citizens); priests heal biological units, soldiers and citizens alike (today: check what
   they heal and make it so); **supply wagons/carts/trucks repair mechanical vehicles** (a new
   healer row on each supply type, target tag `mechanical`, at a rate in line with the repair
   drone of its age; say the numbers). Healers still never heal themselves and never heal
   enemies.
3. **Citizens repair buildings at no cost**: a damaged own building is repaired by citizens
   (the builder's repair order, `et_builder.repair_pct`) without spending resources; check what
   repair costs today and make it free; idle citizens near a damaged building may repair it on
   their own only if that is how builders already behave (do not add new automation).
4. **Attrition relief**: an own supply unit cancels attritional damage (`rom-sim/src/attrition.rs`,
   damage taken in enemy territory) for own units within a radius (data: e.g. a new
   `et_supply_aura(radius)` row or a column on the supply type; 8 tiles to start). Allies too if
   the attrition code already knows allies; say what you chose.
5. Docs: `app/docs/database.md`, `app/docs/content.md`, `app/docs/computer-player.md` if the AI's
   army composition changes.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests; every simulation change adds a scenario run twice with equal
  hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new
  playthrough and ascension hashes and rules fingerprint reported with why they moved (or that
  they did not).
- Screenshots you have Read, saved as `app/docs/img/140-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/crates/rom-sim/src/healing.rs`, `app/crates/rom-sim/src/attrition.rs`, `app/crates/rom-sim/src/construction.rs` (repair), `app/crates/rom-app/src/selection.rs`, the seed rows of the medic, priest, repair drone and the supply wagon/cart/truck types.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t140/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/140-support-units` on branch `task/140-support-units`; commit there
(stage by name); do not merge, tag or push.
