# Task 95 — Projectiles you can see: arrows, bullets, rockets, shells; glowing beams

Crates: `rom-render` (drawing), `rom-app` (what the snapshot says about each shot),
`rom-kit`/model files for the big projectiles, content data only for a per-weapon projectile
kind (schema change, stop and report if needed). Implementer: Claude (Opus).

## Why — the owner (2026-11-18)

"I want to see projectile weapons throwing projectiles (arrows, bullets, rockets, artillery
projectiles). Big projectiles should have a model, small ones can be simple lines; for the laser
beams preferably make them glowing or something more spectacular."

Today a shot is drawn as a tracer (task 66: thin lines with a bright head) from the snapshot's
recent-shot data (`recent_shot` in `sim_thread.rs`, `projectile_speed` of the weapon).

## Build

1. **What a weapon throws comes from data** (rule 2): a projectile kind per weapon — `arrow`,
   `bolt`, `stone` (slings, catapults), `bullet`, `shell` (cannon and artillery), `rocket`,
   `missile`, `torpedo`, `beam` (lasers), `plasma`, `railgun` — derived from the weapon's
   existing data where possible, or a new `et_weapon` column filled by `generate.py` (a schema
   change: add it deliberately, note it in `database.md`). No branching on unit keys.
2. **Flight**: presentation only, never the simulation. Each projectile flies from the shooter's
   muzzle to the target over the time the simulation already allows (`projectile_speed`), along
   a line (bullets, beams) or an arc (arrows, stones, shells; height from distance), turning to
   face its velocity; it vanishes at impact with a small impact effect (dust, spark, flash) by
   kind. Interpolated on the render thread between snapshots so it moves smoothly.
3. **Look**:
   - small (bullets, bolts): a short bright streak; volleys of many shooters stay cheap;
   - arrows: a thin shaft with fletching (a tiny model or a textured quad);
   - big (stones, shells, rockets, missiles, torpedoes): small models from the part kit, with a
     smoke trail for rockets and missiles;
   - beams (lasers): a glowing beam with a bright core, a soft halo (additive blending), a
     short flicker and a glow at the impact point; plasma as a glowing bolt;
   - space weapons the same, in space and between orbit and ground.
4. **Budget**: instanced drawing; a cap on projectiles in flight with the oldest small ones
   dropped first. `--bench` on a 200-versus-200 battle (`rom battle` scene in the app, or a
   script) with projectiles on and off: report both.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/art-direction.md`, `research/02-rendering-ui-stack.md`,
task 66's card, `rom-app/src/sim_thread.rs` (`recent_shot`), the tracer drawing in `rom-render`,
`db/content/generate.py` (weapons).

## Done when

- Gate green from `app/`; playthrough hash unchanged (presentation only; a new data column is
  not hashed by the simulation, or say why it is).
- Tests: the kind comes from data for every weapon; a projectile's flight ends at the target
  at the simulation's impact tick; the cap holds.
- Screenshots you looked at, in `app/docs/img/`, scripts in `app/docs/tasks/95-*-script.txt`:
  archers volley, musket line, artillery shells in flight, rockets with trails, ships firing,
  lasers in space, beams from orbit.
- Bench numbers.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, deviations, anything not verified.
