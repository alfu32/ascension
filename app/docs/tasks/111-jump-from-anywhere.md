# Task 111 — Jump from anywhere in a system; arrive at the edge of an unexplored system or anywhere in an explored one

Crates: `rom-sim` (`jumps.rs`, `space.rs`, travel orders), schema + seed (one `et_inhibitor`
column), `rom-rules` (load it), `rom-app` (the send order on the star map and in the system view,
`space_view.rs` known systems), `rom-render` only for the charge marker and arrival hints,
`rom-ai` only where its travel orders need a destination point. Implementer: Claude (Opus).

## Why — the owner (2026-11-19), verbatim

"lets jump to hyperspace from anywhere inside the syste to:
- the edge of an unknown system
- anywhere inside a known system
wind-up time still applies on departure,
the departure takes place from orbit"

Answers to the follow-up questions (2026-11-19):
- **Routes stay.** A jump still goes along a route to a connected system; a voyage over several
  systems is several jumps. Only the departure point and the arrival point change.
- **Known = explored by the player:** the systems the star map does not label "(unexplored)".
- **Departure from orbit = anywhere in the system's space.** A ship charges where it is; a craft on
  a planet's surface (or in its air) first climbs to orbit, then charges.
- **Inhibitors:** "inhibitors inflict damage only when jumping close to the planet the inhibitor is
  on, so the adversary will still be able to jump inside the system but further away from the
  planet(s) with inhibitor(s)".

## Today (main at 0.0.102; `docs/database.md`, "Routes and jumps (task 70)")

A ship ordered to another system flies to the edge of its system toward the next system
(`World::system_edge`), charges there for its `jump_charge` time (`et_movement.jump_charge_ticks`
or `space.jump_charge_ticks`), departs (`depart_ships`), transits for the route's length and arrives
at the edge of the next system (`arrive_transits`). An enemy jump inhibitor in the departure system
lets a ship leave only back along the route it came by, and lengthens its charge (`charge_pct`);
an enemy inhibitor in the arrival system strikes every arriving ship for `arrival_damage` (task 104).
Which systems a player "knows" is decided in `rom-app` (`space_view::known_systems_with`), not in
the simulation.

## Build

1. **Explored systems in the simulation.** Add `World::system_explored(player, system) -> bool`
   (deterministic; same answer as today's star-map label: home system, systems where the player or
   an ally has or had ships, the celestial home, systems whose star or bodies its team has explored;
   an ascended player knows all). Move the app's rule onto it so the label and the jump agree. If it
   needs new state (e.g. a per-player set of visited systems), keep it in the hash only once
   non-empty, in the snapshot and the save (bump the save format if so).
2. **Departure.** A travel order to another system charges where the ship is (no flight to the
   edge first). A ship on a surface or in the air layer first rises to orbit (the existing climb,
   if one exists; otherwise say what you did), then charges. Charge time, group departure, "any new
   order cancels the charge" and the inhibitor's departure hold (only back along the arrival route,
   longer charge) stay as they are.
3. **Arrival.**
   - The final system of a voyage, explored by the player: the ship arrives at the point the
     order names (any point of that system's space, clamped inside its bounds).
   - Not explored: the ship arrives at the edge of that system facing the system it came from,
     as today.
   - An intermediate system of a multi-jump voyage: the ship arrives at the edge (as today) and
     charges there for its next jump.
   - Transit time still follows the route's length (task 70); charting rules unchanged.
4. **Inhibitor blow by distance.** Replace "every ship arriving in the system" with "every enemy
   ship whose arrival point lies within `arrival_radius` of the body the inhibitor stands on". New
   column `et_inhibitor.arrival_radius INTEGER NOT NULL DEFAULT 0 CHECK (arrival_radius >= 0)` in
   the system's space units (milli-tiles, like positions); 0 = no blow. Choose the seed value so the
   blow covers the planet's orbit and its near space but not the system as a whole (relate it to the
   usual spacing of planet orbits; state the number and why). Several inhibitors: the largest blow
   whose radius covers the point applies (as today). Edge arrivals normally fall outside it.
5. **Orders and UI.** Right-click (the existing send) on a point inside an explored system sends the
   selection there; on an unexplored system it sends them to its edge, and the hover text says so
   ("arrives at the edge"). Show each ship's charge where it charges (today's marker, moved). The
   order is still a `Command`; the destination point travels in it (no back door).
6. **Computer players.** Their voyages keep working; where they choose an arrival point, prefer one
   outside enemy inhibitor radii when the target allows (data: an `ai_weight_key` row if a threshold
   is needed). Do not retune anything else.
7. **Docs.** `docs/database.md` "Routes and jumps" and `docs/world-model.md`: the new departure,
   arrival and inhibitor rules.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md`, `app/docs/database.md` ("Routes and
jumps", "et_inhibitor"), cards 70, 104 and 106, `rom-sim/src/jumps.rs`, `rom-sim/src/space.rs`
(`system_edge`, `depart_ships`, `arrive_transits`), `rom-app/src/space_view.rs`
(`known_systems_with`), `rom-ai` (search `travel` and `jump`).

## Done when

- Gate green from `app/` (fmt, clippy, tests, `generate.py --check`, database lint, release build,
  build-rules, check-rules, playthrough). The playthrough is a flat match; if its hash changes, say
  why. Fight matrix `strengths=255 unanswered=0`.
- Tests (`rom-sim`, each run twice comparing state hashes): a ship charges where it stands (no
  flight to the edge) for its `jump_charge`; it arrives at the ordered point in an explored system
  and at the edge of an unexplored one; a multi-jump voyage charges at each intermediate edge; an
  enemy inhibitor strikes an arrival inside its radius and not one outside; the inhibitor's
  departure hold still works; a surface craft climbs to orbit before charging; save and load mid-
  charge and mid-transit give the same hash.
- `rom universe … --simulate 6000 --space` on Cluster and Sector, seeds 1–3, runs (Sector under the
  memory cap).
- Screenshots you looked at (`app/docs/img/111-*.png`): ships charging away from the edge, and ships
  arriving inside an explored system.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g` and `df -h /tmp`; Sector and larger runs one at
a time under `systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`; run the app one
instance at a time in self-ending modes with `env -u WAYLAND_DISPLAY`. Never `xdotool`, `wmctrl` or
`sudo`; do not stop processes you did not start. Wait loops must not use a `pgrep -f` pattern that
matches the loop itself. Logs only in your own scratchpad subfolder `scratchpad/t111/`; never write
`scratchpad/gate*.sh`; never put a cargo target dir or a source copy in the scratchpad. Never write
the word for star-lane routes other than "route".

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the arrival-radius value and why, deviations, anything not verified.
