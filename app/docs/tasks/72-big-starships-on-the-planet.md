# Task 72 — Starships much bigger on the planet; freighters that explain themselves

Crates: `rom-render` (planet-view ship drawing, picking), `rom-app` (orders
panel, hints), `et_view_scale` rows through `app/db/content/generate.py`,
`rom-sim` only for item 2 (landing clearance), docs. No model file changes
(all models are locked). Starts after tasks 59, 74 and 70 are merged
(they work in `rom-render` and `rom-app`). Implementer: Claude (Opus).

## Why — the owner (2026-11-17)

"The starships in planetary view (either hovering or landed) should be much,
much bigger, 7–10 times bigger." And: "I don't get how interplanetary (or
interstellar) commerce vessels work."

## Build

1. **Size in the planet view.** Every starship class (survey ship, corvette,
   frigate, cruiser, battleship, carrier, transport, freighter; not orbital
   craft or the satellite) is drawn 2–3 times its 0.0.71 size in the
   planet view (owner 2026-11-17, after seeing 7–10×: too big), in orbit and landed, through `et_view_scale.planet_pct`
   (data). Keep the classes in size order. Fix the existing mismatch task 69
   found: the drawn model, its pick box, health bar and hull length must all
   use the same size (`scale_ship_view` scales `ModelRef.size`, which
   `ModelGpu::place` never reads). The hand-over to the system view
   (task 69's morph) must stay smooth at the new size.
2. **Landing a big ship.** A landed carrier, transport or freighter at this
   size covers many tiles. Landing checks the drawn footprint (a landing
   footprint per ship type from data: a stat, not the view scale; the
   simulation must not read presentation rows), used by
   `World::landing_problem` and the freighter's landing legs. The owner's
   rule (2026-11-17):
   - the footprint may not cover a **building** or a **mountain**;
   - it **may** cover forest (trees stay; the ship stands on them);
   - it **may** cover water in part, as long as a small share of the
     footprint is land (a constant from data, about 20 %; state the value).
   Anything else on the ground is handled as today. Deterministic, tested
   twice with equal hashes (one test per rule above), saved if state is
   added.
3. **Hover altitude by size.** Ships holding over the planet hover at an
   altitude that grows with their size (owner, 2026-11-17: bigger ships
   higher, smaller ones lower), so a mixed fleet over one point stacks in
   height instead of merging into one blob: altitude from data per ship type
   or from its size, in size order. Ships of one class over the same point
   still spread sideways by size. Say whether altitude is presentation only
   or simulation state (ground fire and batteries, task 75, will care); if
   it is simulation state, it is integer, hashed and saved. The hand-over to
   the system view stays smooth from every altitude.
4. **Freighters explain themselves.** A "Trade route" button on a selected
   freighter (and on caravans), with a hint: "Pick a city on another planet
   of this system". While picking, valid target cities are ringed; invalid
   ones say why on hover (same planet, no city of yours where the freighter
   is, another system before the abundance-age research Interstellar trade
   from task 70). If task 70 is merged first, cities in other systems are
   valid targets once that research is done, and the route card shows the
   whole path. The selection card shows the route ("City 1 → City 3,
   +188 per trip") and the next leg. The help line and
   `app/docs/world-model.md` describe it in one short paragraph.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules); playthrough complete (explain any hash change);
`approved_models_are_unchanged` green; screenshots you looked at: ships of
every class in orbit over a city at the new size (stacked by altitude), a
landed transport on forest and a freighter partly over water beside a city, the zoom from orbit to system view at three steps,
the freighter's Trade route button, hint and route card; bench of a planet
view with 20 ships in orbit (before and after, with `uptime` load). Commit
on the branch (stage by name), do not push or merge; final message with
commands and real output, files changed, data and schema changes,
deviations, anything not verified.
