# Task 102 — Projectiles take their flight time: damage lands when the shot arrives

Crates: content data (`db/content/generate.py`, regenerated `db/seed.sql`), `rom-render`
(`projectiles.rs`: drawn speeds come from the data), `rom-sim` only if a test or a fix needs it.
Implementer: Claude (Opus).

## Why — the owner (2026-11-19)

Task 95 drew every projectile, but only the city, tower and fort arrows have a
`projectile_speed`; every other weapon hits the tick it fires, so the drawn shot lands after the
damage. The owner: "counted when the projectile is supposed to land (do it by the numbers … not
reactive to the event of landing, the flight is pure animation)."

The simulation already does this for any weapon with `projectile_speed > 0`
(`rom-sim/src/combat.rs`): at firing it computes `travel = distance × tick_rate / speed` and queues
a `PendingHit` with `land_tick`; `land_projectiles` applies it at that tick to the target by id,
wherever the target is by then. No collision, no dodging, no event from the renderer. Keep exactly
that model.

## Build

1. **A speed per projectile kind, in data.** In `generate.py`, a table of `projectile_speed`
   (milli-tiles per game second) by `et_weapon.projectile` kind, applied to every weapon of that
   kind (a per-weapon override only where a weapon clearly differs, e.g. a howitzer shell vs a
   tank shell, stated in a comment). Start from the speeds task 95 draws
   (`rom-render/src/projectiles.rs`, `Look::speed`: arrow 9, bullet 40, shell 24 tiles/s …) and
   keep them believable at game scale: arrows and stones slow enough to see, bullets fast.
   `beam` stays instant (`0`, light speed); decide `railgun` and `plasma` and say why.
   The existing `ARROW_SPEED` of the defences fits into the same table.
2. **Orbit and space.** Check how `distance` is measured for shots to and from orbit and between
   ships in space, and that the travel times there come out sensible (seconds, not minutes). Fix
   only by data if possible; if the simulation needs a change, keep it small, deterministic and
   tested, and say so.
3. **The renderer draws what the simulation says.** With a speed in the data, `ShotView.land_tick`
   is the simulation's; remove or reduce the renderer's own speed table so a kind's drawn speed
   cannot disagree with the data (beams keep their instant line and flicker).
4. **Balance.** Slower shots change fights (a target that dies before a volley lands wastes it;
   kiting changes). Run the fight matrix and both duel tables; the counter rule must still hold
   (`unanswered=0`). Report the duel tables before and after. If a matchup breaks, adjust speeds
   (data) before anything else, and say what moved.
5. **Re-pin.** The playthrough and ascension hashes will change: report the new ones and update
   anything that pins them (grep the tree).

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` (et_weapon, the task 95 projectile
note), task 95's card, `rom-sim/src/combat.rs` (firing and `land_projectiles`),
`rom-render/src/projectiles.rs`, `rom-app/src/sim_thread.rs` (`recent_shot`, `travel_ticks`),
`db/content/generate.py` (weapons, `PROJECTILES`).

## Done when

- Gate green from `app/` (fmt, clippy, tests, `generate.py --check`, database lint, release build,
  build-rules, check-rules, playthrough); new hashes reported.
- `rom fight-matrix --db target/rules.db --jobs 2`: `unanswered=0`.
- Duels, `--seeds 1..100 --jobs 2`, profiles `balanced:moderate,balanced:easiest` and
  `balanced:moderate,balanced:moderate`, before (0.0.95: 95–0 with 5 draws; 50–41 with 9 draws)
  and after.
- Tests: every weapon with a travelling projectile kind has a speed from data; a shot's damage
  lands at `fire tick + travel` and not before (scenario test, run twice, same hash); a beam still
  lands the tick it fires; the drawn flight ends at the simulation's land tick.
- Screenshots you looked at (`app/docs/img/102-*.png`): an archer volley and an artillery shell
  mid-flight with the target still at full health.

Memory rules: `CARGO_BUILD_JOBS=4`; duels `--jobs 2`; check `free -g` before heavy steps; run the
app one instance at a time under `systemd-run --user --scope -q -p MemoryMax=6G -p
MemorySwapMax=0 env -u WAYLAND_DISPLAY …` in self-ending modes. Never `xdotool`, `wmctrl` or
`sudo`; do not stop processes you did not start. Wait loops must not use a `pgrep -f` pattern
that matches the loop itself. The session scratchpad is shared: write your logs in your own
subfolder and never overwrite `scratchpad/gate.sh`.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the speeds chosen and why, balance before/after, deviations, anything not
verified.
