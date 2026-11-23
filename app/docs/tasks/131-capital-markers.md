# Task 131 — Capitals marked clearly: the city, its planet and its system

Owner (chat, 2026-11-23). Crates: `rom-render` (the markers), `rom-app` (passing the known
capitals into the views; the selection panel's title). Implementer: Claude (Opus). Starts after
task 130 is merged: 130 adds what each player knows about each capital
(`World::known_capitals`, in the snapshot); this task draws it.

## Why — the owner, verbatim

"i also want the capitol city, planet and system to be marked clearly as well for the player as
for the adversaries"

Who sees what (the owner, 2026-11-23): **your own capital is always marked**. An adversary's
capital is revealed gradually by research (task 130): its **system**, then its **planet**, then
its **city**, each a level of `KnownCapital`; plain sight also counts, and the Galactic
Empires upgrade Total Network reveals all. This task marks exactly what `known_capitals` says,
at the level it says: a system-level capital marks only its system on the star map, a
planet-level one also its planet, a city-level one also the city.

## Known before starting (task 130, release 0.0.124)

- `World::known_capitals(viewer)` (`rom-sim/src/intel.rs`): per owner the level (system, planet,
  city), the system, the body (planet level and up), the city and its position (city level only),
  and whether it is visible now. Own and allied capitals at city level always. The views get it
  through the world the sim thread already reads (`rom-app/src/sim_thread.rs`).
- Total Network is a research at the existing Deep Space Tracking Station (`bld.jump_detector`),
  not a new building.
- Baselines on main (0.0.124): tests 1339 passed / 6 ignored; playthrough 7e76f479ddb35b66,
  ascension 522a00b42b24ed73; fight matrix 270/0; duel 20–0; rules 44ec2cf65bf109a9; save 18.

## What exists

- `World::capital(player)` (`rom-sim/src/capture.rs`): each player's capital city (its first
  complete city), saved and hashed. The capital rules (capture, hold, elimination by capital)
  and the "Your capital is under attack" alert already use it.
- Fog of war: `vision.rs` (`visible_entities`, `remembered_buildings`), `jumps.rs`
  (`explored_systems`). The views draw remembered buildings at their last known place.
- Nothing marks a capital on screen today: not in the world, the minimap, the system view or the
  star map. (`CAPITAL_CLASS` in `rom-render/src/layers.rs` is about capital *ships*; leave it.)

## What changes

1. **Which capitals a player knows**: from task 130's `known_capitals` in the snapshot (owner,
   level, city, body, system, last known position, visible or remembered). No new sim state here.
2. **The city, in the world view.** Over each capital known at city level: a capital emblem (a crowned star or
   a small crown over a pennant, drawn as HUD vector art in the owner's player colour, with a
   dark outline so it reads on any ground), standing above the city's tallest model, always
   facing the camera, at a constant screen size within a range of zoom (readable from the normal
   camera height up to the whole-planet view), never hidden by terrain or buildings (drawn after
   the world, like the health bars). Around the city on the ground: a ring in the owner's colour,
   wider than the city's territory circle, so the capital stands out from other cities at a
   glance. Remembered (not currently visible) enemy capitals use the same emblem faded, as
   remembered buildings are drawn now.
3. **The minimap**: the capital emblem in the owner's colour at the capital's place, larger than
   building dots, drawn above them.
4. **The planet** (system view, and the planet seen from orbit): a planet that holds a capital
   known at planet level or finer carries the emblem beside it in each such owner's colour (side by side when players
   share a planet), plus a thin ring around the planet in the colour of the capital's owner.
   The home planet of the viewing player gets this from the first tick.
5. **The system** (star map): a system that holds a capital known at any level shows the emblem next to its
   star, in each owner's colour, so a glance at the star map shows where every known capital
   is. Fits with the existing system labels and markers without overlapping them.
6. **The selection panel**: a selected capital city's title reads "<city name> — Capital" (or
   the panel's existing style for a subtitle), and its tooltip says that losing it can end the
   game when the match uses the capital elimination rule.
7. The emblem is one vector icon added to the HUD's icon set (`hud_icons.rs`, `app/docs/hud.md`):
   look-and-feel as the existing HUD icons; no fantasy subjects.

Presentation only: no simulation rule, order or balance changes, so the playthrough and
ascension hashes, the fight matrix and the duel do not move.

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests, with new tests that each
  known level draws its markers and nothing finer (system only; system and planet; all three),
  own capital always, remembered capitals faded. Playthrough and ascension hashes unchanged from main when you start (report
  them).
- Screenshots you have looked at with the Read tool, saved as `app/docs/img/131-capital-*.png`:
  the own capital in the world view at normal height and zoomed out to the whole planet, the
  minimap, the system view with the home planet marked, the star map with the own system and
  an enemy capital's system marked (a scripted or saved match where the enemy capital has been
  discovered), and an enemy capital remembered under fog. Use the game's own screenshot and
  bench paths, as earlier tasks did.
- Report: files changed, the choices you made where this card left one, the screenshots, and
  anything you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/hud.md`, `app/docs/world-model.md`,
`app/docs/tasks/130-capital-reveal-ladder.md` and its `known_capitals`, `rom-sim/src/capture.rs`,
`rom-render/src/hud_minimap.rs`, `rom-render/src/overlay.rs`, `rom-render/src/scene/space.rs`,
`rom-render/src/bars.rs`, `rom-app/src/shell.rs` (`capital()`).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl`
or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t131/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/131-capital-markers` on branch
`task/131-capital-markers`; commit there (stage by name); do not merge, tag or push.
