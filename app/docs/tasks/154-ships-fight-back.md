# Task 154 — Ships in orbit and in space fight back

Owner (chat, 2026-11-25), verbatim: "i find the ships in orbit to be quite passive while the
enemy bangs at them ... no reaction, units should atack automatically enemy units within sight
or range ( or maybe should i have had simply changed their stance .... then ok )".

Ships start in the default combat stance (aggressive), so this is a bug, not a setting.
Orchestrator's reading of `rom-sim/src/stance.rs::acquire_target`: for an attacker **in orbit**
the candidates are `entities_in_radius(e.pos, …)` plus, only for `orbit_only` weapons, the ground
under the orbit; other ships in the same orbit (kept in `space.orbits`) are never candidates, so
an orbiting ship never picks an enemy ship by itself. Check this, and check ships in **system
space** (between bodies) and ships holding a body (`holds_orbit`, formation places of task 143)
the same way.

Crate: `rom-sim` only, plus its tests (and `app/docs/` notes). Implementer: Claude (Opus). Runs
beside task 145 (rom-render, rom-app): do not touch those crates.

## What changes

1. **Orbit.** An armed ship or satellite in orbit acquires enemy entities in the same orbit
   (same body) that its weapons can hit, within the stance's range (aggressive: the respond
   range; defensive: reach + 3 tiles; stand ground: reach; hold fire: never), nearest first,
   ties by id; plus what it already acquires on the ground. Ground-to-orbit acquisition stays
   as it is.
2. **System space.** Armed ships in system space acquire enemy ships near them by the same
   rules (whatever index space uses).
3. **Fire back.** A unit under attack by an enemy it can hit, standing idle, holding orbit or
   holding a body, answers at once (not after the next 10-tick acquisition round), unless its
   stance is hold fire. Moving units keep moving (as on the ground today); say if the ground
   already does this and match it.
4. **The computer player** gains nothing new to do: its ships benefit by themselves.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests: two fleets in one orbit, one attacks, the other returns fire
  without orders; the same in system space; hold fire does not answer; each scenario run twice
  with equal hashes); fight matrix `unanswered=0`; **duel 20–0**; database loads with lint views
  empty; playthrough and ascension hashes reported with why they moved (or that they did not).
- A screenshot you have Read, saved as `app/docs/img/154-orbit-fight.png`, through the game's
  own paths (a scripted scenario: `app/docs/tasks/154-script.txt`; existing script steps only,
  you may not change rom-app): two fleets in orbit, both firing.
- Report: files changed, choices, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/crates/rom-sim/src/stance.rs`,
`app/crates/rom-sim/src/orbital.rs`, `app/crates/rom-sim/src/space.rs`,
`app/crates/rom-sim/src/combat.rs`, `app/docs/database.md` (stances, orbit).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t154/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/154-ships-fight-back` on branch `task/154-ships-fight-back`;
commit there (stage by name); do not merge, tag or push.
