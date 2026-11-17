# Task 80 — Destroyed buildings and units leave stains on the ground

Crates: `rom-sim` (the stain list, kept and pruned in the tick), `rom-save`
(stains saved), `rom-render` (drawing them on the terrain), `rom-app`
(snapshot plumbing only), schema and seed (two constants), docs
(`app/docs/world-model.md` or the terrain doc where ground drawing is
described). Starts after task 75 is merged (it touches the same crates; see "Builds on task 75").
Implementer: Claude (Opus).

## Why — the owner (2026-11-17)

"Destroyed buildings and units should leave a stain proportional with their
stature on the ground. The stain should dissipate after some time (a few
dozen rotations)."

A "rotation" is one day of the planet: one orbit of the light that
represents the sun around the planet orb, `world.day_ticks` (6000 ticks, 5
game minutes).

## Builds on task 75

Task 75 (0.0.76) added, for the views only (not hashed, not saved, kept one game minute):
`World::blasts()` (where an area weapon struck) and `World::wrecks()` (buildings that fell,
complete ones only), drawn in `rom-render/src/overlay/effects.rs` as a scorch on the ground for a
minute, a collapsing and burning ruin, and debris pieces. This task makes the lasting mark:

- The stain list of item 4 replaces the one-minute scorch as what remains on the ground; the
  blast's flash, shockwave and fire stay as task 75 made them and hand over to the stain.
- The debris pieces of task 75 (flat dark shards standing on the ground, which read as cards in
  `docs/img/bombard-after.png`) become the building stain's debris of item 2: low, lying on the
  ground, fading with the stain.
- Use the destruction point task 75 already found for wrecks (`remove_dead`), extended to units
  and to buildings under construction, rather than a second one.

## Read first

`AGENTS.md`; `app/docs/database.md` (constants); `rom-sim/src/world.rs`
(`despawn` and its callers: only some of them are destructions);
`rom-sim/src/combat.rs`, `attrition.rs`, `orbital.rs` (where things lose their
last hit points); `rom-sim/src/snapshot.rs` (what the renderer receives);
`rom-save` (format, currently 8 — check after merging task 75);
`rom-render/src/scene/` (terrain drawing, shadows, how units and buildings
are drawn on the cube-sphere), `rom-render/src/sun.rs` (`day_ticks`);
`db/schema.sql` (`entity_type.footprint_w`, `footprint_h`, `radius`).

## Build

1. **What leaves a stain.** A building or unit on a planet surface that is
   destroyed: it lost its last hit points (combat, splash, orbital
   bombardment, attrition, an attacker consumed by its own attack). Nothing
   else that removes an entity leaves a stain: a building demolished by its
   owner (owner 2026-11-17: only destroyed buildings), boarding a transport, an upgrade that replaces a
   building, a unit going into a garrison, a ship leaving for space,
   ascension (task 76), and units that go down with their carrier in space.
   A building still under construction leaves a stain scaled by how far it
   was built. Ships destroyed in orbit or in space leave none. Decide this
   from the cause the simulation already knows at the call site (a reason
   passed to `despawn`, or a separate entry point), not from type keys
   (rule 2).
2. **Size from stature.** The stain is a round, slightly irregular mark
   centred where the thing stood.
   - Buildings: radius from the footprint (about 0.6 × the footprint
     diagonal), so a city centre leaves a large scar and a house a small one.
   - Units and landed ships: radius from the collision radius (about 2.5 ×
     `radius`), with a floor so a foot soldier still leaves a visible mark.
   - The factors live in one place in code, or as constants if you find
     they need tuning; say which.
   - Look: soot, ash and churned earth darkening the ground (darker in the
     centre, ragged edge, a little variation per stain from its id). A
     building's stain also shows debris: scattered rubble and broken
     material in its colours, drawn the same way as the stain (no mesh made
     per stain), fading with it. No blood or gore. On water there is no stain; a stain overlapping the
     shore stops at the water.
3. **Fading.** A stain lasts `world.stain_days` days (new constant, 6: the
   owner's choice of 2026-11-17, 30 game minutes at the default day length) and fades out
   smoothly over its life, mostly in the last third. Lifetime in ticks =
   `world.stain_days × world.day_ticks`. Overlapping stains darken the ground
   more, up to a limit (no black holes where many units died).
4. **Simulation state, deterministic.** Stains are part of the match state
   so that a saved and loaded match looks the same: each surface keeps a
   list of stains (position in milli-tiles, radius in milli-tiles, tick of
   destruction), appended in id order at destruction and pruned when
   expired. At most `world.stain_cap` stains per surface (new constant,
   2000); past it the oldest goes. Integers only. They take part in the
   state hash. No gameplay effect: nothing in the simulation or the computer
   players reads them.
5. **Save.** Stains are saved and restored; bump the save format by one and
   keep the loader's rule for older formats as it is today (state it).
6. **Drawing.** The snapshot carries the stains of the surface being drawn
   (or a change list, if the full list is too large to copy every frame:
   measure). The renderer draws them on the terrain without baking a mesh
   per stain during the match (rule 9): for example as decals from one mesh
   made before play, draped on the terrain, or inside the terrain shader.
   They follow the relief, sit under units, buildings, vegetation and
   shadows, and are lit by the day and night like the ground. Fog of war:
   a stain is drawn only where the viewing player currently has vision (it
   reveals that something died there).
7. **Constants.** `world.stain_days` (6, days) and `world.stain_cap` (2000)
   through `app/db/content/generate.py`, documented in
   `app/docs/database.md`; lint views empty.
8. **Tests.**
   - A building and a unit destroyed by damage each leave one stain of the
     expected radius; a unit boarding a transport, an upgraded building and
     a ship destroyed in orbit leave none.
   - A demolished building leaves none; a half-built building destroyed by
     damage leaves a smaller stain than a finished one.
   - A stain is gone after `stain_days × day_ticks` ticks; the cap drops
     the oldest.
   - Run a battle twice: equal hashes; save and load mid-battle: equal
     hashes and equal stain lists.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint, release build,
build-rules/check-rules); playthrough complete (the hash changes because
stains enter the state hash: say so and show it run twice equal); `rom
duel` subset unchanged in results (stains have no gameplay effect); frame
time of a big battle with many stains, before and after; screenshots you
looked at: a battlefield just after a fight (stains of a city centre,
houses with their debris, soldiers and a tank side by side), the same place half faded and
fully faded (scripted time skip), stains at night, a stain at a shore;
commit on the branch (stage by name), do not push or merge; final message
with commands and real output, files changed, schema and save-format
changes, deviations, anything not verified.
