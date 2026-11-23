# Task 128 — The drone operator's grid of drones: visible, individually targetable, one drone per enemy

Owner feedback on review round 17 (chat, 2026-11-23), following task 120 (issue #3). Crates:
`rom-sim` (the cloud), `rom-rules` (if a cloud field changes), `rom-render` (drawing the drones),
`rom-app` (only `src/gallery.rs`, the model sheet and gallery), `app/db/schema.sql` + seed (the
`et_drone_cloud` row; schema change authorised here), `rom-save` if the cloud's saved state
changes, docs. Implementer: Claude (Opus).

## Why — the owner, verbatim

"i indicated that the drone operator unit launches a cloud of drones, these hover above him, are
part of the unit itself but when attacking the deploy forward to the attacked target, they are
disposed in a rectangular grid of 9 or 16 , these are kamikaze drones so one drone attacks one
enemy unit. all parts of the unit are targetable, drones are targetable individually, they are
constantly replenished though as long as the operator unit is not targeted ( as long as they
live ),. so only the operator unit takes hit points. in this case i dont see in the model review
the drone cloud/grid component. sounds are ok."

## What exists (task 120, `app/crates/rom-sim/src/drones.rs`, read its module comment)

One `Cloud { drones, pos, rebuild }` per operator: a point that flies ahead of the operator on a
leash (`et_drone_cloud`: capacity 16, leash 7000, strike_range 1000, rebuild_ticks 120,
drone_hp 8, drone_speed 5000). Strikes are the operator's weapon shots, each spending a drone.
Weapons that hit the air are **screened** by the cloud: they hit the cloud instead of the
operator and bring down one drone per `drone_hp` of damage. The renderer draws up to 16 tiny
craft swirling round the point; in play they are a few black specks
(`app/docs/img/120-drone-cloud.png`). The gallery and the model sheet draw the operator alone
(`gallery.rs`: `cloud: None`).

## What changes

1. **A grid, hovering above the operator.** The drones fly in a rectangular grid: side =
   ⌈√capacity⌉ (16 → 4 × 4, 9 → 3 × 3), slots filled row by row, spacing and hover height as
   data or derived constants you state (start near 0.6 tile spacing and 1.5 tiles above the
   ground, so the grid reads clearly at the normal camera height). At rest the grid hovers
   over the operator. On a move it flies ahead, and on an attack it deploys forward to the
   target, as the cloud point does now (leash, operator keeping its distance). The grid faces
   the direction it travels.
2. **One drone per enemy.** Kamikaze: a drone leaves its slot, dives at one enemy unit and is
   spent. Drones in flight are reserved for their target: an enemy is not given more drones
   than it takes to kill it (by the damage path's numbers, so infantry gets one and an armoured
   vehicle about ten), and the other drones go to other enemies within strike range, the
   ordered target first, then the nearest. Several drones may be in the air at once. Keep the
   numbers of task 120 (one drone kills or nearly kills same-age infantry; about ten for the
   information-age main battle tank), and state the strike rate you end up with.
3. **Every part targetable; drones one hit each.** No more screening. The operator is a target
   for every weapon that can hit a ground unit. Each drone is a target of its own for weapons
   that hit the air (the drones fly). A hit on a drone destroys that one drone, whatever the
   damage: drones have no hit points, and only the operator does (remove or repurpose
   `drone_hp` deliberately: schema, seed, `database.md`). A weapon that hits both layers picks
   by its usual target choice (nearest or the ordered target); say what you chose.
4. **Replenished while the operator lives.** As now: a new drone every `rebuild_ticks` below
   capacity. It takes off from the operator and flies to the first empty slot. The operator's
   death ends the unit, its grid and its drones in flight.
5. **Drawn so they can be seen.** Each drone is a small quadrotor built from kit parts (a body
   and four rotors, at a size that reads at the normal camera height: start at 0.18 tile
   across), in the owner's team colour, with a gentle bob and a slow rotor shimmer taken from
   the tick and the slot. No randomness outside the sim. A diving drone streaks to its target,
   as now. Fit it into the instanced drawing (task 121: no per-frame GPU buffers).
6. **In the model review.** The model sheet (`rom-app --model <id> --screenshot`) and the
   gallery draw the drone operator with its full grid hovering above it, so the review page
   shows the whole unit. Do this through the type's `et_drone_cloud` data, not its key.
7. **The AI and balance.** The counter rule (card 71, `app/docs/computer-player.md`) must still
   hold: fight matrix `unanswered=0`; duel 20–0. Re-tune within `et_drone_cloud` and the
   operator's weapon rows if needed, and report the numbers.

All integer and deterministic in `rom-sim` (slot positions in milli-tiles; per-drone state, if
any, in id order), saved (bump the save format if the saved cloud changes; old saves load with
a full grid) and hashed, and data-driven (no key branching).

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ 1326 passed, plus your
  new tests: the grid's slots, one drone per infantry target, one hit kills one drone,
  replenishing, a twice-run hash scenario). Playthrough and ascension hashes will move: report
  the new ones and why. Fight matrix `unanswered=0`, duel 20–0.
- A self-ending scripted fight (operators against riflemen, against a tank, against anti-air)
  with outcome numbers, and two screenshots you have looked at:
  `app/docs/img/128-drone-grid.png` (the grid hovering above an idle operator, then deployed
  forward in an attack) and the model sheet of `unit.drone_operator@information` with its grid.
- Report: files changed, what you chose where this card left a choice, the numbers, and
  anything you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/tasks/120-drone-operator.md`,
`app/crates/rom-sim/src/drones.rs`, `app/docs/database.md`, `app/db/schema.sql`
(`et_drone_cloud`), `app/docs/computer-player.md` (counter rule, fight matrix),
`app/crates/rom-render/src/entities.rs` (`CloudView`), `app/crates/rom-app/src/gallery.rs`,
`app/docs/models-in-meshcraft.md` (kit parts).

Baselines on main (0.0.122): tests 1326 passed / 6 ignored; playthrough hash 6fbf29ce22a018b0,
ascension hash e9dea528f5a017ab; fight matrix `strengths=270 unanswered=0`; duel 20–0, mean_tick
54449; rules fingerprint 786304d6bbafd296.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl`
or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t128/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/128-drone-grid` on branch `task/128-drone-grid`; commit
there (stage by name); do not merge, tag or push.
