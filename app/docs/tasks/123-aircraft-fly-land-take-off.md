# Task 123 — Jets and biplanes fly in the air, land and take off

Crates: `rom-sim` (the landed/airborne state, take-off and landing, who can hit whom), `rom-render`
(the altitude, climbing and descending), `rom-app` (the order buttons and hotkeys only),
`app/db/schema.sql` + seed (one movement column; schema change authorised here), `rom-ai` only
through data, docs. Implementer: Claude (Opus). Starts after task 118 is merged (both touch
`rom-sim` and the seed); task 119 (helicopters) builds on it and starts after it.

## Why — the owner (chat, 2026-11-21)

Found in task 122: jets and biplanes are drawn at ground height; only orbital craft climb (the
orbital fighters and bombers have two planes of action: the air above their planet and its
orbit). The owner: "jets and biplanes flying like the orbital fighters and bombers have 2 planes
of action > ok, they can land and takeoff ( but not in the orbital plane obviously )."

## Design

- **Two planes for aircraft**: *landed* (on the ground) and *airborne* (the air above the
  planet). Never orbit: that stays the orbital craft's (`climb_ticks`).
- **Which units**: every unit whose movement class is the air class and that is not an orbital
  craft (biplanes, fighters, jet fighters, bombers, drone fighters, interceptors and the like —
  list them from the seed). Data, not names: a new `et_movement.takeoff_ticks` (ticks to take
  off or land; 0 = never lands, always airborne, as today). Orbital craft keep their behaviour.
- **Airborne** (as today in the simulation): flies over everything, hit only by weapons that hit
  air. **Drawn at a flight altitude** like the orbital craft's air plane — the same kind of height
  and shadow (look at `rom-render/src/layers.rs` `hover_altitude`, `entities.rs` aloft ships, and
  how orbital craft are drawn when they come down into the air); bigger aircraft may fly a little
  higher; never inside terrain or buildings.
- **Landed**: on the ground at its spot, drawn on the ground; cannot attack; can be hit by ground
  weapons as well as anti-air (it is a ground target while landed); does not block ground units
  more than a small vehicle would.
- **Taking off / landing** takes `takeoff_ticks` (drawn as a climb or descent along the heading);
  while doing so the aircraft is hit as if landed and cannot attack.
- **Orders**: **Land** (on open ground at its position, or at a clicked spot; an airfield if one
  is clicked) and **Take off**. A move, attack or patrol order given to a landed aircraft takes
  off first, then carries out the order. Aircraft come out of the airfield landed beside it, or
  airborne if the producer is not an airfield — pick one rule, from data if possible, and write it
  in the docs. Idle airborne aircraft stay airborne (circling their spot, drawn only).
- All integer, deterministic, saved (save format bump if the state is new) and hashed; one way in
  (orders only).

## Build

1. Schema: `et_movement.takeoff_ticks` with a CHECK; seed values for the aircraft (e.g. 60–100
   ticks); lint views empty; `database.md` updated.
2. `rom-sim`: the state, the two orders, take-off before move/attack, targeting rules for landed
   aircraft; unit tests and a twice-run hash scenario.
3. Snapshot and `rom-render`: altitude by state (airborne at flight height, landed at ground,
   climbing/descending in between), shadows on the ground under airborne aircraft.
4. `rom-app`: Land and Take off buttons in the order panel (the existing Land button of ships and
   craft, if it fits) with hotkeys; tooltips.
5. Computer players: they keep using aircraft airborne; check they do not leave them landed.
6. Docs: `world-model.md` (aircraft planes), `unit-series.md`, the order list in the HUD docs.

## Done when

- Gate green; fight matrix `strengths=255 unanswered=0`; duel 20–0; new hashes reported with
  what moved them.
- Screenshots you looked at: `app/docs/img/123-jets-airborne.png` (jets fighting in the air over
  a city, shadows below) and `app/docs/img/123-jets-landed.png` (aircraft landed at an airfield).
  Keep the session script as `app/docs/tasks/123-aircraft-script.txt` (the dogfight script of task
  122, `122-dogfight-script.txt`, is a good start).

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql` (`et_movement`,
`movement_class`), `app/docs/world-model.md`, `app/docs/unit-series.md`, `rom-sim/src/orbital.rs`
(how craft change planes), `rom-sim` combat targeting by movement class, `rom-render/src/layers.rs`
and `entities.rs` (aloft drawing), `app/docs/tasks/122-dogfight-script.txt`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big test runs and every game run under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0` (game: `env -u WAYLAND_DISPLAY`,
self-ending). Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start (the
owner may be playing). Logs only in the scratchpad's `t123/`; never write `scratchpad/gate*.sh`;
no cargo target dir in the scratchpad or /tmp. Work in `../rise-of-man-worktrees/123-aircraft`
on branch `task/123-aircraft`; commit there; do not merge, tag or push.
